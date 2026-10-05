//! The harness of the back-end tests: a program compiled all the way to a WASM module.
//!
//! A test writes a program the way [`mlkc_fixture`] writes a project, and the harness runs the
//! front end, the construction of MIR, and both stages of the back end over it ([ADR-0020]).
//! What a test reads of the result is the module the assembler wrote and what the back end
//! reported ([`CodegenDiag`]).
//!
//! [`project`] compiles one module, and [`run_project`] asks the driver for the [`LinkPlan`] of
//! the project: the compiled modules in the order they link in, the entry point, and the same
//! bodies as programs the interpreter runs ([ADR-0021]). A test that runs a project compares
//! what the two compute (`mlkc-interp` is the other side of the comparison).
//!
//! [adr-0020]: ../../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../../docs/adr/0021-translation-units.md

// One harness is shared by the test binaries of the suite, and no single one of them uses all
// of what it offers: what a binary does not call is not dead code.
#![allow(
    dead_code,
    reason = "the harness is shared by the test binaries of the suite, and no one of them uses all of it"
)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use mlkc_codegen_wasm::{
    CodegenDiag, DebugInfo, FnShape, ModuleMir, Sources, WasmModule, compile_module,
};
use mlkc_driver::{Driver, LinkPlan};
use mlkc_hir_def::{BodyLoc, EntityLoc, FunctionLoc, ModuleId, Name, ProjectData, ProjectId};
use mlkc_interp::{Extern, Program};
use mlkc_mir::FunctionLoc as MirFunctionLoc;
use mlkc_vfs::VfsPath;

/// The name the project of a fixture is compiled under.
const PROJECT: &str = "app";

/// The directory the fixtures of the back-end tests live in, relative to the tests of the crate.
pub const SPECS_DIR: &str = "specs";

/// The directory the runs of a project, relative to the tests of the crate.
pub const RUNS_DIR: &str = "runs";

/// The directory of the tests of this crate.
const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

/// A program compiled to a WASM module, with everything a test reads of it.
pub struct Compiled {
    /// The module the back end was handed.
    pub module: Arc<ModuleMir>,
    /// What the back end reported about the program.
    pub diagnostics: Vec<CodegenDiag>,
    /// The WASM module the artifacts were assembled into.
    pub wasm: WasmModule,
}

impl Compiled {
    /// The module as text, for a person to read and for a snapshot.
    ///
    /// The binary custom sections --- the debug tables --- are left out: they are a blob in a
    /// text file, and what the code reads is what a person reads here.
    pub fn wat(&self) -> String {
        let text =
            wasmprinter::print_bytes(&self.wasm.bytes).expect("the emitted module to be printable");
        let mut kept = String::with_capacity(text.len());

        for line in text.lines() {
            if line.trim_start().starts_with("(@custom \"") {
                continue;
            }

            kept.push_str(line);
            kept.push('\n');
        }

        kept
    }

    /// Checks the module against the WebAssembly specification, with the proposals it uses.
    ///
    /// A body that carries a codegen diagnostic is not a value the driver would ever assemble,
    /// so a test of a diagnostic does not validate it.
    pub fn validate(&self) {
        use wasmparser::{Validator, WasmFeatures};

        // A `ref.func` names a function the module declared, and a declaration is a declarative
        // element segment ([ADR-0026][adr-0026]).
        //
        // [adr-0026]: ../../../docs/adr/0026-closure-representation.md
        let features = WasmFeatures::GC
            | WasmFeatures::GC_TYPES
            | WasmFeatures::FUNCTION_REFERENCES
            | WasmFeatures::REFERENCE_TYPES
            | WasmFeatures::BULK_MEMORY;

        if let Err(error) = Validator::new_with_features(features).validate_all(&self.wasm.bytes) {
            panic!(
                "the emitted module does not validate: {error}\n\n{}",
                self.wat(),
            );
        }
    }
}

/// A project linked the way a run of it needs.
pub struct RunProject {
    /// The plan the driver built: the compiled modules, their order, and what it reported.
    pub plan: Arc<LinkPlan>,
    /// The modules of the plan, in the order they link in.
    pub modules: Vec<Compiled>,
    /// The canonical name of every module, by its place in `modules`.
    pub names: Vec<String>,
    /// The entry point: the place of the module in `modules`, the name of the function, and the
    /// entity the interpreter calls.
    pub entry: Option<(usize, String, EntityLoc<FunctionLoc>)>,
    /// The bodies of the program in the CFG form, with the externs a host provides.
    pub cfg: Program,
    /// The bodies of the program in the SSA form, with the externs a host provides.
    pub ssa: Program,
    /// The externs the modules import: the canonical module and name of each, and the shape it
    /// crosses the boundary as. A host implements exactly these.
    pub externs: BTreeMap<(String, String), FnShape>,
}

