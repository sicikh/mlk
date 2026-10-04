//! The names of a compiled module, which a stack trace and a debugger read.
//!
//! The `name` section names the functions and the locals of a module. A lifted lambda has no
//! name of its own, and is named after the body that wrote it; the environment it is entered
//! with is its first parameter, and no source name binds it ([ADR-0026][adr-0026]).
//!
//! [adr-0026]: ../../../docs/adr/0026-closure-representation.md

mod harness;

use std::collections::BTreeMap;

use mlkc_codegen_wasm::DebugInfo;
use wasmparser::{BinaryReader, Name, NameSectionReader, Parser, Payload};

/// A program with a lambda that declares a parameter of its own.
const SOURCE: &str =
    "//- /main.mlk\nfun main(): Int =\n    let add = fn(x: Int) -> x + 1 in\n    add(1)\n";

/// The names the `name` section holds: the name of every function by its index, and the name of
/// every local by the index of its function and the place of the local.
fn names(bytes: &[u8]) -> (BTreeMap<u32, String>, BTreeMap<u32, BTreeMap<u32, String>>) {
    let mut functions = BTreeMap::new();
    let mut locals: BTreeMap<u32, BTreeMap<u32, String>> = BTreeMap::new();

    for payload in Parser::new(0).parse_all(bytes) {
        let Payload::CustomSection(section) = payload.expect("the module to parse") else {
            continue;
        };

        if section.name() != "name" {
            continue;
        }

        for entry in
            NameSectionReader::new(BinaryReader::new(section.data(), section.data_offset()))
        {
            match entry.expect("the name section to read") {
                Name::Function(map) => {
                    for naming in map {
                        let naming = naming.expect("a function name");

                        functions.insert(naming.index, naming.name.to_owned());
                    }
                },
                Name::Local(map) => {
                    for naming in map {
                        let naming = naming.expect("a local name");
                        let names = locals.entry(naming.index).or_default();

                        for local in naming.names {
                            let local = local.expect("a local");

                            names.insert(local.index, local.name.to_owned());
                        }
                    }
                },
                _ => {},
            }
        }
    }

    (functions, locals)
}

/// A lifted lambda is named after the body that wrote it, and its parameters read as the source
/// declared them; the environment it is entered with is unnamed.
#[test]
fn a_lambda_names_the_parameters_it_declared() {
    let compiled = harness::project_with(SOURCE, DebugInfo::None);
    let (functions, locals) = names(&compiled.wasm.bytes);

    let lambda = functions
        .iter()
        .find_map(|(index, name)| (name == "main::<mlkc@lambda-0>").then_some(*index))
        .expect("the lifted lambda to be named after the body that wrote it");
    let names = locals.get(&lambda).expect("the lambda to have locals");

    // The lambda is entered with its environment, which no name binds; the parameter it
    // declared is the local after it, and reads as the name the source wrote.
    assert_eq!(names.get(&0), None, "the environment to have no name");
    assert_eq!(names.get(&1).map(String::as_str), Some("x"));

    compiled.validate();
}

/// A function declared in a `local` is a function of the module of its own: it is named after the
/// entity that declares it and the name the declaration wrote, and its parameters read as the
/// source declared them.
#[test]
fn a_function_declared_in_a_local_names_its_parameters() {
    let compiled = harness::project_with(
        "//- /main.mlk\nfun main(): Int =\n    \
         local fun double(x: Int): Int = x * 2 in\n    \
         double(21)\n",
        DebugInfo::None,
    );
    let (functions, locals) = names(&compiled.wasm.bytes);

    let double = functions
        .iter()
        .find_map(|(index, name)| (name == "main::double").then_some(*index))
        .expect("the lifted function to be named under the entity that declared it");
    let names = locals.get(&double).expect("the function to have locals");

    assert_eq!(names.get(&0).map(String::as_str), Some("x"));

    compiled.validate();
}
