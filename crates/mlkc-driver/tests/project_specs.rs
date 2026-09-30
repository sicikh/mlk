//! The spec tests of a project: what its modules resolve to and check to, read as a snapshot.
//!
//! Every fixture under [`project_spec::SPECS_DIR`] has a test of its own, declared below, and a
//! snapshot next to it that holds what the driver made of the project: the surface of every
//! module, the bodies it holds, the scope it resolved to, the types its signatures and bodies
//! were checked to, and the diagnostics. The test names are the ones the review sees when a
//! fixture changes, so they are spelled out instead of derived from the directory names.
//!
//! A new fixture needs a line here: the tests read the directory of the fixtures as well,
//! and fail when a fixture it holds has no test.

mod project_spec;

use std::path::PathBuf;

/// Declares a test per fixture, and the list of the fixtures they cover.
macro_rules! project_specs {
    ($($name:ident: $fixture:literal,)*) => {
        $(
            #[test]
            fn $name() {
                project_spec::run($fixture);
            }
        )*

        /// The fixtures the tests of this file cover.
        const FIXTURES: &[&str] = &[$($fixture,)*];
    };
}

project_specs! {
    // What a module takes from the rest of the project: the name of a module, the names under a
    // prefix of module paths, and a path rooted at the project.
    imports: "imports",

    // The names of the language, which the standard library declares: a project's prelude names
    // them, the module it names re-exports them, and the walk follows the re-export.
    prelude: "prelude",

    // A name a module re-exports is the path the module wrote: the walk continues in the module
    // that reads it, and the name it ends at is one of another module.
    reexports: "reexports",

    // What a module is told about when a path names nothing: the diagnostics of a resolution,
    // and the names they left unresolved.
    broken: "broken",

    // What the checker makes of every expression of a body: the literals, the operators over
    // `Int` and `Bool`, a call, and a `let`, each of them with the type it was checked to.
    expressions: "expressions",

    // The signatures the first check requires: missing types, an inferred `_`, and a public
    // function the lowering has already reported.
    missing_types: "missing-types",

    // What a check finds: a value where another one belongs, a callee that is not a function,
    // a call of the wrong arity, and a class applied to arguments.
    type_errors: "type-errors",

    // The paths of a body, which a resolution never walks: a name a module keeps to itself, a
    // class written where a value belongs, and a module that is not there.
    hidden_name: "hidden-name",
}

/// A fixture without a test is a snapshot nobody looks at.
#[test]
fn every_fixture_has_a_test() {
    let mut covered: Vec<PathBuf> = FIXTURES
        .iter()
        .map(|fixture| project_spec::fixture_path(fixture))
        .collect();
    covered.sort();

    let fixtures = project_spec::fixtures();

    assert!(
        !fixtures.is_empty(),
        "no fixture was found under {}",
        project_spec::SPECS_DIR
    );

    assert_eq!(
        fixtures,
        covered,
        "the fixtures under {} and the tests of this file disagree",
        project_spec::SPECS_DIR
    );
}
