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

mod lexer;
mod parser;
mod syntax;
mod token_source;

use mlkc_parser_core::AnyParse;

use crate::{parser::Parser, syntax::module::parse_module_root};

/// Parses `source` as a module.
///
/// The diagnostics of the parse are attached to the returned parse, and the ranges they
/// point at are offsets into `source`.
pub fn parse(source: &str) -> AnyParse {
    let mut parser = Parser::from_str(source);

    parse_module_root(&mut parser);

    parser.finish()
}
