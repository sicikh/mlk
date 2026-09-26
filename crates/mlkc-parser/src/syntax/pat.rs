//! Patterns: what a name is bound by --- the left-hand side of a `let`, and a parameter of a
//! function declaration.

use mlkc_parser_core::{
    parse_recovery::ParseRecoveryTokenSet,
    parsed_syntax::ParsedSyntax::{self, Present},
    prelude::*,
};
use mlkc_syntax::{
    SyntaxKind::{self, *},
    T,
};

use crate::{
    parser::MlkParser,
    syntax::{auxiliary::parse_name, parse_error::expected_pattern},
};

/// The tokens a broken pattern is recovered at: what follows a pattern wherever one is
/// written --- the `=` and the `in` of a `let`, the type of a parameter, and the end of the
/// list of parameters.
const PATTERN_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![T![=], T![in], T![:], T![,], T![')']];

/// Parses a pattern.
// test mlk a_binding_may_be_ignored
// fun ignored(): Int =
//     let _ = 1 in
//     2
pub(crate) fn parse_pat(p: &mut MlkParser) -> ParsedSyntax {
    match p.cur() {
        IDENT => parse_ident_pat(p),
        UNDERSCORE => parse_wildcard_pat(p),
        _ => ParsedSyntax::Absent,
    }
}

/// Parses a pattern, or what is written where one belongs and is not one.
///
/// A pattern is what a name is bound by, and a keyword or a literal is not one: what is
/// written there is read into a pattern that is not there, so that what the mistake costs is
/// the pattern. The tokens that follow one --- the `=` of a `let`, the type of a parameter ---
/// belong to the declaration rather than to the pattern, and are left where they are.
pub(crate) fn parse_pat_or_recover(p: &mut MlkParser) -> ParsedSyntax {
    match parse_pat(p).or_recover_with_token_set(
        p,
        &ParseRecoveryTokenSet::new(BOGUS_PAT, PATTERN_RECOVERY_SET),
        expected_pattern,
    ) {
        Ok(pat) => Present(pat),
        Err(_) => ParsedSyntax::Absent,
    }
}

/// Parses a pattern that binds a name.
fn parse_ident_pat(p: &mut MlkParser) -> ParsedSyntax {
    parse_name(p).map(|name| name.precede(p).complete(p, IDENT_PAT))
}

/// Parses the pattern that ignores its value.
fn parse_wildcard_pat(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T!["_"]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T!["_"]);

    Present(m.complete(p, WILDCARD_PAT))
}
