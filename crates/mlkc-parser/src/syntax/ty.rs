//! Types, and the paths they are written with.
//!
//! A path is written the same way wherever it is written: what a type is named by and what a
//! value is named by are one rule of the grammar, and an expression reads it by the same
//! [`parse_path`]. What a path means is what differs, and that is the business of whoever
//! reads the tree.

use mlkc_parser_core::{
    parse_lists::ParseSeparatedList,
    parse_recovery::{ParseRecoveryTokenSet, RecoveryResult},
    parsed_syntax::ParsedSyntax::Present,
    prelude::*,
};
use mlkc_syntax::{
    SyntaxKind::{self, *},
    T,
};

use crate::{
    parser::MlkParser,
    syntax::{
        auxiliary::parse_name,
        parse_error::{expected_name, expected_type},
    },
};

/// The tokens a broken type is recovered at: the end of the list of types it is a part of,
/// or the end of the annotation it was written in.
const TYPE_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![T![,], T![']'], T![=], T![')']];

/// Parses a type.
// test mlk a_type_may_be_left_to_be_inferred
// fun inferred(value: _): _ =
//     value
pub(crate) fn parse_type(p: &mut MlkParser) -> ParsedSyntax {
    match p.cur() {
        UNDERSCORE => parse_infer_type(p),
        _ => parse_path(p).map(|path| path.precede(p).complete(p, PATH_TYPE)),
    }
}

/// Parses the type of a value the reader is meant to infer: `_`.
fn parse_infer_type(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T!["_"]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T!["_"]);

    Present(m.complete(p, INFER_TYPE))
}

/// Parses a path: a single segment, or a segment qualified by another path, as in `a.b.c`.
///
/// A qualified path is a path whose qualifier is a path of its own, and the qualifier is
/// what holds the dot: `a.b` is a path of the segment `b` qualified by `a.`. Every dot
/// therefore closes the path parsed so far into a qualifier, and starts a path around it.
//
// A path is what a type is written as and what an expression names a value by, so the rule
// is read from both sides of the grammar and lives apart from either.
// test mlk a_path_qualifies_its_segments
// fun qualified(value: std.core.Int): Unit =
//     value
pub(crate) fn parse_path(p: &mut MlkParser) -> ParsedSyntax {
    let segment = parse_path_segment(p);

    if segment.is_absent() {
        return ParsedSyntax::Absent;
    }

    // A single segment is a path already.
    let mut path = segment.map(|segment| segment.precede(p).complete(p, PATH));

    while p.at(T![.]) {
        let qualifier = path.precede(p);

        p.bump(T![.]);
        let qualifier = qualifier.complete(p, PATH_QUALIFIER);

        // The segment that follows the dot is the segment of the new path, not a child of
        // the qualifier: the qualifier is closed before it is parsed.
        let m = qualifier.precede(p);
        parse_path_segment(p).or_add_diagnostic(p, expected_name);
        path = Present(m.complete(p, PATH));
    }

    path
}

/// Parses a segment of a path: a name, and the type arguments applied to it.
fn parse_path_segment(p: &mut MlkParser) -> ParsedSyntax {
    let name = parse_name(p);

    if name.is_absent() {
        return ParsedSyntax::Absent;
    }

    let m = name.precede(p);

    parse_type_args(p).ok();

    Present(m.complete(p, PATH_SEGMENT))
}

/// Parses the type arguments of a path segment.
// test mlk type_arguments_are_applied_to_a_segment
// fun applied(value: Map[Int, String,]): Unit =
//     value
//
// test mlk type_arguments_nest
// fun nested(value: Map[Int, Map[String, Int]]): Unit =
//     value
fn parse_type_args(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T!['[']) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T!['[']);
    TypeArgListParse.parse_list(p);
    p.expect(T![']']);

    Present(m.complete(p, TYPE_ARGS))
}

/// The list of the type arguments of a path segment: `Map[Int, String]`.
struct TypeArgListParse;

impl ParseSeparatedList for TypeArgListParse {
    type Kind = SyntaxKind;
    type Parser<'source> = MlkParser<'source>;

    const LIST_KIND: SyntaxKind = TYPE_ARG_LIST;

    fn parse_element(&mut self, p: &mut MlkParser) -> ParsedSyntax {
        parse_type(p)
    }

    fn is_at_list_end(&self, p: &mut MlkParser) -> bool {
        p.at(T![']'])
    }

    fn recover(&mut self, p: &mut MlkParser, parsed_element: ParsedSyntax) -> RecoveryResult {
        parsed_element.or_recover_with_token_set(
            p,
            &ParseRecoveryTokenSet::new(BOGUS_TYPE, TYPE_RECOVERY_SET),
            expected_type,
        )
    }

    fn separating_element_kind(&mut self) -> SyntaxKind {
        T![,]
    }

    fn allow_trailing_separating_element(&self) -> bool {
        true
    }
}