/// Compiles the project a fixture writes, one module per mark ([`mlkc_fixture`]).
///
/// # Panics
///
/// Panics when the fixture is not a project the front end reads clean, when it holds more than
/// one module, and when a pass of the driver bugged: a test writes programs the compiler is
/// expected to compile, and anything else is the test's mistake or the compiler's bug.
pub fn project(fixture: &str) -> Compiled {
    project_with(fixture, DebugInfo::DwarfFull)
}

/// Compiles the project a fixture writes at the debug level `debug`.
///
/// The format is what the driver would ask for under its options ([ADR-0025]); the tests of
/// the debug information read what each option carries.
///
/// [adr-0025]: ../../../docs/adr/0025-debug-information-formats.md
pub fn project_with(fixture: &str, debug: DebugInfo) -> Compiled {
    let (mut driver, project) = setup(fixture);
    let index = driver
        .module_index(&project)
        .expect("the project to be held by the driver");
    let mut modules = index.iter();
    let (_, id) = modules.next().expect("the fixture to hold a module");

    assert!(
        modules.next().is_none(),
        "the back end compiles one module at a time, and the fixture holds more",
    );

    let module = driver
        .mir_module(id)
        .expect("the module to be read by the front end");
    let (wasm, diagnostics) = compile(&mut driver, id, &module, debug);

    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    Compiled {
        module,
        diagnostics,
        wasm,
    }
}

/// Links the project a fixture writes, and reads it the way a run does.
///
/// The modules come from the driver's plan, in the order it found, and the bodies of every one
/// of them are put into the programs the interpreter runs.
///
/// # Panics
///
/// Panics for the reasons [`project`] does, and when the project does not link.
pub fn run_project(fixture: &str) -> RunProject {
    let (mut driver, project) = setup(fixture);
    let plan = driver
        .link(&project)
        .unwrap_or_else(|| panic!("the project to link: {}", reported(&mut driver, &project)));
    let mut modules = Vec::new();
    let mut names = Vec::new();
    let mut entry = None;
    let mut cfg = Program::new();
    let mut ssa = Program::new();
    let mut externs = BTreeMap::new();

    for (index, id) in plan.order.iter().enumerate() {
        let module = driver
            .mir_module(*id)
            .expect("a module of the plan to be read by the front end");
        let name = plan
            .names
            .get(id)
            .expect("a module of the plan to have a name")
            .clone();

        for import in &module.imports {
            let external = Extern {
                module: import.module.clone(),
                name: import.name.clone(),
            };

            cfg.declare_extern(import.entity.clone(), external.clone());
            ssa.declare_extern(import.entity.clone(), external);

            // What a host implements is the import of an extern: an import of another module is
            // resolved by the linker, from the module it names.
            if import.external {
                externs.insert(
                    (import.module.clone(), import.name.clone()),
                    import.signature.shape(&module.builtins),
                );
            }
        }

        for function in &module.functions {
            ssa.insert(Arc::clone(&function.body));
        }

        // The CFG form of every function: a HIR body holds its own and the functions lifted out
        // of it, flat, and the module lists them all ([`Driver::mir`]).
        let mut origins = BTreeSet::new();

        for function in &module.functions {
            let origin = function.function.origin().clone();

            if !origins.insert(origin.clone()) {
                continue;
            }

            let bodies = driver
                .mir(&origin)
                .expect("a body the front end read clean to have a CFG form");

            for body in bodies.iter() {
                cfg.insert(Arc::clone(body));
            }
        }

        if let Some((entry_module, entry_name)) = &plan.entry
            && entry_module == id
        {
            let function = module
                .functions
                .iter()
                .find(|function| &function.name == entry_name)
                .expect("the entry to be a function of its module");
            let MirFunctionLoc::Entity(owner) = &function.function else {
                panic!("the entry to be a function of the project");
            };
            let BodyLoc::Function(loc) = &owner.item else {
                panic!("the entry to be a function");
            };

            entry = Some((index, entry_name.clone(), EntityLoc {
                module: *id,
                item: loc.clone(),
            }));
        }

        modules.push(Compiled {
            module,
            diagnostics: Vec::new(),
            wasm: (*plan.modules[id]).clone(),
        });
        names.push(name);
    }

    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    RunProject {
        plan,
        modules,
        names,
        entry,
        cfg,
        ssa,
        externs,
    }
}

