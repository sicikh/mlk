//! The spec tests of the lowering.
//!
//! Every fixture under [`spec_test::SPECS_DIR`] has a test of its own, declared below, and a
//! snapshot next to it that holds what the lowering made of the module. The test names are
//! the ones the review sees when a fixture changes, so they are spelled out instead of
//! derived from the file names.
//!
//! A new fixture needs a line here: the tests read the directory of the fixtures as well,
//! and fail when a fixture it holds has no test.

mod spec_test;

use std::path::PathBuf;

/// Declares a test per fixture, and the list of the fixtures they cover.
macro_rules! spec_tests {
    ($($name:ident: $fixture:literal,)*) => {
        $(
            #[test]
            fn $name() {
                spec_test::run($fixture);
            }
        )*

        /// The fixtures the tests of this file cover.
        const FIXTURES: &[&str] = &[$($fixture,)*];
    };
}

spec_tests! {
    // The declarations of a prelude, which have no bodies, and a function that binds a name
    // and calls another one.
    hello: "valid/hello.mlk",

    // Where every operator sits in the precedence table, and what calls, parentheses and the
    // signs in front of an expression do to it.
    expressions: "valid/expressions.mlk",

    // Bindings: nested, ignored, and bound to the result of a call.
    let_in: "valid/let_in.mlk",

    // Paths of types: qualified, applied to type arguments, and a name read inside the type an
    // argument was applied to.
    types: "valid/types.mlk",

    // Declarations that declare no body: what they are is their signature.
    no_body: "valid/no_body.mlk",

    // A parameter is a pattern: a name the body binds, and a wildcard it ignores.
    parameters: "valid/parameters.mlk",

    // A class and a function that share a name: the two are names of different namespaces, and
    // a name is read in the one the place it is written in asks for.
    namespaces: "valid/namespaces.mlk",

    // A module that declares the path it is known by in its project.
    preamble: "valid/preamble.mlk",

    // How far a declaration is visible, and the attributes it carries next to it.
    visibility: "valid/visibility.mlk",

    // What a module takes from the rest of the project: a name, a name it is renamed to, and
    // the name of a module of the project.
    imports: "valid/imports.mlk",

    // A path rooted at the project: the keyword a module names its own project by, in the
    // preamble, in an import, in a type and in an expression.
    project_paths: "valid/project_paths.mlk",

    // Equals the shape of a literal: an integer, a string, and a subtraction that is an
    // operator rather than a sign; a string holds what its escapes decode to.
    literals: "valid/literals.mlk",

    // A module that declares one name twice: the lowering reports it, and the item tree holds
    // both declarations, told apart by their names.
    duplicate_name: "invalid/duplicate_name.mlk",

    // A token where a declaration belongs: it is a bogus declaration, and the items around it
    // are read as if it were not there.
    stray_token: "invalid/stray_token.mlk",

    // A parameter whose pattern the parser could not read: what it binds is not there, and the
    // declaration around it is still an item of the module.
    broken_parameter: "invalid/broken_parameter.mlk",

    // A body that holds no expression.
    missing_body_expr: "invalid/missing_body_expr.mlk",

    // A declaration without a name: it is an item of the module, under the name that is not
    // there, and it is not a duplicate of another one like it.
    missing_name: "invalid/missing_name.mlk",

    // A call that is never closed: the arguments that are there are read.
    unclosed_call: "invalid/unclosed_call.mlk",

    // An attribute the language does not have: the HIR holds the attributes the compiler knows
    // what to do with, and this one is a mistake.
    unknown_attribute: "invalid/unknown_attribute.mlk",

    // A module that imports one name twice: the first import is what the name denotes.
    duplicate_import: "invalid/duplicate_import.mlk",

    // A path of names carrying a type argument, written at the root of the path and at a
    // segment after it.
    path_with_type_args: "invalid/path_with_type_args.mlk",

    // The `project` keyword applied to type arguments, in a signature and in a body: a project
    // is not a type, and nothing is applied to it.
    project_with_type_args: "invalid/project_with_type_args.mlk",

    // The keyword written where a name belongs all over a module: the items and the body are
    // read whole, and the names the keyword stands in for are missing.
    project_written_as_a_name: "invalid/project_written_as_a_name.mlk",

    // The keyword written where a pattern belongs: the parameter keeps the type the module
    // wrote, the `let` keeps its value, and what binds a name is missing.
    project_where_a_pattern_belongs: "invalid/project_where_a_pattern_belongs.mlk",

    // `@extern` on a function with a body, and on a type: neither is a thing the language has.
    extern_misuse: "invalid/extern_misuse.mlk",

    // `@builtin` on a function with a body: a builtin is implemented by the compiler, and the
    // declaration is what puts its name in the scope of the module.
    builtin_with_body: "invalid/builtin_with_body.mlk",

    // The same attribute written twice on one declaration: what it says is what its first
    // writing says.
    repeated_attribute: "invalid/repeated_attribute.mlk",

    // One parameter name declared twice: the parameters of a function are what its body
    // binds, and a name a body reads is one name.
    duplicate_parameter_name: "invalid/duplicate_parameter_name.mlk",

    // An escape the language has no meaning for: the value keeps what the module wrote, and
    // the sequence is what the lowering reports.
    unknown_escape: "invalid/unknown_escape.mlk",

    // A name an import brings in is a name the module declares: the declaration is what the
    // name means, and the import is told about wherever either of them is written.
    import_of_declared_name: "invalid/import_of_declared_name.mlk",

    // A public function whose signature is not complete: what a caller of it depends on is the
    // surface of the module.
    public_without_signature: "invalid/public_without_signature.mlk",
}

/// A fixture without a test is a snapshot nobody looks at.
#[test]
fn every_fixture_has_a_test() {
    let mut covered: Vec<PathBuf> = FIXTURES
        .iter()
        .map(|fixture| spec_test::fixture_path(fixture))
        .collect();
    covered.sort();

    let fixtures = spec_test::fixtures();

    assert!(
        !fixtures.is_empty(),
        "no fixture was found under {}",
        spec_test::SPECS_DIR
    );

    assert_eq!(
        fixtures,
        covered,
        "the fixtures under {} and the tests of this file disagree",
        spec_test::SPECS_DIR
    );
}
