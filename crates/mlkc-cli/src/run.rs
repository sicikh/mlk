//! Running a build: the modules and the manifest of a project ([ADR-0021]).
//!
//! A build is not one module but a program: every module of the language is a WASM module of
//! its own, linked by function imports under the canonical name of the provider, and an extern
//! of the language is a function a host implements. The manifest says what the build is --- the
//! modules in the order they are instantiated in, what each imports and exports, and where the
//! program begins --- and this host does what it says: it defines the externs it implements,
//! instantiates the modules in the order of the manifest, and calls the entry point.
//!
//! What a build may carry beside the manifest is `host.wasm`, the shim for a host that cannot
//! make GC values --- a JavaScript one. This host is not one of those: it makes an `i31`
//! immediate itself, so it implements the externs directly and never instantiates the shim.
//!
//! [adr-0021]: ../docs/adr/0021-translation-units.md

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail};
use mlkc_driver::Manifest;
use wasmtime::{
    AnyRef, Caller, Config, Engine, ExternType, I31, Instance, Linker, Module, OptLevel, Store, Val,
};

use crate::archive::Archive;

/// The functions of the language a host implements, by the name a program imports them under.
///
/// The names are the ones the library declares `#[extern]` ([ADR-0015]); a build that imports
/// any other one is one this host cannot run, and the error says so.
///
/// [adr-0015]: ../docs/adr/0015-standard-library.md
const HOST_FUNCTIONS: &[&str] = &["print-int", "print-bool"];

/// What a program printed, as the host functions record it.
#[derive(Default)]
struct Output {
    printed: Vec<String>,
}

/// Where a build is read from: the directory it was written into, or the archive it came in.
enum Build {
    /// The files of a build, under the directory they were written into.
    Directory(PathBuf),

    /// The same files, held in the ZIP archive the editor hands over.
    Archived(Archive),
}

impl Build {
    /// The build at `path`: a directory of files, or an archive of them.
    fn read(path: &Path) -> anyhow::Result<Build> {
        let found =
            fs::metadata(path).with_context(|| format!("no build at {}", path.display()))?;

        if found.is_dir() {
            return Ok(Build::Directory(path.to_path_buf()));
        }

        let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;

        Ok(Build::Archived(Archive::read(bytes).with_context(
            || format!("{} is not an archive of a build", path.display()),
        )?))
    }

    /// The bytes of the file `name` of the build.
    fn file(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        match self {
            Build::Directory(directory) => {
                let path = directory.join(name);

                fs::read(&path).with_context(|| format!("failed to read {}", path.display()))
            },
            Build::Archived(archive) => Ok(archive.file(name)?.to_vec()),
        }
    }
}

/// Runs the build at `path`, and hands back what the program printed.
///
/// A build is a directory of modules and a manifest, and `path` is either that directory or
/// the ZIP archive the editor hands the same files over in: the files are read from wherever
/// they are, and nothing is unpacked (a build is not written out to run). A build that
/// declares no entry point is not one to run, and says so.
///
/// `debug` is whether a native debugger follows the run ([ADR-0025]): the engine then
/// translates the DWARF of a module into debug information for the code it compiles --- the
/// GDB/LLDB JIT interface --- and does not optimize, so that a breakpoint stands where a line
/// is. A module that carries no DWARF is one a debugger has nothing to say about, however the
/// engine is configured.
///
/// [adr-0025]: ../docs/adr/0025-debug-information-formats.md
pub fn run(path: &Path, debug: bool) -> anyhow::Result<Vec<String>> {
    let build = Build::read(path)?;
    let manifest: Manifest = serde_json::from_slice(&build.file(Manifest::FILE)?)
        .with_context(|| format!("`{}` is not a manifest of a build", Manifest::FILE))?;

    let mut config = Config::new();

    config.wasm_gc(true);

    if debug {
        config.debug_info(true);
        config.cranelift_opt_level(OptLevel::None);
    }

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, Output::default());
    let mut linker: Linker<Output> = Linker::new(&engine);
    let mut compiled = BTreeMap::new();

    for held in &manifest.modules {
        let bytes = build.file(&held.file)?;

        compiled.insert(held.name.clone(), Module::new(&engine, bytes)?);
    }

    // What the build imports and no module provides is an extern of the language, and a host
    // implements it: one definition per name, however many modules import it ([ADR-0021]).
    let externs: BTreeSet<(&str, &str)> = manifest
        .modules
        .iter()
        .flat_map(|held| held.imports.iter())
        .filter(|import| import.external)
        .map(|import| (import.module.as_str(), import.name.as_str()))
        .collect();

    for (module, name) in externs {
        // The type is the one the importing module declares: every extern of the language
        // takes an immediate and gives one back ([ADR-0018]).
        let ty = compiled
            .values()
            .flat_map(Module::imports)
            .find_map(|import| {
                match import.ty() {
                    ExternType::Func(ty) if import.module() == module && import.name() == name => {
                        Some(ty)
                    },
                    _ => None,
                }
            })
            .with_context(|| {
                format!("no module imports `{module}::{name}`, which the manifest names")
            })?;

        if !HOST_FUNCTIONS.contains(&name) {
            bail!("the host does not implement the extern `{module}::{name}`");
        }

        let recorded = name.to_owned();

        linker.func_new(
            module,
            name,
            ty,
            move |mut caller: Caller<'_, Output>, params, results| {
                let immediate = match &params[0] {
                    Val::AnyRef(Some(value)) => {
                        value
                            .as_i31(&caller)?
                            .map(|it| it.get_i32())
                            .unwrap_or_default()
                    },
                    _ => 0,
                };

                caller.data_mut().printed.push(if recorded == "print-bool" {
                    (immediate != 0).to_string()
                } else {
                    immediate.to_string()
                });

                results[0] = Val::AnyRef(Some(AnyRef::from_i31(
                    &mut caller,
                    I31::new_i32(0).expect("zero to be an immediate"),
                )));

                Ok(())
            },
        )?;
    }

    // The providers of a module stand before it in the manifest, and an import is resolved when
    // the module that needs it is instantiated ([ADR-0021]).
    let mut instances: BTreeMap<&str, Instance> = BTreeMap::new();

    for held in &manifest.modules {
        let module = compiled
            .get(&held.name)
            .expect("every module of the manifest to be compiled");
        let instance = linker
            .instantiate(&mut store, module)
            .map_err(anyhow::Error::from)
            .with_context(|| format!("failed to instantiate `{}`", held.name))?;

        // What a module exports is what the modules after it import: the linker holds it under
        // the canonical name of the module that offers it ([ADR-0021]).
        for export in &held.exports {
            let function = instance
                .get_func(&mut store, &export.name)
                .with_context(|| format!("`{}` exports no `{}`", held.name, export.name))?;

            linker.define(&store, &held.name, &export.name, function)?;
        }

        instances.insert(&held.name, instance);
    }

    let entry = manifest
        .entry
        .as_ref()
        .context("the build declares no `#[entry]`, so there is nothing to run")?;
    let instance = instances
        .get(entry.module.as_str())
        .with_context(|| format!("the build holds no module `{}`", entry.module))?;
    let function = instance
        .get_func(&mut store, &entry.name)
        .with_context(|| format!("`{}` exports no `{}`", entry.module, entry.name))?;
    let mut results = [Val::AnyRef(None)];

    function.call(&mut store, &[], &mut results)?;

    Ok(store.into_data().printed)
}
