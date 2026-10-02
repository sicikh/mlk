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

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use mlkc_codegen_wasm::{
    CodegenDiag, DebugLevel, FnShape, FunctionCtx, ModuleMir, WasmModule, assemble_module,
    emit_function, layout,
};
use mlkc_driver::{Driver, LinkPlan};
use mlkc_hir_def::{BodyLoc, EntityLoc, FunctionLoc, ModuleId, Name, ProjectData, ProjectId};
use mlkc_interp::{Extern, Program};
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
    pub fn wat(&self) -> String {
        wasmprinter::print_bytes(&self.wasm.bytes).expect("the emitted module to be printable")
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
    let (wasm, diagnostics) = compile(&mut driver, &module);

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
    let plan = driver.link(&project).expect("the project to link");
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

            let body = driver
                .mir(&function.owner)
                .expect("a body the front end read clean to have a CFG form");

            cfg.insert(body);
        }

        if let Some((entry_module, entry_name)) = &plan.entry
            && entry_module == id
        {
            let function = module
                .functions
                .iter()
                .find(|function| &function.name == entry_name)
                .expect("the entry to be a function of its module");
            let BodyLoc::Function(loc) = &function.owner.item else {
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
fn compile(driver: &mut Driver, module: &ModuleMir) -> (WasmModule, Vec<CodegenDiag>) {
    let layout = layout(module);
    let mut artifacts = Vec::new();
    let mut diagnostics = Vec::new();

    for function in &module.functions {
        let lir = driver
            .lir(&function.owner)
            .expect("a function of the module to be lowered");
        let ctx = FunctionCtx {
            name: &function.name,
            signature: &function.signature,
            param_names: &function.param_names,
            layout: &layout,
        };
        let (artifact, reports) = emit_function(&lir, &ctx);

        artifacts.push(artifact);
        diagnostics.extend(reports);
    }

    let (wasm, reports) = assemble_module(module, &artifacts, DebugLevel::Full);

    diagnostics.extend(reports);

    (wasm, diagnostics)
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
