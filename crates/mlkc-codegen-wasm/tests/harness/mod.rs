//! The harness of the back-end tests: a program compiled all the way to a WASM module.
//!
//! A test writes a program the way [`mlkc_fixture`] writes a project, and the harness runs the
//! front end, the construction of MIR, and both stages of the back end over it ([ADR-0020]). What
//! a test reads of the result is the module the assembler wrote, the artifact of every function,
//! and what the back end reported ([`CodegenDiag`]).
//!
//! [`project`] compiles one module, and [`run_project`] compiles every module of a project and
//! reads them the way a run does: the modules in an order where a provider comes before the
//! modules that import it ([ADR-0021]), the entry point, and the same bodies as programs the
//! interpreter runs. A test that runs a project compares what the two compute
//! ([`crate`](../../mlkc_interp) is the other side of the comparison).
//!
//! [adr-0020]: ../../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../../docs/adr/0021-translation-units.md

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use mlkc_codegen_wasm::{
    CodegenDiag, DebugLevel, FnSignature, FuncArtifact, FunctionCtx, ModuleFunction, ModuleImport,
    ModuleMir, WasmModule, assemble_module, emit_function, layout,
};
use mlkc_driver::Driver;
use mlkc_hir_def::{
    BodyLoc, EntityData, EntityLoc, FunctionLoc, ItemLocLike, ModuleId, Name, Pat, ProjectData,
    ProjectId, Visibility,
};
use mlkc_hir_ty::{Builtins, Ty};
use mlkc_interp::{Extern, Program};
use mlkc_mir::{Callee, Rvalue, StmtKind};
use mlkc_vfs::VfsPath;

/// The name the project of a fixture is compiled under.
const PROJECT: &str = "app";

/// The directory the fixtures of the back-end tests live in, relative to the tests of the crate.
pub const SPECS_DIR: &str = "specs";

/// The directory of the runs of a project, relative to the tests of the crate.
pub const RUNS_DIR: &str = "runs";

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

/// A project compiled to WASM modules and read by the interpreter, as a run of it needs both.
pub struct RunProject {
    /// The modules of the project, in an order where a provider comes before the modules that
    /// import it.
    pub modules: Vec<Compiled>,
    /// The canonical name of every module, by its place in `modules`.
    pub names: Vec<String>,
    /// The entry point: the place of the module in `modules`, the name of the function, and the
    /// entity the interpreter calls.
    pub entry: Option<(usize, String, EntityLoc<FunctionLoc>)>,
    /// The bodies of the project in the CFG form, with the externs a host provides.
    pub cfg: Program,
    /// The bodies of the project in the SSA form, with the externs a host provides.
    pub ssa: Program,
    /// The externs the modules import: the canonical module and name of each, and how many words
    /// it takes. A host implements exactly these.
    pub externs: BTreeMap<(String, String), u32>,
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
    let names = names(&mut driver);
    let builtins = driver
        .builtins()
        .expect("the standard library to declare the classes of the language");

    let index = driver
        .module_index(&project)
        .expect("the project to be held by the driver");
    let mut modules = index.iter();
    let (_, id) = modules.next().expect("the fixture to hold a module");

    assert!(
        modules.next().is_none(),
        "the back end compiles one module at a time, and the fixture holds more",
    );

    let canonical = names
        .get(&id)
        .expect("a module of the project to have a canonical name")
        .clone();
    let compiled = compile_module(&mut driver, id, &canonical, &names, builtins);

    // A compiler that bugged has no business answering a value: the report says what was being
    // computed when a pass panicked, and failing here is how a review sees it.
    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    compiled
}

