//! The spec tests of a project: the diagnostics the driver reports for its modules.
//!
//! Every project of the corpus has a test of its own, declared below, and a snapshot next to
//! it that holds what the driver reported about every module of it. The dumps of the passes
//! live in the suites of the crates that own them: this suite is about the messages a host
//! reads, and about the pipeline running to the end without an internal exception.
//!
//! The test names are the ones the review sees when a project changes, so they are spelled out
//! instead of derived from the names of the corpus. A new project needs a line here --- and in
//! the list of every suite of the pipeline --- and the suite fails when the corpus holds a
//! project that no test covers.

mod project_spec;

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

    // What the construction of MIR reports about a body the checker accepted: an integer
    // literal outside the 31-bit range of the word.
    out_of_range: "out-of-range",

    // A choice between expressions: the block that branches, the block of every arm, and the
    // block the arms meet in, in both forms of the MIR.
    branching: "branching",

    // What a lambda lowers to: the code lifted into the body's flat arena with its captures, the
    // closure the body creates over the slots they hold, and the `Capture` reads inside the code.
    lambdas: "lambdas",

    // What a function declared in a `local` lowers to: the signature the check gave it, the
    // direct call of it, and the function of the module its code is lifted into.
    locals: "locals",
}

/// A project of the corpus without a test is a snapshot nobody looks at.
#[test]
fn every_fixture_has_a_test() {
    let mut covered: Vec<String> = FIXTURES.iter().map(|it| (*it).to_owned()).collect();
    covered.sort();

    let fixtures = project_spec::fixtures();

    assert!(
        !fixtures.is_empty(),
        "no project was found in {}",
        mlkc_fixture::projects_dir().display(),
    );

    assert_eq!(
        fixtures, covered,
        "the projects of the corpus and the tests of this file disagree",
    );
}
