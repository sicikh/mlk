//! The lexer of the MLK language.
//!
//! Every token of MLK is a regular language, so the tokens are described once,
//! as [`logos`] patterns, and the DFA is generated from them. The rest of this
//! module is the adapter between that generated lexer and the parser core's
//! [`LexerTrait`].
//!
//! Two things are worth knowing about the contract:
//!
//! - [`LexerTrait::next_token`] returns *every* token, trivia included.
//!   Collecting the trivia and keeping it away from the parser is the job of the
//!   token source, not of the lexer.
//! - A lexical error is a token too: [`ERROR_TOKEN`]. It covers a single
//!   character, so the parser can recover close to where the mistake is.
//!
//! # Checkpoints
//!
//! The state of the lexer is a position in the source, not a borrowed DFA:
//! [`LexerTrait::rewind`] restores a checkpoint by moving the position back, and
//! the next call to [`LexerTrait::next_token`] runs the DFA from there. This is
//! why the lexer hands out [`LexerCheckpoint`]s cheaply.

use logos::Logos;
use mlkc_parser_core::{
    diagnostic::ParseDiagnostic,
    lexer::{Lexer as LexerTrait, LexerCheckpoint, LexerWithCheckpoint, TokenFlags},
};
use mlkc_rowan::SyntaxKind as SyntaxKindTrait;
use mlkc_syntax::{
    SyntaxKind::{self, *},
    TextRange, TextSize,
};

/// A token, as lexed: its kind and the range of the source it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    kind: SyntaxKind,
    range: TextRange,
}

impl Token {
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub fn range(&self) -> TextRange {
        self.range
    }
}

/// The tokens of MLK, as recognized by the generated lexer.
///
/// The variant order does not matter: the longest match wins, and a keyword wins
/// over an identifier of the same length because a literal token has a higher
/// priority than a regular expression.
///
/// Identifiers may contain `-` between identifier characters, so `println-int`
/// is a single name, while `x - 1` is a subtraction. The narrowest way to write
/// this down is what makes `x-1` a name as well.
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    #[regex(r"[ \t]+")]
    Whitespace,
    #[regex(r"\r\n|\n|\r")]
    Newline,
    #[regex(r"//[^\n\r]*", allow_greedy = true)]
    Comment,
    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    MultilineComment,

    #[token("as")]
    AsKw,
    #[token("fun")]
    FunKw,
    #[token("in")]
    InKw,
    #[token("let")]
    LetKw,
    #[token("module")]
    ModuleKw,
    #[token("pub")]
    PubKw,
    #[token("type")]
    TypeKw,
    #[token("use")]
    UseKw,

    #[regex(r"[0-9]+")]
    IntLiteral,
    // A string ends at the first quote that is not escaped: a backslash escapes whatever
    // follows it, so `\"` is a quote the literal holds. What an escape means is not read
    // here: the value of a literal is decoded where it is read, and the lexer is what decides
    // where the literal ends.
    #[regex(r#""(\\.|[^"\\\n\r])*""#)]
    StringLiteral,
    // A string that is never closed. The run is one broken token rather than one for every
    // character of it, and a backslash at the end of it is part of the run.
    #[regex(r#""(\\.|[^"\\\n\r])*\\?"#)]
    UnterminatedString,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*(-[a-zA-Z0-9_]+)*")]
    Ident,

    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("{")]
    LCurly,
    #[token("}")]
    RCurly,
    #[token("@")]
    At,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBrack,
    #[token("]")]
    RBrack,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("=")]
    Eq,
    #[token("->")]
    Arrow,
    // `_` is a wildcard pattern, not a name: it wins over an identifier of the
    // same length (`_`), while a longer identifier (`_x`) is longer and wins.
    #[token("_", priority = 3)]
    Underscore,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("==")]
    Eq2,
    #[token("!=")]
    BangEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    And2,
    #[token("||")]
    Or2,
}