/// What the driver reported about every module of a project, for a link that did not happen.
fn reported(driver: &mut Driver, project: &ProjectId) -> String {
    let Some(index) = driver.module_index(project) else {
        return "the project is not one the driver holds".to_owned();
    };

    let mut report = String::new();

    for (path, module) in index.iter() {
        let path = path.iter().map(Name::as_str).collect::<Vec<_>>().join("::");
        let diagnostics = driver.diagnostics(module.0);

        report.push_str(&format!("\n{path}: {diagnostics:?}"));
    }

    if let Some(ice) = driver.ice() {
        report.push_str(&format!("\n\n{ice}"));
    }

    report
}

/// The driver a fixture is compiled by: the standard library, the project that depends on it,
/// and the modules of the fixture.
fn setup(fixture: &str) -> (Driver, ProjectId) {
    let mut driver = Driver::new();
    driver.use_std();

    let project = ProjectId::new(PROJECT);
    let mut data = ProjectData::default();
    data.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );
    driver.set_project(project.clone(), data);

    for module in mlkc_fixture::modules(fixture) {
        let path = VfsPath::new_virtual_path(module.place.clone());
        driver.set_file_text(path.clone(), Some(module.source.clone()));

        let file = driver
            .file_id(&path)
            .expect("a module of the fixture to be pushed");
        driver.set_module_project(ModuleId(file), project.clone());
    }

    (driver, project)
}

/// Compiles the functions of a module, and assembles the module they become.
///
/// Every body is lowered by the driver --- the `lir` stage of [ADR-0022] --- and encoded from
/// what the stage handed over, which is the path the linker takes.
///
/// [adr-0022]: ../../../docs/adr/0022-wasm-lir.md
fn compile(
    driver: &mut Driver,
    module_id: ModuleId,
    module: &ModuleMir,
    debug: DebugInfo,
) -> (WasmModule, Vec<CodegenDiag>) {
    let lirs = driver
        .module_lir(module_id)
        .expect("the functions of the module to be lowered");

    compile_module(module, &lirs, debug, &sources(driver, module))
}

/// What the debug tables of a module read: the path, the text, and the lines of every file its
/// bodies were read from.
fn sources(driver: &mut Driver, module: &ModuleMir) -> Sources {
    let primary = module.functions.first().map_or_else(
        || module.name.clone(),
        |function| driver.file_path(function.function.module().0).to_string(),
    );
    let mut sources = Sources::new(primary);

    for function in &module.functions {
        let file = function.function.module().0;

        let Some(lines) = driver.line_index(file) else {
            continue;
        };
        let Some(text) = driver.file_text(file) else {
            continue;
        };

        sources.insert(file, driver.file_path(file).to_string(), text, lines);
    }

    sources
}

/// Compiles one module, written as its source.
pub fn module(source: &str) -> Compiled {
    module_with(source, DebugInfo::DwarfFull)
}

/// Compiles one module, written as its source, at the debug level `debug`.
pub fn module_with(source: &str, debug: DebugInfo) -> Compiled {
    project_with(&format!("//- /main.mlk\n{source}"), debug)
}

/// Compiles the fixture under [`SPECS_DIR`], named by `name`.
pub fn fixture(name: &str) -> Compiled {
    let path = fixture_path(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    project(&source)
}

/// Compiles the run under [`RUNS_DIR`], named by `name`.
pub fn run(name: &str) -> RunProject {
    let path = run_path(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    run_project(&source)
}

/// The directory the fixtures of the back-end tests live in.
pub fn specs_dir() -> PathBuf {
    Path::new(TESTS_DIR).join(SPECS_DIR)
}

/// The path of the fixture `name`, given as a name relative to [`SPECS_DIR`].
pub fn fixture_path(name: &str) -> PathBuf {
    specs_dir().join(format!("{name}.mlk"))
}

/// The directory the modules of a run live in.
pub fn runs_dir() -> PathBuf {
    Path::new(TESTS_DIR).join(RUNS_DIR)
}

/// The path of the run `name`, given as a name relative to [`RUNS_DIR`].
pub fn run_path(name: &str) -> PathBuf {
    runs_dir().join(format!("{name}.mlk"))
}
