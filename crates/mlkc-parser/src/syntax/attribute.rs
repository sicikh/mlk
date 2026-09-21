//! Attributes: the `@name` list that stands in front of a declaration.

use mlkc_parser_core::{parsed_syntax::ParsedSyntax::Present, prelude::*};
use mlkc_syntax::SyntaxKind::{self, *};

use crate::{
    parser::MlkParser,
    syntax::{auxiliary::parse_name, parse_error::expected_name},
};

/// Whether the parser is at a declaration that starts with the keyword `keyword`,
/// possibly preceded by an attribute list.
///
/// The keyword cannot simply be looked at when the declaration starts with `@`: both a
/// `fun` and a `type` declaration carry attributes, so a rule has to look past the
/// attributes to tell which of the two it is. Walking them is possible because an
/// attribute is always `@` followed by a name.
///
/// A malformed attribute list (`@` in front of something that is not a name) does not
/// match: the tokens around it are foreign to a declaration, and the caller recovers from
/// them as a broken declaration.
pub(crate) fn is_at_declaration(p: &mut MlkParser, keyword: SyntaxKind) -> bool {
    let mut index = 0;

    while p.nth_at(index, AT) && p.nth_at(index + 1, IDENT) {
        index += 2;
    }

    p.nth_at(index, keyword)
}

/// Parses the attributes of a declaration.
///
/// The list is created even when it stays empty: the parent node has a slot for it, and a
/// missing slot would be read as a missing list by everything that reads the tree.
pub(crate) fn parse_attribute_list(p: &mut MlkParser) -> CompletedMarker {
    let m = p.start();

    while p.at(AT) {
        // An attribute is present as soon as there is an `@`, so the rule cannot return
        // absent here and the mistake is reported by `parse_attribute`.
        parse_attribute(p).ok();
    }

    m.complete(p, ATTRIBUTE_LIST)
}

/// Parses a single attribute.
fn parse_attribute(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(AT) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(AT);
    parse_name(p).or_add_diagnostic(p, expected_name);

    Present(m.complete(p, ATTRIBUTE))
}
