//! The token source of MLK: the layer between the lexer and the parser.
//!
//! The lexer lexes trivia like any other token; the token source is the one that
//! knows what trivia is. It collects the trivia as the parser bumps over tokens
//! and never lets it through: [`current`](TokenSourceTrait::current) is always a
//! non-trivia token.
//!
//! Every piece of trivia belongs either to the token before it or to the token
//! after it. Trivia on the same line as the token before it *trails* that token,
//! everything from the line break on is *leading* trivia of the next one. The
//! tree sink needs the split to place the trivia in the tree unambiguously.

use mlkc_parser_core::{
    diagnostic::ParseDiagnostic,
    lexer::BufferedLexer,
    token_source::{
        BumpWithContext, TokenSource as TokenSourceTrait, TokenSourceCheckpoint,
        TokenSourceWithBufferedLexer, Trivia,
    },
};
use mlkc_rowan::{TextRange, TextSize, TriviaPieceKind};
use mlkc_syntax::SyntaxKind::{self, *};

use crate::lexer::Lexer;

/// The state of the token source: where it is, and how much trivia it has seen.
pub(crate) type Checkpoint = TokenSourceCheckpoint<SyntaxKind>;

/// Token source for the parser that skips over any trivia token.
pub(crate) struct TokenSource<'src> {
    lexer: BufferedLexer<SyntaxKind, Lexer<'src>>,
    /// The trivia the parser has skipped so far, in the order it was lexed.
    trivia_list: Vec<Trivia>,
}

impl<'src> TokenSource<'src> {
    /// Creates a token source over `source`, positioned at its first token.
    pub fn from_str(source: &'src str) -> Self {
        let lexer = BufferedLexer::new(Lexer::from_str(source));
        let mut token_source = Self {
            lexer,
            trivia_list: Vec::new(),
        };

        token_source.next_non_trivia_token(true);

        token_source
    }

    /// Lexes tokens until the current one is not trivia, collecting the trivia.
    ///
    /// `first_token` is `true` for the first token of the source and for the token
    /// that follows a skipped one. There is no token before them to attach trivia
    /// to, so everything skipped is leading trivia of the token that is found.
    fn next_non_trivia_token(&mut self, first_token: bool) {
        // Only the trivia on the same line as the previous token trails it.
        let mut trailing = !first_token;

        loop {
            let kind = self.lexer.next_token(());

            // The byte order mark is part of the file, but not part of the language:
            // it is kept as trivia, so that the tree stays lossless.
            // (A grammar that wants it as a token of its own gives the root node a
            // `bom: 'UNICODE_BOM'?` field, the way biome does.)
            if kind == UNICODE_BOM {
                self.trivia_list.push(Trivia::new(
                    TriviaPieceKind::Skipped,
                    self.lexer.current_range(),
                    trailing,
                ));
                continue;
            }

            let Ok(trivia_kind) = TriviaPieceKind::try_from(kind) else {
                // The current token is not trivia: the parser can look at it.
                break;
            };

            // From the line break on, the trivia belongs to the next token.
            if trivia_kind.is_newline() {
                trailing = false;
            }

            self.trivia_list.push(Trivia::new(
                trivia_kind,
                self.lexer.current_range(),
                trailing,
            ));
        }
    }

    /// Creates a checkpoint the token source can later return to with [Self::rewind].
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            trivia_len: self.trivia_list.len() as u32,
            lexer_checkpoint: self.lexer.checkpoint(),
        }
    }

    /// Restores the token source to a previous checkpoint.
    ///
    /// The trivia the parser collected since the checkpoint is dropped: it belongs
    /// to tokens the parser decided not to parse.
    pub fn rewind(&mut self, checkpoint: Checkpoint) {
        assert!(self.trivia_list.len() >= checkpoint.trivia_len as usize);

        self.trivia_list.truncate(checkpoint.trivia_len as usize);
        self.lexer.rewind(checkpoint.lexer_checkpoint);
    }
}

impl<'src> TokenSourceTrait for TokenSource<'src> {
    type Kind = SyntaxKind;

    /// Returns the kind of the current non-trivia token
    #[inline(always)]
    fn current(&self) -> Self::Kind {
        self.lexer.current()
    }

    /// Returns the range of the current non-trivia token
    #[inline(always)]
    fn current_range(&self) -> TextRange {
        self.lexer.current_range()
    }

    #[inline(always)]
    fn text(&self) -> &'src str {
        self.lexer.source()
    }

    #[inline(always)]
    fn position(&self) -> TextSize {
        self.current_range().start()
    }

    #[inline(always)]
    fn has_preceding_line_break(&self) -> bool {
        self.lexer.has_preceding_line_break()
    }

    #[inline(always)]
    fn has_preceding_whitespace(&self) -> bool {
        self.lexer.has_preceding_whitespace()
    }

    #[inline(always)]
    fn bump(&mut self) {
        self.bump_with_context(());
    }

    #[inline(always)]
    fn skip_as_trivia(&mut self) {
        self.skip_as_trivia_with_context(());
    }

    fn finish(self) -> (Vec<Trivia>, Vec<ParseDiagnostic>) {
        (self.trivia_list, self.lexer.finish())
    }
}

