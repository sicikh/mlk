//! The spec tests of the parser.
//!
//! Every fixture under [`spec_test::SPECS_DIR`] has a test of its own, declared below, and
//! a snapshot next to it that holds the parse. The test names are the ones the review sees
//! when a fixture changes, so they are spelled out instead of derived from the file names.
//!
//! A new fixture needs a line here: the tests read the directory of the fixtures as well,
//! and fail when a fixture it holds has no test.
//!
//! The specs written next to the rules run as well, and need no declaration of any kind:
//! see [`inline_specs`].

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
    // The program the language is being built for: the declarations of a prelude, and a
    // function that binds a name and calls another one.
    hello: "valid/hello.mlk",

    // Where every operator sits in the precedence table, and what parentheses, calls and the
    // signs in front of an expression do to it.
    expressions: "valid/expressions.mlk",

    // Bindings: nested, ignored, and bound to the result of a call.
    let_in: "valid/let_in.mlk",

    // Paths of types, qualified and applied to type arguments, and paths of values, which
    // are written the same way.
    paths: "valid/paths.mlk",

    // Types left to be inferred, in a return type, a parameter and a type argument.
    infer_type: "valid/infer_type.mlk",

    // A module without items: the comment it holds belongs to the end of the file.
    comments: "valid/comments.mlk",

    // Strings: what a backslash escapes is part of the literal, and the literal ends at the
    // quote that is not escaped.
    strings: "valid/strings.mlk",

    // A `let` without its `in`: the binding expression is parsed and the diagnostic points
    // at the place where the body was supposed to start.
    missing_in: "invalid/missing_in.mlk",

    // A token that stands where a declaration belongs: it is kept as a bogus declaration,
    // and the declarations around it are parsed as if it were not there.
    stray_token: "invalid/stray_token.mlk",

    // A character the language does not have: the lexer reports it, and the parser
    // recovers from the token it became.
    lexical_error: "invalid/lexical_error.mlk",

    // A parameter that does not start with a name.
    broken_parameter: "invalid/broken_parameter.mlk",

    // A call that is never closed.
    unclosed_call: "invalid/unclosed_call.mlk",
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

/// The specs written next to the rules they cover.
///
/// A spec is a comment in the source of the parser, holding a module that must parse
/// cleanly:
///
/// ```text
/// // test mlk let_in
/// // fun main(): Int =
/// //     let x = 1 in
/// //     x
/// ```
///
/// The comment sits where the rule it covers lives, so that the rule and the module it is
/// expected to parse are read together. A mistake is described by a fixture under `specs`,
/// whose snapshot holds the diagnostics and the tree the parser recovered into.
#[test]
fn inline_specs() {
    spec_test::run_inline_specs();
}
