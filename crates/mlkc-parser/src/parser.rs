//! The parser of MLK: the driver that turns the source into a syntax tree.
//!
//! The parse rules in [`crate::syntax`] never build a tree. They emit *events* — start a
//! node, emit a token, finish the node — and the diagnostic of every mistake they find.
//! This module runs the rules and hands the events to a [`LosslessTreeSink`], which is the
//! one that assembles the green tree and decides where every piece of trivia goes.
//! No rule ever looks at trivia.

use mlkc_parser_core::{
    AnyParse, NodeParse, Parser as ParserTrait, ParserContext, diagnostic::merge_diagnostics,
    event, token_source::TokenSource as TokenSourceTrait, tree_sink::LosslessTreeSink,
};
use mlkc_rowan::NodeCache;
use mlkc_syntax::{MlkLanguage, SyntaxKind};
use mlkc_syntax_factory::SyntaxFactory;

use crate::{lexer::Lexer, token_source::TokenSource};

/// The parser of MLK.
///
/// One parser parses one source: it owns the token source and the events the rules emitted.
/// It is borrowed mutably by the rules, which read the current token through the
/// [`ParserTrait`] methods.
pub(crate) struct Parser<'src> {
    context: ParserContext<SyntaxKind>,
    source: TokenSource<'src>,
}

/// The parser the parse rules are written for.
pub(crate) type MlkParser<'src> = Parser<'src>;

impl<'src> Parser<'src> {
    /// Creates a parser that reads `source`.
    pub(crate) fn from_str(source: &'src str) -> Self {
        Self {
            context: ParserContext::default(),
            source: TokenSource::from_str(source),
        }
    }

    /// Consumes the parser and returns the tree of the source, with the diagnostics
    /// of the parser and of the lexer.
    ///
    /// The green nodes of the tree are the ones `cache` holds where it holds them, and the
    /// ones this parse writes into it where it does not.
    pub(crate) fn finish(self, cache: &mut NodeCache) -> AnyParse {
        // The text is borrowed from the caller of `from_str`, not from the token source,
        // so it stays alive after the token source is consumed below.
        let text = self.source.text();

        let (events, parser_diagnostics) = self.context.finish();
        let (trivia, lexer_diagnostics) = self.source.finish();

        // A lexical mistake is described more precisely than the parse rule that trips
        // over its token ("unterminated string literal" instead of "expected an
        // expression"), and the two often point at the same offset. The merge keeps the
        // first diagnostic of the two, so the lexer goes first.
        let diagnostics = merge_diagnostics(lexer_diagnostics, parser_diagnostics);

        let mut sink: LosslessTreeSink<'_, MlkLanguage, SyntaxFactory> =
            LosslessTreeSink::with_cache(text, &trivia, cache);
        event::process(&mut sink, events, diagnostics);

        let (root, diagnostics) = sink.finish();
        let root = root
            .as_send()
            .expect("the rules always parse the root node");

        NodeParse::new(root, diagnostics).into()
    }

    /// The kind of the `n`th token after the current one, trivia excluded.
    ///
    /// The [`ParserTrait`] method needs the lexer type spelled out, which no parse rule
    /// should care about; this wrapper fixes it to the lexer of this crate.
    pub(crate) fn nth(&mut self, n: usize) -> SyntaxKind {
        <Self as ParserTrait>::nth::<Lexer<'src>>(self, n)
    }

    /// Whether the `n`th token after the current one is of kind `kind`.
    pub(crate) fn nth_at(&mut self, n: usize, kind: SyntaxKind) -> bool {
        <Self as ParserTrait>::nth_at::<Lexer<'src>>(self, n, kind)
    }
}

impl<'src> ParserTrait for Parser<'src> {
    type Kind = SyntaxKind;
    type Source = TokenSource<'src>;

    fn context(&self) -> &ParserContext<SyntaxKind> {
        &self.context
    }

    fn context_mut(&mut self) -> &mut ParserContext<SyntaxKind> {
        &mut self.context
    }

    fn source(&self) -> &TokenSource<'src> {
        &self.source
    }

    fn source_mut(&mut self) -> &mut TokenSource<'src> {
        &mut self.source
    }
}

#[cfg(test)]
mod tests {
    use mlkc_syntax::{MlkLanguage, UNICODE_BOM};

    /// The byte order mark cannot be written in a fixture or in a spec comment: it is an
    /// invisible character at the start of a file.
    #[test]
    fn the_byte_order_mark_ends_up_in_the_tree() {
        let source = "\u{feff}fun main(): Unit = 1";

        let parsed = crate::parse(source);

        assert!(
            parsed.diagnostics().is_empty(),
            "the mark is not a mistake: {:?}",
            parsed.diagnostics()
        );

        let tree = parsed.syntax::<MlkLanguage>();
        assert_eq!(tree.to_string(), source);

        let first_token = tree.first_token().expect("the tree to hold the source");
        assert_eq!(first_token.kind(), UNICODE_BOM);
    }
}
