//! The harness of the interpreter tests: a project read by the front end, and the two forms of
//! every body of it as programs the interpreter runs.
//!
//! A test writes a project the way [`mlkc_fixture`] writes one, and the harness compiles it with
//! the driver, records every body in both of its forms ([ADR-0019][adr-0019]), and declares
//! every `#[extern]` function of the project as an external of the program. What a test then
//! does is call a function by its canonical name and read the word it gives back.
//!
//! [adr-0019]: ../../../docs/adr/0019-mir.md

use std::collections::BTreeMap;

use mlkc_driver::Driver;
use mlkc_hir_def::{
    BodyLoc, EntityData, EntityLoc, FunctionLoc, ItemLocLike, ModuleId, Name, ProjectData,
    ProjectId,
};
use mlkc_interp::{Extern, Host, Program, Trap, Value, run};
use mlkc_vfs::VfsPath;

/// The name the project of a fixture is compiled under.
const PROJECT: &str = "app";

/// A program interpreted in both forms of MIR.
pub struct Compiled {
    /// The bodies in the CFG form, what the front end lowers.
    pub cfg: Program,
    /// The bodies in the SSA form, what the back end reads.
    pub ssa: Program,
    /// The entities of the program, by the canonical name a test calls them by:
    /// `app::main::double` is the `double` of the module `main` of the project `app`.
    entities: BTreeMap<String, EntityLoc<FunctionLoc>>,
}

impl Compiled {
    /// The entity a canonical name denotes.
    pub fn entity(&self, function: &str) -> &EntityLoc<FunctionLoc> {
        self.entities
            .get(function)
            .unwrap_or_else(|| panic!("the program to declare `{function}`"))
    }

    /// Runs the CFG form of `function`.
    pub fn cfg(&self, function: &str, args: &[Value], host: &mut dyn Host) -> Result<Value, Trap> {
        run(&self.cfg, self.entity(function), args, host)
    }

    /// Runs the SSA form of `function`.
    pub fn ssa(&self, function: &str, args: &[Value], host: &mut dyn Host) -> Result<Value, Trap> {
        run(&self.ssa, self.entity(function), args, host)
    }

    /// Runs the two forms of `function` with no host, and checks that they agree.
    ///
    /// This is the shape of a test of the interpreter: the two forms are one meaning, and a
    /// disagreement is a bug of the construction of SSA and not of the language.
    pub fn run(&self, function: &str, args: &[Value]) -> Result<Value, Trap> {
        let cfg = run(&self.cfg, self.entity(function), args, &mut NoHost)?;
        let ssa = run(&self.ssa, self.entity(function), args, &mut NoHost)?;

        assert_eq!(
            cfg, ssa,
            "the two forms of `{function}` to compute the same word",
        );

        Ok(ssa)
    }
}

/// The host of a run that is not meant to call one.
pub struct NoHost;

impl Host for NoHost {
    fn call(&mut self, function: &Extern, _args: &[Value]) -> Result<Value, Trap> {
        Err(Trap::host(format!(
            "the test called `{function}`, and no host implements it",
        )))
    }
}

/// A host that records what a program prints.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Output {
    /// What `print-int` and `print-bool` were called with, in the order they were called.
    pub printed: Vec<String>,
}

impl Host for Output {
    fn call(&mut self, function: &Extern, args: &[Value]) -> Result<Value, Trap> {
        let argument = args.first().and_then(Value::immediate);

        match function.name.as_str() {
            "print-int" => {
                let value = argument.expect("an immediate to print");
                self.printed.push(value.to_string());

                Ok(Value::Unit)
            },
            "print-bool" => {
                let value = argument.expect("an immediate to print");
                self.printed.push((value != 0).to_string());

                Ok(Value::Unit)
            },
            _ => {
                Err(Trap::host(format!(
                    "the test host does not implement `{function}`",
                )))
            },
        }
    }
}

/// Compiles the project a fixture writes, one module per mark ([`mlkc_fixture`]).
pub fn project(fixture: &str) -> Compiled {
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

    let mut compiled = Compiled {
        cfg: Program::new(),
        ssa: Program::new(),
        entities: BTreeMap::new(),
    };

    programs(&mut driver, &project, PROJECT, &mut compiled);

    let standard = ProjectId::new(mlkc_stdlib::PROJECT);
    programs(&mut driver, &standard, mlkc_stdlib::PROJECT, &mut compiled);

    // A compiler that bugged has no business answering a value: the report says what was being
    // computed when a pass panicked, and failing here is how a review sees it.
    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    compiled
}

/// Compiles one module, written as its source.
pub fn module(source: &str) -> Compiled {
    project(&format!("//- /main.mlk\n{source}"))
}

/// Records the bodies and the extern declarations of every module of a project.
fn programs(driver: &mut Driver, project: &ProjectId, name: &str, compiled: &mut Compiled) {
    let Some(index) = driver.module_index(project) else {
        return;
    };

    for (path, module) in index.iter() {
        let canonical = format!(
            "{name}::{}",
            path.iter().map(Name::as_str).collect::<Vec<_>>().join("::"),
        );
        let lowered = driver
            .lower(module.0)
            .expect("a module of the project to be lowered");

        for (item, _) in lowered.item_tree().entities() {
            let Some(EntityData::Function(data)) = lowered.item_tree().entity_data(item.clone())
            else {
                continue;
            };

            if !data.attributes.external {
                continue;
            }

            let Ok(loc) = FunctionLoc::try_from(item.clone()) else {
                continue;
            };
            let Some(name) = item.name() else {
                continue;
            };

            let function = EntityLoc { module, item: loc };
            let external = Extern {
                module: canonical.clone(),
                name: name.to_string(),
            };

            compiled
                .cfg
                .declare_extern(function.clone(), external.clone());
            compiled.ssa.declare_extern(function, external);
        }

        for body in lowered.bodies() {
            let BodyLoc::Function(loc) = &body.owner().item else {
                continue;
            };

            let owner = EntityLoc {
                module: body.owner().module,
                item: loc.clone(),
            };
            let Some(name) = loc.name() else {
                continue;
            };
            let key = format!("{canonical}::{name}");

            let cfg = driver
                .mir(body.owner())
                .expect("a clean body to have a CFG form");
            let ssa = driver
                .mir_ssa(body.owner())
                .expect("a clean body to have an SSA form");

            compiled.cfg.insert(cfg);
            compiled.ssa.insert(ssa);
            compiled.entities.insert(key, owner);
        }
    }
}
