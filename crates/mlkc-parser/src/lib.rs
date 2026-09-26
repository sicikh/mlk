//! The parser of MLK.
//!
//! The parser turns the source of a module into a lossless syntax tree and a list of
//! diagnostics. It is built on [`mlkc_parser_core`], the event based parser infrastructure
//! that the parse rules are written against:
//!
//! 1. the lexer turns the source into tokens, trivia included;
//! 2. the token source collects the trivia, so that the rules only ever see the tokens of
//!    the language, and distributes the trivia over the tree in the end;
//! 3. the parser runs the rules, which emit events and diagnostics;
//! 4. the tree sink assembles the events into the green tree, the one that the diagnostics
//!    are attached to.
//!
//! The tree is lossless: the text of the root node is the source that was parsed, and a
//! mistake does not lose the tokens around it — they end up inside a `Bogus*` node.
//!
//! The green nodes of a tree are built through a cache: a token, and a node the cache holds,
//! are one allocation wherever a parse writes the same thing again, and a caller that parses
//! the next revision of a file through [`parse_with_cache`] shares the nodes the two
//! revisions have in common.

mod lexer;
mod parser;
mod syntax;
mod token_source;

use mlkc_parser_core::AnyParse;
use mlkc_rowan::NodeCache;

use crate::{parser::Parser, syntax::module::parse_module_root};

/// Parses `source` as a module.
///
/// The diagnostics of the parse are attached to the returned parse, and the ranges they
/// point at are offsets into `source`.
///
/// The parse builds the green nodes of the tree it makes and keeps none of them: a caller
/// that parses the revisions of a file one after another hands a cache to
/// [`parse_with_cache`] instead.
pub fn parse(source: &str) -> AnyParse {
    parse_with_cache(source, &mut NodeCache::default())
}

/// Parses `source` as a module, sharing the green nodes of the parses `cache` was used for.
///
/// A green node is decided by its shape, so a token that did not move, or a small node that
/// was not edited, is what an earlier parse built rather than something this parse builds
/// again: a caller that parses the revisions of one file, or the files of one project, holds
/// the trees of them as far apart as the edits are. Which nodes a cache keeps is the cache's
/// business, and a cache that is never handed to a second parse is what [`parse`] builds for
/// itself.
///
/// What the cache holds after a parse is the nodes of that parse: a node the parse did not
/// write again is dropped, and a node of the tree a caller keeps is what the caller keeps.
pub fn parse_with_cache(source: &str, cache: &mut NodeCache) -> AnyParse {
    let mut parser = Parser::from_str(source);

    parse_module_root(&mut parser);

    parser.finish(cache)
}
