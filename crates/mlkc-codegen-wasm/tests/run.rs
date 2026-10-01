//! Running a project: what the interpreter computes and what the modules compute agree.
//!
//! A run is a project of several modules with an entry point, written the way [`mlkc_fixture`]
//! writes one ([`harness::RUNS_DIR`]). Every run is compiled to WASM, interpreted in both forms
//! of MIR, and instantiated with `wasmtime`; the three have to print the same thing, and what
//! they print is the snapshot. The host functions are the ones the tests implement:
//! `print-int` and `print-bool` of `std::runtime` ([ADR-0015], [ADR-0021]).
//!
//! A run without a test is a snapshot nobody looks at, so a new one needs a line below.
//!
//! [adr-0015]: ../../../docs/adr/0015-standard-library.md
//! [adr-0021]: ../../../docs/adr/0021-translation-units.md

mod harness;

use std::{fs, path::PathBuf};

use harness::RunProject;
use mlkc_interp::{Extern, Host, Trap, Value};
use wasmtime::{
    AnyRef, Caller, Config, Engine, FuncType, I31, Instance, Linker, Module, RefType, Store, Val,
    ValType,
};

/// The directory the runs of a project live in, as the paths the tests name them by.
const RUNS_DIR: &str = harness::RUNS_DIR;

/// Declares a test per run, and the list of the runs they cover.
macro_rules! runs {
    ($($name:ident: $run:literal,)*) => {
        $(
            #[test]
            fn $name() {
                run($run);
            }
        )*

        /// The runs the tests of this file cover.
        const RUNS: &[&str] = &[$($run,)*];
    };
}

runs! {
    // Two modules of one project: `main` prints what a function of the other module computes,
    // and the interpreter, the two forms of MIR, and the instantiated modules all agree.
    two_modules: "two-modules",

    // Four modules linked in a chain: a call of a call, a boolean, and a choice are read back
    // the same way by the interpreter and by the modules.
    messages: "messages",
}

/// Runs the run `name`, and checks it against its snapshot.
fn run(name: &str) {
    let project = harness::run(name);

    for module in &project.modules {
        module.validate();

        assert!(
            module.diagnostics.is_empty(),
            "`{}` to be emitted without a report: {:?}",
            module.module.name,
            module.diagnostics,
        );
    }

    let cfg = interpret(&project, &project.cfg);
    let ssa = interpret(&project, &project.ssa);
    let wasm = instantiate(&project);

    assert_eq!(
        cfg.printed, ssa.printed,
        "the two forms of the project to agree",
    );
    assert_eq!(
        ssa.printed, wasm.printed,
        "the interpreter and the modules to agree",
    );

    let mut snapshot = String::new();

    snapshot.push_str("Modules, in the order they are linked:\n");

    for name in &project.names {
        snapshot.push_str(&format!("- `{name}`\n"));
    }

    if let Some((index, name, _)) = &project.entry {
        snapshot.push_str(&format!("\nEntry: `{}::{name}`\n", project.names[*index]));
    }

    snapshot.push_str("\nWhat the program printed:\n");

    for line in &ssa.printed {
        snapshot.push_str(&format!("- {line}\n"));
    }

    insta::with_settings!({
        prepend_module_to_snapshot => false,
        snapshot_path => RUNS_DIR,
    }, {
        insta::assert_snapshot!(name, snapshot);
    });
}

/// A run without a test is a snapshot nobody looks at.
#[test]
fn every_run_has_a_test() {
    let mut covered: Vec<PathBuf> = RUNS.iter().map(|run| harness::run_path(run)).collect();
    covered.sort();

    let runs = runs();

    assert!(!runs.is_empty(), "no run was found under {RUNS_DIR}");

    assert_eq!(
        runs, covered,
        "the runs under {RUNS_DIR} and the tests of this file disagree",
    );
}

/// The runs under [`RUNS_DIR`], as the paths the tests name them by.
fn runs() -> Vec<PathBuf> {
    let directory = harness::runs_dir();
    let entries = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    let mut runs = Vec::new();

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.extension() == Some("mlk".as_ref()) {
            runs.push(path);
        }
    }

    runs.sort();

    runs
}

/// What a program printed, as a host records it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Output {
    printed: Vec<String>,
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

/// Runs the entry point of the project in the interpreter.
fn interpret(project: &RunProject, program: &mlkc_interp::Program) -> Output {
    let mut output = Output::default();

    if let Some((_, _, entity)) = &project.entry {
        mlkc_interp::run(program, entity, &[], &mut output).expect("the program to run");
    }

    output
}

/// The store the host functions of a run record into.
#[derive(Debug, Default)]
struct HostData {
    printed: Vec<String>,
}

/// Instantiates every module of the project and runs the entry point.
fn instantiate(project: &RunProject) -> Output {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config).expect("the engine to start");
    let mut store = Store::new(&engine, HostData::default());
    let mut linker: Linker<HostData> = Linker::new(&engine);

    // The host implements what the modules import and no module provides: the externs of the
    // project, by the canonical name the linker uses ([ADR-0021]).
    //
    // [ADR-0021]: ../../../docs/adr/0021-translation-units.md
    for ((module, name), arity) in &project.externs {
        let ty = FuncType::new(
            &engine,
            (0..*arity).map(|_| ValType::Ref(RefType::EQREF)),
            [ValType::Ref(RefType::EQREF)],
        );
        let recorded = name.clone();

        linker
            .func_new(
                module,
                name,
                ty,
                move |mut caller: Caller<'_, HostData>, params, results| {
                    let argument = match &params[0] {
                        Val::AnyRef(Some(value)) => {
                            value
                                .as_i31(&caller)?
                                .map(|value| value.get_i32())
                                .unwrap_or_default()
                        },
                        _ => 0,
                    };

                    caller.data_mut().printed.push(if recorded == "print-bool" {
                        (argument != 0).to_string()
                    } else {
                        argument.to_string()
                    });

                    results[0] = Val::AnyRef(Some(AnyRef::from_i31(
                        &mut caller,
                        I31::new_i32(0).expect("zero to be an immediate"),
                    )));

                    Ok(())
                },
            )
            .expect("the host function to be defined");
    }

    let mut instances: Vec<Instance> = Vec::new();

    for (index, compiled) in project.modules.iter().enumerate() {
        let module = Module::new(&engine, &compiled.wasm.bytes).expect("the module to compile");
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap_or_else(|error| panic!("`{}` to instantiate: {error}", project.names[index]));

        // The exports of a module are what the modules after it import: the linker resolves an
        // import by the canonical name of the provider ([ADR-0021]).
        for export in &compiled.wasm.exports {
            let function = instance
                .get_func(&mut store, &export.name)
                .unwrap_or_else(|| {
                    panic!("`{}::{}` to be exported", project.names[index], export.name)
                });

            linker
                .define(&store, &project.names[index], &export.name, function)
                .expect("the export to be defined");
        }

        instances.push(instance);
    }

    if let Some((index, name, _)) = &project.entry {
        let function = instances[*index]
            .get_func(&mut store, name)
            .unwrap_or_else(|| panic!("`{name}` to be exported"));
        let mut results = [Val::AnyRef(None)];

        function
            .call(&mut store, &[], &mut results)
            .expect("the entry point to run");
    }

    Output {
        printed: store.into_data().printed,
    }
}
