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

    // Where every operator sits in the precedence table, and what calls and parentheses do to
    // it.
    expressions: "valid/expressions.mlk",

    // Bindings: nested, ignored, and bound to the result of a call.
    let_in: "valid/let_in.mlk",

    // Paths of types, qualified and applied to type arguments, and the types a module leaves
    // to be inferred.
    types: "valid/types.mlk",

    // Declarations that declare no body: what they are is their signature.
    no_body: "valid/no_body.mlk",

    // A class and a function that share a name: the two are names of different namespaces, and
    // a name is read in the one the place it is written in asks for.
    namespaces: "valid/namespaces.mlk",

    // A module that declares the path it is known by in its project.
    preamble: "valid/preamble.mlk",

    // How far a declaration is visible, and the attributes it carries next to it.
    visibility: "valid/visibility.mlk",

    // Equals the shape of a literal: an integer, a string, and a subtraction that is an
    // operator rather than a sign.
    literals: "valid/literals.mlk",

    // A module that declares one name twice: the lowering reports it, and the item tree holds
    // both declarations, told apart by their names.
    duplicate_name: "invalid/duplicate_name.mlk",

    // A token where a declaration belongs: it is a bogus declaration, and the items around it
    // are read as if it were not there.
    stray_token: "invalid/stray_token.mlk",

    // A parameter the parser could not read: it is not a parameter of the signature, and the
    // declaration around it is still an item of the module.
    broken_parameter: "invalid/broken_parameter.mlk",

    // A body that holds no expression.
    missing_body_expr: "invalid/missing_body_expr.mlk",

    // A declaration without a name: it is an item of the module, under the name that is not
    // there, and it is not a duplicate of another one like it.
    missing_name: "invalid/missing_name.mlk",

    // A call that is never closed: the arguments that are there are read.
    unclosed_call: "invalid/unclosed_call.mlk",

    // A negative number: the language has no sign, so the expression is one the parser cannot
    // read, and the body of the function holds a missing expression.
    negative_literal: "invalid/negative_literal.mlk",

    // An attribute the language does not have: the HIR holds the attributes the compiler knows
    // what to do with, and this one is a mistake.
    unknown_attribute: "invalid/unknown_attribute.mlk",
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
