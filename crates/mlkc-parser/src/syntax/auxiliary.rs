//! The leaf nodes that several rules of the grammar need.

use mlkc_parser_core::{parsed_syntax::ParsedSyntax::Present, prelude::*};
use mlkc_syntax::SyntaxKind::*;

use crate::parser::MlkParser;

/// Parses a name.
///
/// A name is a single identifier. Identifiers are never keywords — the lexer tells the two
/// apart — so a rule that is at `IDENT` does not have to look at the text.
pub(crate) fn parse_name(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(IDENT) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();
    p.bump(IDENT);

    Present(m.complete(p, NAME))
}