impl BumpWithContext for TokenSource<'_> {
    /// MLK is lexed the same way everywhere: there is nothing to switch between.
    type Context = ();

    #[inline(always)]
    fn bump_with_context(&mut self, _context: Self::Context) {
        // The end of the file is not a token the parser can skip over.
        if self.current() != EOF {
            self.next_non_trivia_token(false);
        }
    }

    /// Skips the current token as skipped token trivia.
    #[inline(always)]
    fn skip_as_trivia_with_context(&mut self, _context: Self::Context) {
        if self.current() != EOF {
            self.trivia_list.push(Trivia::new(
                TriviaPieceKind::Skipped,
                self.current_range(),
                false,
            ));

            self.next_non_trivia_token(true);
        }
    }
}

impl<'src> TokenSourceWithBufferedLexer<Lexer<'src>> for TokenSource<'src> {
    fn lexer(&mut self) -> &mut BufferedLexer<SyntaxKind, Lexer<'src>> {
        &mut self.lexer
    }
}

#[cfg(test)]
mod tests {
    use mlkc_parser_core::token_source::NthToken;

    use super::*;

    /// The trivia the source collected, as `(kind, trailing)` pairs.
    fn trivia(source: &TokenSource<'_>) -> Vec<(TriviaPieceKind, bool)> {
        source
            .trivia_list
            .iter()
            .map(|trivia| (trivia.kind(), trivia.trailing()))
            .collect()
    }

    #[test]
    fn the_current_token_is_the_first_non_trivia_token() {
        let mut source = TokenSource::from_str("  // a comment\nfun main");

        assert_eq!(source.current(), FUN_KW);
        assert_eq!(
            source.current_range(),
            TextRange::at(TextSize::from(15), TextSize::from(3))
        );
        assert_eq!(source.text(), "  // a comment\nfun main");

        assert_eq!(trivia(&source), [
            (TriviaPieceKind::Whitespace, false),
            (TriviaPieceKind::SingleLineComment, false),
            (TriviaPieceKind::Newline, false),
        ]);

        source.bump();

        assert_eq!(source.current(), IDENT);
        assert_eq!(trivia(&source), [
            (TriviaPieceKind::Whitespace, false),
            (TriviaPieceKind::SingleLineComment, false),
            (TriviaPieceKind::Newline, false),
            (TriviaPieceKind::Whitespace, true),
        ]);
    }

    #[test]
    fn trivia_after_a_line_break_leads_the_next_token() {
        let mut source = TokenSource::from_str("fun\n  main");

        source.bump();

        assert_eq!(source.current(), IDENT);
        assert_eq!(trivia(&source), [
            (TriviaPieceKind::Newline, false),
            (TriviaPieceKind::Whitespace, false),
        ]);
        assert!(source.has_preceding_line_break());
        assert!(source.has_preceding_whitespace());
    }

    #[test]
    fn a_skipped_token_becomes_trivia() {
        let mut source = TokenSource::from_str("let x = 1");

        source.skip_as_trivia();

        assert_eq!(source.current(), IDENT);
        assert_eq!(
            source.current_range(),
            TextRange::at(TextSize::from(4), TextSize::from(1))
        );
        // The skipped token, and the space after it as leading trivia of the next token.
        assert_eq!(trivia(&source), [
            (TriviaPieceKind::Skipped, false),
            (TriviaPieceKind::Whitespace, false),
        ]);
    }

    #[test]
    fn the_end_of_the_file_is_never_skipped() {
        let mut source = TokenSource::from_str("");

        assert_eq!(source.current(), EOF);
        source.bump();
        assert_eq!(source.current(), EOF);
        source.skip_as_trivia();
        assert_eq!(source.current(), EOF);
        assert!(trivia(&source).is_empty());
    }

    #[test]
    fn looks_ahead_over_the_trivia() {
        let mut source = TokenSource::from_str("let x\n = 1");

        assert_eq!(source.nth(0), LET_KW);
        assert_eq!(source.nth(1), IDENT);
        assert_eq!(
            source.nth_range(1),
            Some(TextRange::at(TextSize::from(4), TextSize::from(1)))
        );
        assert!(!source.has_nth_preceding_line_break(1));

        assert_eq!(source.nth(2), EQ);
        assert!(source.has_nth_preceding_line_break(2));
        assert!(source.has_nth_preceding_whitespace(2));

        // Looking ahead does not consume anything.
        assert_eq!(source.current(), LET_KW);
        source.bump();
        assert_eq!(source.current(), IDENT);
    }

    #[test]
    fn a_checkpoint_restores_the_trivia() {
        let mut source = TokenSource::from_str("let x = 1");
        let checkpoint = source.checkpoint();

        source.bump();
        source.bump();

        assert_eq!(source.current(), EQ);
        assert_eq!(trivia(&source).len(), 2);

        source.rewind(checkpoint);

        assert_eq!(source.current(), LET_KW);
        assert!(trivia(&source).is_empty());

        source.bump();
        source.bump();

        assert_eq!(source.current(), EQ);
        assert_eq!(trivia(&source).len(), 2);
    }

    #[test]
    fn the_byte_order_mark_is_trivia() {
        let source = TokenSource::from_str("\u{feff}fun");

        assert_eq!(source.current(), FUN_KW);
        assert_eq!(trivia(&source), [(TriviaPieceKind::Skipped, false)]);
    }

    #[test]
    fn the_lexer_diagnostics_reach_the_parser() {
        let mut source = TokenSource::from_str("let $ = 1");

        source.bump();

        let (trivia_list, diagnostics) = source.finish();

        assert_eq!(trivia_list.len(), 1);
        assert!(trivia_list[0].kind().is_whitespace());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "unexpected character `$`");
    }
}