impl Tok {
    fn kind(self) -> SyntaxKind {
        match self {
            Self::Whitespace => WHITESPACE,
            Self::Newline => NEWLINE,
            Self::Comment => COMMENT,
            Self::MultilineComment => MULTILINE_COMMENT,
            Self::AsKw => AS_KW,
            Self::FunKw => FUN_KW,
            Self::InKw => IN_KW,
            Self::LetKw => LET_KW,
            Self::ModuleKw => MODULE_KW,
            Self::PubKw => PUB_KW,
            Self::TypeKw => TYPE_KW,
            Self::UseKw => USE_KW,
            Self::IntLiteral => INT_LITERAL,
            Self::StringLiteral => STRING_LITERAL,
            // Reported as a broken token, with a diagnostic of its own:
            // see [Lexer::lex_token].
            Self::UnterminatedString => ERROR_TOKEN,
            Self::Ident => IDENT,
            Self::Dot => DOT,
            Self::Comma => COMMA,
            Self::LCurly => L_CURLY,
            Self::RCurly => R_CURLY,
            Self::At => AT,
            Self::LParen => L_PAREN,
            Self::RParen => R_PAREN,
            Self::LBrack => L_BRACK,
            Self::RBrack => R_BRACK,
            Self::Colon => COLON,
            Self::Semicolon => SEMICOLON,
            Self::Eq => EQ,
            Self::Arrow => ARROW,
            Self::Underscore => UNDERSCORE,
            Self::Plus => PLUS,
            Self::Minus => MINUS,
            Self::Star => STAR,
            Self::Slash => SLASH,
            Self::Eq2 => EQ2,
            Self::BangEq => BANG_EQ,
            Self::Lt => LT,
            Self::Gt => GT,
            Self::LtEq => LT_EQ,
            Self::GtEq => GT_EQ,
            Self::And2 => AND2,
            Self::Or2 => OR2,
        }
    }
}

pub(crate) struct Lexer<'src> {
    source: &'src str,
    /// The byte offset of the end of the current token, and the start of the next one.
    position: usize,
    /// The byte offset of the start of the current token.
    current_start: usize,
    current_kind: SyntaxKind,
    /// What the trivia before the current token was.
    current_flags: TokenFlags,
    /// Set while the lexer walks over trivia, to be reported on the token after it.
    after_line_break: bool,
    after_whitespace: bool,
    /// The length of the byte order mark, if the source starts with one.
    unicode_bom_length: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'src> Lexer<'src> {
    pub fn from_str(source: &'src str) -> Self {
        Self {
            source,
            position: 0,
            current_start: 0,
            // There is no token yet: the first one is lexed by `next_token`.
            current_kind: TOMBSTONE,
            current_flags: TokenFlags::empty(),
            after_line_break: false,
            after_whitespace: false,
            unicode_bom_length: if source.starts_with('\u{feff}') { 3 } else { 0 },
            diagnostics: Vec::new(),
        }
    }

    /// Lexes the next token and returns it together with its range,
    /// or `None` at the end of the file.
    ///
    /// This is a convenience wrapper over [`LexerTrait::next_token`] for the
    /// callers that do not track the current token: tests, tools, the driver.
    /// The parser reads tokens through the token source instead.
    pub(crate) fn next_token(&mut self) -> Option<Token> {
        let kind = LexerTrait::next_token(self, ());

        if kind == EOF {
            None
        } else {
            Some(Token {
                kind,
                range: self.current_range(),
            })
        }
    }

    /// Lexes one token at [`self.position`](Lexer::position) and moves the
    /// position past it.
    fn lex_token(&mut self) -> SyntaxKind {
        let source = self.source;

        // The byte order mark is a token of its own, so that it does not end up
        // inside the first token and is still part of the tree.
        if self.position == 0 && self.unicode_bom_length > 0 {
            self.position = self.unicode_bom_length;
            return UNICODE_BOM;
        }

        let rest = &source[self.position..];

        if rest.is_empty() {
            return EOF;
        }

        let mut lexer = Tok::lexer(rest);

        match lexer.next() {
            Some(Ok(Tok::UnterminatedString)) => {
                // The whole run is one broken token: the parser should see one
                // mistake, not one for every character of the string.
                let range = self.range_of(lexer.span().end);
                self.position += lexer.span().end;
                self.diagnostics
                    .push(ParseDiagnostic::new("unterminated string literal", range));
                ERROR_TOKEN
            },
            Some(Ok(token)) => {
                self.position += lexer.span().end;
                token.kind()
            },
            Some(Err(_)) => {
                // One character per error token: the parser recovers closer to
                // the mistake, and the diagnostics name the character that is
                // not part of the language.
                let character = rest.chars().next().expect("the source is not empty");
                let length = character.len_utf8();
                let range = self.range_of(length);
                self.diagnostics.push(ParseDiagnostic::new(
                    format!("unexpected character `{character}`"),
                    range,
                ));
                self.position += length;
                ERROR_TOKEN
            },
            None => EOF,
        }
    }

    /// The range of the `length` bytes that start at the current position.
    fn range_of(&self, length: usize) -> TextRange {
        TextRange::at(
            self.text_position(),
            TextSize::try_from(length).expect("Input to be smaller than 4 GB"),
        )
    }

    /// The flags of the token that starts after the trivia the lexer walked over.
    fn take_flags(&mut self) -> TokenFlags {
        let mut flags = TokenFlags::empty();

        if self.after_line_break {
            flags |= TokenFlags::PRECEDING_LINE_BREAK;
            self.after_line_break = false;
        }

        if self.after_whitespace {
            flags |= TokenFlags::PRECEDING_WHITESPACE;
            self.after_whitespace = false;
        }

        flags
    }

    /// Remembers the trivia the lexer just walked over: the flags of the next
    /// non-trivia token describe it.
    ///
    /// A comment separates nothing: it can stand anywhere whitespace can.
    fn remember_trivia(&mut self, kind: SyntaxKind) {
        match kind {
            WHITESPACE => self.after_whitespace = true,
            NEWLINE => self.after_line_break = true,
            _ => {},
        }
    }
}

