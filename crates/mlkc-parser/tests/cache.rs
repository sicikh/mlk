//! What a caller that parses through a cache is promised.
//!
//! A cache is a table of green nodes, and a green node is decided by its shape, so a parse
//! that shares a cache with the parse of the revision before it is promised two things: the
//! tree of a revision is the tree of the text it was parsed from, whatever the cache holds,
//! and the parts the two revisions wrote the same way are one allocation rather than two.

use mlkc_parser::{parse, parse_with_cache};
use mlkc_parser_core::AnyParse;
use mlkc_rowan::{Direction, NodeCache};
use mlkc_syntax::{FUN_KW, MlkLanguage, SyntaxKind, SyntaxToken};

/// The first revision of a file.
const FIRST: &str = "\
fun main(): Int =
    1

fun other(): Int =
    2
";

/// The same file with one function written after it: the tokens of the first revision are the
/// tokens of this one, at the offsets they were at.
const SECOND: &str = "\
fun main(): Int =
    1

fun other(): Int =
    2

fun added(): Int =
    3
";

/// The first token of `kind` in a tree, which is what shows what a parse shared.
fn first_token(parsed: &AnyParse, kind: SyntaxKind) -> SyntaxToken {
    parsed
        .syntax::<MlkLanguage>()
        .descendants_tokens(Direction::Next)
        .find(|token| token.kind() == kind)
        .expect("the tree to hold a token of that kind")
}

/// The first `fun` of a parse: the token both revisions begin with.
fn first_fun(parsed: &AnyParse) -> SyntaxToken {
    first_token(parsed, FUN_KW)
}

#[test]
fn a_parse_through_a_cache_holds_the_source_it_was_parsed_from() {
    let mut cache = NodeCache::default();

    let first = parse_with_cache(FIRST, &mut cache);
    let second = parse_with_cache(SECOND, &mut cache);

    assert_eq!(first.diagnostics(), []);
    assert_eq!(second.diagnostics(), []);
    assert_eq!(first.syntax::<MlkLanguage>().to_string(), FIRST);
    assert_eq!(second.syntax::<MlkLanguage>().to_string(), SECOND);
}

#[test]
fn a_parse_shares_the_nodes_the_revision_before_it_wrote_the_same_way() {
    let mut cache = NodeCache::default();

    let first = parse_with_cache(FIRST, &mut cache);
    let second = parse_with_cache(SECOND, &mut cache);

    // The two revisions wrote the token the same way and it did not move, so the trees point
    // at one green node, at one offset. A key of a syntax element is not a value a reader
    // reads, which is why the comparison is spelled out rather than asserted with `assert_eq!`.
    assert!(
        first_fun(&first).key() == first_fun(&second).key(),
        "the cache did not share the token the two revisions wrote the same way"
    );
}

#[test]
fn a_parse_that_shares_no_cache_shares_no_nodes() {
    // Nothing of the first parse outlives it but its own tree, so nothing of it is what the
    // second parse points at.
    let first = parse(FIRST);
    let second = parse(SECOND);

    assert!(first_fun(&first).key() != first_fun(&second).key());
}
