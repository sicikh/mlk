//! Patterns: the left-hand side of a `let`.

use mlkc_parser_core::{parsed_syntax::ParsedSyntax::Present, prelude::*};
use mlkc_syntax::{SyntaxKind::*, T};

use crate::{parser::MlkParser, syntax::auxiliary::parse_name};

/// Parses a pattern.
pub(crate) fn parse_pat(p: &mut MlkParser) -> ParsedSyntax {
    match p.cur() {
        IDENT => parse_ident_pat(p),
        UNDERSCORE => parse_wildcard_pat(p),
        _ => ParsedSyntax::Absent,
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