impl<'src> LexerTrait<'src> for Lexer<'src> {
    const NEWLINE: Self::Kind = NEWLINE;

    const WHITESPACE: Self::Kind = WHITESPACE;

    type Kind = SyntaxKind;

    type LexContext = ();

    type ReLexContext = ();

    fn source(&self) -> &'src str {
        self.source
    }

    fn current(&self) -> Self::Kind {
        self.current_kind
    }

    fn current_start(&self) -> TextSize {
        TextSize::try_from(self.current_start).expect("Input to be smaller than 4 GB")
    }

    fn next_token(&mut self, _context: Self::LexContext) -> Self::Kind {
        self.current_start = self.position;

        let kind = self.lex_token();
        self.current_kind = kind;

        if kind.is_trivia() {
            // The trivia itself is not a token for the parser: remember it, so
            // that the flags of the next token describe what was skipped.
            self.remember_trivia(kind);
            self.current_flags = TokenFlags::empty();
        } else {
            self.current_flags = self.take_flags();
        }

        kind
    }

    fn has_preceding_line_break(&self) -> bool {
        self.current_flags.has_preceding_line_break()
    }

    fn has_preceding_whitespace(&self) -> bool {
        self.current_flags.has_preceding_whitespace()
    }

    fn has_unicode_escape(&self) -> bool {
        // Identifiers of MLK are plain UTF-8: there is nothing to escape yet.
        false
    }

    fn current_flags(&self) -> TokenFlags {
        self.current_flags
    }

    fn rewind(&mut self, checkpoint: LexerCheckpoint<Self::Kind>) {
        self.position = checkpoint.position.into();
        self.current_start = checkpoint.current_start.into();
        self.current_kind = checkpoint.current_kind;
        self.current_flags = checkpoint.current_flags;
        self.after_line_break = checkpoint.after_line_break;
        self.after_whitespace = checkpoint.after_whitespace;
        self.unicode_bom_length = checkpoint.unicode_bom_length;
        self.diagnostics
            .truncate(checkpoint.diagnostics_pos as usize);
    }

    fn finish(self) -> Vec<ParseDiagnostic> {
        self.diagnostics
    }

    fn position(&self) -> usize {
        self.position
    }

    fn push_diagnostic(&mut self, diagnostic: ParseDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn advance(&mut self, n: usize) {
        self.position += n;
    }

    fn advance_char_unchecked(&mut self) {
        let length = self.source[self.position..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);
        self.advance(length);
    }
}

impl<'src> LexerWithCheckpoint<'src> for Lexer<'src> {
    fn checkpoint(&self) -> LexerCheckpoint<Self::Kind> {
        LexerCheckpoint {
            position: self.text_position(),
            current_start: self.current_start(),
            current_kind: self.current_kind,
            current_flags: self.current_flags,
            after_line_break: self.after_line_break,
            after_whitespace: self.after_whitespace,
            unicode_bom_length: self.unicode_bom_length,
            diagnostics_pos: self.diagnostics.len() as u32,
        }
    }
}

#[cfg(test)]
mod tests;
