//! The snapshot tests of the back end: what every fixture under the specs directory compiles to.
//!
//! Every fixture has a test of its own, declared below, and a snapshot next to it that holds the
//! WASM module the back end assembled for it, printed by `wasmprinter`, and what the back end
//! reported about the program. The test names are the ones the review sees when a fixture
//! changes, so they are spelled out instead of derived from the directory names.
//!
//! A new fixture needs a line here: the tests read the directory of the fixtures as well, and
//! fail when a fixture it holds has no test.
//!
//! `INSTA_UPDATE=always cargo test -p mlkc-codegen-wasm` rewrites the snapshots, and the diff of
//! them is what a review reads ([ADR-0006], [ADR-0020]).
//!
//! [adr-0006]: ../../../docs/adr/0006-snapshot-testing.md
//! [adr-0020]: ../../../docs/adr/0020-wasm-backend.md

mod harness;

use std::{fmt::Write as _, fs, path::PathBuf};

/// Declares a test per fixture, and the list of the fixtures they cover.
macro_rules! wasm_specs {
    ($($name:ident: $fixture:literal,)*) => {
        $(
            #[test]
            fn $name() {
                run($fixture);
            }
        )*

        /// The fixtures the tests of this file cover.
        const FIXTURES: &[&str] = &[$($fixture,)*];
    };
}

wasm_specs! {
    // What an operator over immediates compiles to: the prologue that refines every parameter
    // once, the instruction of the operator over unboxed `i32`s, and the boxed word it computes.
    arithmetic: "arithmetic",

    // What a choice compiles to: one case of the dispatch loop per block, the parameter an edge
    // passes to its target, and a call to a function of the same module.
    branching: "branching",

    // What a recursive function compiles to: the call whose index the layout assigned, and the
    // dispatch loop that runs again with the arguments the edge passed.
    recursion: "recursion",
}

/// Runs the fixture `fixture`, and checks it against its snapshot.
fn run(fixture: &str) {
    let compiled = harness::fixture(fixture);
    let mut snapshot = String::new();

    snapshot.push_str("```wat\n");
    snapshot.push_str(&compiled.wat());

    if !snapshot.ends_with('\n') {
        snapshot.push('\n');
    }

    snapshot.push_str("```\n");

    if !compiled.diagnostics.is_empty() {
        snapshot.push_str("\nWhat the back end reported:\n\n");

        for diagnostic in &compiled.diagnostics {
            writeln!(snapshot, "- {diagnostic}").expect("writing to a string to never fail");
        }
    }

    insta::with_settings!({
        prepend_module_to_snapshot => false,
        snapshot_path => harness::SPECS_DIR,
    }, {
        insta::assert_snapshot!(fixture, snapshot);
    });
}

/// The fixtures under [`harness::SPECS_DIR`], as the paths the tests name them by.
fn fixtures() -> Vec<PathBuf> {
    let directory = harness::specs_dir();
    let entries = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    let mut fixtures = Vec::new();

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.extension() == Some("mlk".as_ref()) {
            fixtures.push(path);
        }
    }

    fixtures.sort();

    fixtures
}

/// A fixture without a test is a snapshot nobody looks at.
#[test]
fn every_fixture_has_a_test() {
    let mut covered: Vec<PathBuf> = FIXTURES
        .iter()
        .map(|fixture| harness::fixture_path(fixture))
        .collect();
    covered.sort();

    let fixtures = fixtures();

    assert!(
        !fixtures.is_empty(),
        "no fixture was found under {}",
        harness::SPECS_DIR,
    );

    assert_eq!(
        fixtures,
        covered,
        "the fixtures under {} and the tests of this file disagree",
        harness::SPECS_DIR,
    );
}
