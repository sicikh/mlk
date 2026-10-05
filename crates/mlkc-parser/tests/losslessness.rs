//! Losslessness over arbitrary text ([ADR-0002]).
//!
//! The spec tests assert that the tree of a parse holds its source on the fixtures they name;
//! losslessness is a property of every text, so it is asserted here over arbitrary ones
//! ([ADR-0006]): `quickcheck` draws the texts character by character, empty strings and
//! control characters included, and the text the tree prints is compared with the text that
//! was parsed.
//!
//! [adr-0002]: ../../../docs/adr/0002-lossless-syntax-tree.md
//! [adr-0006]: ../../../docs/adr/0006-snapshot-testing.md

use mlkc_parser::{parse, parse_with_cache};
use mlkc_parser_core::AnyParse;
use mlkc_rowan::NodeCache;
use mlkc_syntax::MlkLanguage;
use quickcheck_macros::quickcheck;

/// The text of the tree a parse built.
fn tree_text(parsed: &AnyParse) -> String {
    parsed.syntax::<MlkLanguage>().to_string()
}

/// Parsing a text and printing its tree gives the text back.
#[quickcheck]
fn round_trip(source: String) -> bool {
    tree_text(&parse(&source)) == source
}

/// Both parses of a text through one cache build the tree of that text.
///
/// The second parse reads the green nodes the first one wrote through the cache, so a node
/// the cache handed back is under test as much as a node the parse wrote itself.
#[quickcheck]
fn round_trip_through_a_cache(source: String) -> bool {
    let mut cache = NodeCache::default();

    let first = parse_with_cache(&source, &mut cache);
    let second = parse_with_cache(&source, &mut cache);

    tree_text(&first) == source && tree_text(&second) == source
}
