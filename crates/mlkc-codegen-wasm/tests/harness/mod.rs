//! The harness of the back-end tests: a program compiled all the way to a WASM module.
//!
//! A test writes a program the way [`mlkc_fixture`] writes a project, and the harness runs the
//! front end, the construction of MIR, and both stages of the back end over it ([ADR-0020]). What
//! a test reads of the result is the module the assembler wrote, the artifact of every function,
//! and what the back end reported ([`CodegenDiag`]).
//!
//! The harness compiles one module at a time: a translation unit is a module ([ADR-0021]), and
//! the linker that joins them is not built yet.
//!
//! [adr-0020]: ../../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../../docs/adr/0021-translation-units.md

use std::{
    fs,
    path::{Path, PathBuf},
};

use mlkc_codegen_wasm::{
    CodegenDiag, DebugLevel, FnSignature, FuncArtifact, FunctionCtx, ModuleFunction, ModuleMir,
    WasmModule, assemble_module, emit_function, layout,
};
use mlkc_driver::Driver;
use mlkc_hir_def::{
    BodyLoc, EntityData, EntityLoc, ItemLocLike, ModuleId, Name, Pat, ProjectData, ProjectId,
    Visibility,
};
use mlkc_hir_ty::Ty;
use mlkc_vfs::VfsPath;

/// The directory the fixtures of the back-end tests live in, relative to the tests of the crate.
pub const SPECS_DIR: &str = "specs";

/// The directory of the tests of this crate.
const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

/// A program compiled to a WASM module, with everything a test reads of it.
pub struct Compiled {
    /// The module the back end was handed.
    pub module: ModuleMir,
    /// The artifact of every function, in the order the module declares them.
    pub artifacts: Vec<FuncArtifact>,
    /// What the back end reported about the program.
    pub diagnostics: Vec<CodegenDiag>,
    /// The WASM module the artifacts were assembled into.
    pub wasm: WasmModule,
}

impl Compiled {
    /// The module as text, for a person to read and for a snapshot.
    pub fn wat(&self) -> String {
        wasmprinter::print_bytes(&self.wasm.bytes).expect("the emitted module to be printable")
    }

    /// The function the module declares under `name`.
    pub fn function(&self, name: &str) -> &ModuleFunction {
        self.module
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("the module to declare a function `{name}`"))
    }

    /// Checks the module against the WebAssembly specification, with the proposals it uses.
    ///
    /// A body that carries a codegen diagnostic is not a value the driver would ever assemble,
    /// so a test of a diagnostic does not validate it.
    pub fn validate(&self) {
        use wasmparser::{Validator, WasmFeatures};

        let features = WasmFeatures::GC
            | WasmFeatures::GC_TYPES
            | WasmFeatures::FUNCTION_REFERENCES
            | WasmFeatures::REFERENCE_TYPES;

        if let Err(error) = Validator::new_with_features(features).validate_all(&self.wasm.bytes) {
            panic!(
                "the emitted module does not validate: {error}\n\n{}",
                self.wat(),
            );
        }
    }
}

/// Compiles the project a fixture writes, one module per mark ([`mlkc_fixture`]).
///
/// # Panics
///
/// Panics when the fixture is not a project the front end reads clean, when it holds more than
/// one module, and when a pass of the driver bugged: a test writes programs the compiler is
/// expected to compile, and anything else is the test's mistake or the compiler's bug.
pub fn project(fixture: &str) -> Compiled {
    let mut driver = Driver::new();
    driver.use_std();

    let project = ProjectId::new("app");
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

    let index = driver
        .module_index(&project)
        .expect("the project to be held by the driver");

    let mut modules = index.iter();
    let (path, id) = modules.next().expect("the fixture to hold a module");

    assert!(
        modules.next().is_none(),
        "the back end compiles one module at a time, and the fixture holds more",
    );

    let name = path.iter().map(Name::as_str).collect::<Vec<_>>().join("::");
    let builtins = driver
        .builtins()
        .expect("the standard library to declare the classes of the language");
    let lowered = driver.lower(id.0).expect("the module to be lowered");
    let types = driver
        .module_types(id)
        .expect("the signatures of the module to resolve");

    let mut functions = Vec::new();

    for body in lowered.bodies() {
        let owner = body.owner();
        let BodyLoc::Function(_) = &owner.item else {
            // A constant's body is a body, but it has no function ABI and nothing calls it yet.
            continue;
        };

        let entity = EntityLoc::from(owner.clone());
        let Some(Ty::Fn { params, ret }) = types.get(&entity) else {
            panic!(
                "the body of `{:?}` to have a function signature",
                owner.item
            );
        };

        let hir = &body.body().body;
        let param_names = hir
            .params()
            .iter()
            .map(|pat| {
                match &hir[*pat] {
                    Pat::Bind(name) => Some(name.clone()),
                    Pat::Missing | Pat::Wildcard => None,
                }
            })
            .collect();

        let Some(EntityData::Function(data)) = lowered.item_tree().entity_data(entity.item.clone())
        else {
            panic!("a body to be owned by a function");
        };

        let name = owner
            .item
            .name()
            .expect("a function of a body to have a name")
            .to_string();
        let body = driver
            .mir_ssa(owner)
            .expect("a body the front end read clean to have an SSA form");

        functions.push(ModuleFunction {
            owner: owner.clone(),
            name,
            signature: FnSignature {
                params: params.clone(),
                ret: ret.as_ref().clone(),
            },
            param_names,
            exported: matches!(data.visibility, Visibility::Public),
            body,
        });
    }

    // A compiler that bugged has no business answering a value: the report says what was being
    // computed when a pass panicked, and failing here is how a review sees it.
    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    let module = ModuleMir {
        name,
        builtins,
        functions,
    };
    let layout = layout(&module);
    let mut artifacts = Vec::new();
    let mut diagnostics = Vec::new();

    for function in &module.functions {
        let ctx = FunctionCtx {
            name: &function.name,
            signature: &function.signature,
            param_names: &function.param_names,
            layout: &layout,
        };
        let (artifact, reports) = emit_function(&function.body, &ctx);

        artifacts.push(artifact);
        diagnostics.extend(reports);
    }

    let (wasm, reports) = assemble_module(&module, &artifacts, DebugLevel::Full);

    diagnostics.extend(reports);

    Compiled {
        module,
        artifacts,
        diagnostics,
        wasm,
    }
}

/// Compiles one module, written as its source.
pub fn module(source: &str) -> Compiled {
    project(&format!("//- /main.mlk\n{source}"))
}

/// Compiles the fixture under [`SPECS_DIR`], named by `name`.
pub fn fixture(name: &str) -> Compiled {
    let path = fixture_path(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    project(&source)
}

/// The directory the fixtures of the back-end tests live in.
pub fn specs_dir() -> PathBuf {
    Path::new(TESTS_DIR).join(SPECS_DIR)
}

/// The path of the fixture `name`, given as a name relative to [`SPECS_DIR`].
pub fn fixture_path(name: &str) -> PathBuf {
    specs_dir().join(format!("{name}.mlk"))
}