/// Compiles every module of the project a fixture writes, and reads them as a run does.
///
/// The modules come in an order where a provider comes before the modules that import it: an
/// import of an extern is a host function and not a dependency, and every other import names a
/// module of the project ([ADR-0021]).
///
/// # Panics
///
/// Panics for the reasons [`project`] does, and when an import names a module the harness does
/// not compile: a body of another project, the standard library's included, is not linked yet.
pub fn run_project(fixture: &str) -> RunProject {
    let (mut driver, project) = setup(fixture);
    let names = names(&mut driver);
    let externals = external_entities(&mut driver);
    let builtins = driver
        .builtins()
        .expect("the standard library to declare the classes of the language");

    let index = driver
        .module_index(&project)
        .expect("the project to be held by the driver");
    let mut compiled = Vec::new();

    for (_, module) in index.iter() {
        let canonical = names
            .get(&module)
            .expect("a module of the project to have a canonical name")
            .clone();

        compiled.push((
            canonical,
            compile_module(
                &mut driver,
                module,
                &names[&module],
                &names,
                builtins.clone(),
            ),
        ));
    }

    let order = module_order(&compiled, &externals);
    let mut slots: Vec<Option<(String, Compiled)>> = compiled.into_iter().map(Some).collect();
    let mut modules = Vec::with_capacity(slots.len());
    let mut module_names = Vec::with_capacity(slots.len());
    let mut cfg = Program::new();
    let mut ssa = Program::new();
    let mut externs = BTreeMap::new();

    for index in order {
        let (canonical, module) = slots[index].take().expect("a module to be placed once");

        for import in &module.module.imports {
            let external = Extern {
                module: import.module.clone(),
                name: import.name.clone(),
            };

            cfg.declare_extern(import.entity.clone(), external.clone());
            ssa.declare_extern(import.entity.clone(), external);

            // What a host implements is the import of an extern: an import of another module is
            // resolved by the linker, from the module it names.
            if externals.contains(&import.entity) {
                externs.insert(
                    (import.module.clone(), import.name.clone()),
                    import.signature.params.len() as u32,
                );
            }
        }

        for function in &module.module.functions {
            ssa.insert(Arc::clone(&function.body));

            let body = driver
                .mir(&function.owner)
                .expect("a body the front end read clean to have a CFG form");

            cfg.insert(body);
        }

        modules.push(module);
        module_names.push(canonical);
    }

    let entry = entry_of(&modules);

    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    RunProject {
        modules,
        names: module_names,
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

/// The canonical name of every module of the projects a fixture is compiled with.
///
/// An import names the module its function belongs to ([ADR-0021]).
fn names(driver: &mut Driver) -> BTreeMap<ModuleId, String> {
    let mut names = BTreeMap::new();

    for (project, project_name) in [
        (ProjectId::new(PROJECT), PROJECT.to_owned()),
        (
            ProjectId::new(mlkc_stdlib::PROJECT),
            mlkc_stdlib::PROJECT.to_owned(),
        ),
    ] {
        let Some(index) = driver.module_index(&project) else {
            continue;
        };

        for (path, module) in index.iter() {
            let path = path.iter().map(Name::as_str).collect::<Vec<_>>().join("::");

            names.insert(module, format!("{project_name}::{path}"));
        }
    }

    names
}

/// The entities of the fixture and of the standard library declared `#[extern]`.
///
/// A call to one is an import a host provides, and never a dependency on another module of the
/// plan; the linker tells the two kinds of import apart by this set.
fn external_entities(driver: &mut Driver) -> BTreeSet<EntityLoc<FunctionLoc>> {
    let mut externals = BTreeSet::new();

    for project in [
        ProjectId::new(PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    ] {
        let Some(index) = driver.module_index(&project) else {
            continue;
        };

        for (_, module) in index.iter() {
            let lowered = driver
                .lower(module.0)
                .expect("a module of a project to be lowered");

            for (item, _) in lowered.item_tree().entities() {
                let Some(EntityData::Function(data)) =
                    lowered.item_tree().entity_data(item.clone())
                else {
                    continue;
                };

                if !data.attributes.external {
                    continue;
                }

                let Ok(loc) = FunctionLoc::try_from(item) else {
                    continue;
                };

                externals.insert(EntityLoc { module, item: loc });
            }
        }
    }

    externals
}

/// Compiles one module of a project: its bodies to artifacts, and the artifacts to a module.
fn compile_module(
    driver: &mut Driver,
    module: ModuleId,
    canonical: &str,
    names: &BTreeMap<ModuleId, String>,
    builtins: Builtins,
) -> Compiled {
    let lowered = driver
        .lower(module.0)
        .expect("a module of a project to be lowered");
    let types = driver
        .module_types(module)
        .expect("the signatures of a module to resolve");
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

    // A callee the module declares is called by its index; every other callee is an import: a
    // function of another module, or one declared `#[extern]` ([ADR-0021]).
    let local: BTreeSet<EntityLoc<FunctionLoc>> = functions
        .iter()
        .filter_map(|function| {
            match &function.owner.item {
                BodyLoc::Function(loc) => {
                    Some(EntityLoc {
                        module: function.owner.module,
                        item: loc.clone(),
                    })
                },
                BodyLoc::Const(_) => None,
            }
        })
        .collect();
    let mut imports = Vec::new();

    for entity in callees(&functions) {
        if local.contains(&entity) {
            continue;
        }

        let Some(module) = names.get(&entity.module) else {
            panic!("the module of a callee to be named: {entity:?}");
        };
        let Some(name) = entity.item.name().map(ToString::to_string) else {
            continue;
        };
        let types = driver
            .module_types(entity.module)
            .expect("the module of a callee to have its types resolved");
        let Some(Ty::Fn { params, ret }) = types.get(&EntityLoc::from(entity.clone())) else {
            panic!("a callee to have a function signature: {entity:?}");
        };

        imports.push(ModuleImport {
            entity,
            module: module.clone(),
            name,
            signature: FnSignature {
                params: params.clone(),
                ret: ret.as_ref().clone(),
            },
        });
    }

    let module = ModuleMir {
        name: canonical.to_owned(),
        builtins,
        imports,
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

/// The entities a call of a body names, in the order of their entities.
fn callees(functions: &[ModuleFunction]) -> BTreeSet<EntityLoc<FunctionLoc>> {
    let mut callees = BTreeSet::new();

    for function in functions {
        for (_, block) in function.body.blocks.iter() {
            for stmt in &block.stmts {
                let StmtKind::Assign { rvalue, .. } = &stmt.kind;

                if let Rvalue::Call {
                    callee: Callee::Entity(entity),
                    ..
                } = rvalue
                {
                    callees.insert(entity.clone());
                }
            }
        }
    }

    callees
}

/// The order the modules are instantiated in: a provider before the modules that import it.
///
/// An import of an extern is a host function and not an edge; every other import names a module
/// of the project ([ADR-0021]). The graph is acyclic by design, so the walk always terminates.
///
/// [adr-0021]: ../../../docs/adr/0021-translation-units.md
fn module_order(
    modules: &[(String, Compiled)],
    externals: &BTreeSet<EntityLoc<FunctionLoc>>,
) -> Vec<usize> {
    let by_name: BTreeMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(index, (name, _))| (name.as_str(), index))
        .collect();
    let mut deps: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); modules.len()];

    for (index, (_, module)) in modules.iter().enumerate() {
        for import in &module.module.imports {
            if externals.contains(&import.entity) {
                continue;
            }

            let provider = by_name.get(import.module.as_str()).unwrap_or_else(|| {
                panic!(
                    "the harness to compile `{}`, which provides `{}`",
                    import.module, import.name,
                )
            });

            if *provider != index {
                deps[index].insert(*provider);
            }
        }
    }

    let mut order = Vec::with_capacity(modules.len());
    let mut placed = vec![false; modules.len()];

    loop {
        let mut progressed = false;

        for index in 0..modules.len() {
            if !placed[index] && deps[index].iter().all(|dependency| placed[*dependency]) {
                placed[index] = true;
                order.push(index);
                progressed = true;
            }
        }

        if !progressed {
            break;
        }
    }

    assert_eq!(
        order.len(),
        modules.len(),
        "the imports of the project to form no cycle",
    );

    order
}

/// The entry point of a project: the first exported function called `main` that takes no word.
///
/// The signature the language will require of `main` --- `() -> Unit` --- is not checked yet.
fn entry_of(modules: &[Compiled]) -> Option<(usize, String, EntityLoc<FunctionLoc>)> {
    for (index, module) in modules.iter().enumerate() {
        for function in &module.module.functions {
            if function.name != "main"
                || !function.exported
                || !function.signature.params.is_empty()
            {
                continue;
            }

            let BodyLoc::Function(loc) = &function.owner.item else {
                continue;
            };
            let entity = EntityLoc {
                module: function.owner.module,
                item: loc.clone(),
            };

            return Some((index, function.name.clone(), entity));
        }
    }

    None
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
