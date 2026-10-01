//! What the back end emits is a well-formed WebAssembly module.
//!
//! Every module a test compiles is validated against the specification, with the proposals the
//! back end uses turned on: a body that carries a codegen diagnostic is not a value the driver
//! would assemble, and it is the diagnostics such a test reads.

mod harness;

use mlkc_codegen_wasm::CodegenDiag;

#[test]
fn the_fixtures_validate() {
    for name in ["arithmetic", "branching", "recursion"] {
        let compiled = harness::fixture(name);

        assert!(
            compiled.diagnostics.is_empty(),
            "`{name}` to be emitted without a report: {:?}",
            compiled.diagnostics,
        );
        compiled.validate();
        assert!(
            !compiled.wasm.exports.is_empty(),
            "`{name}` to export something",
        );
    }
}

#[test]
fn only_a_public_function_is_exported() {
    let compiled = harness::module("pub fun exported(): Int = 1\n\nfun hidden(): Int = 2\n");

    compiled.validate();

    let names: Vec<&str> = compiled
        .wasm
        .exports
        .iter()
        .map(|export| export.name.as_str())
        .collect();

    assert_eq!(names, ["exported"], "a private function is not exported");
}

#[test]
fn an_extern_function_becomes_an_import() {
    let compiled = harness::module("pub fun report(value: Int): Unit =\n    print-int(value)\n");

    compiled.validate();

    assert!(
        compiled.diagnostics.is_empty(),
        "`report` to be emitted: {:?}",
        compiled.diagnostics,
    );

    let imports: Vec<(&str, &str)> = compiled
        .wasm
        .imports
        .iter()
        .map(|import| (import.module.as_str(), import.name.as_str()))
        .collect();

    assert_eq!(
        imports,
        [("std::runtime", "print-int")],
        "the call to become an import of the module that declares it",
    );
}

#[test]
fn a_string_constant_is_reported_not_emitted() {
    let compiled = harness::module("pub fun text(): String = \"hello\"\n");

    assert!(
        compiled.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic,
                CodegenDiag::Unsupported { what, .. } if *what == "a string constant",
            )
        }),
        "the back end to report the string constant: {:?}",
        compiled.diagnostics,
    );
}
