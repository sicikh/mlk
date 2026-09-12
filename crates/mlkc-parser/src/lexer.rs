use logos::Logos;
use mlkc_rowan::{TextRange, TextSize, TriviaPieceKind};
use mlkc_syntax::*;

use crate::tree_sink::Trivia;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum TokenKind {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LCurly,
    #[token("}")]
    RCurly,
    #[token("[")]
    LBrack,
    #[token("]")]
    RBrack,
    #[regex(r"[a-zA-Z+\-*/<>=!?_$%&~^:|][a-zA-Z0-9+\-*/<>=!?_$%&~^:|]*")]
    Ident,
    #[regex(r"[0-9]+")]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[token("#t")]
    TrueKw,
    #[token("#f")]
    FalseKw,
    #[regex(r"[\r\n\f]+")]
    Newline,
    #[regex(r"[ \t]+")]
    Whitespace,
    #[regex(r";[^\n]*", allow_greedy = true)]
    Comment,
    #[logos(error)]
    Error,
}

impl From<TokenKind> for MlkSyntaxKind {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::LParen => L_PAREN,
            TokenKind::RParen => R_PAREN,
            TokenKind::LCurly => L_CURLY,
            TokenKind::RCurly => R_CURLY,
            TokenKind::LBrack => L_BRACK,
            TokenKind::RBrack => R_BRACK,
            TokenKind::Ident => IDENT,
            TokenKind::Number => NUMBER_LITERAL,
            TokenKind::String => STRING_LITERAL,
            TokenKind::TrueKw => BOOLEAN_LITERAL,
            TokenKind::FalseKw => BOOLEAN_LITERAL,
            TokenKind::Newline => NEWLINE,
            TokenKind::Whitespace => WHITESPACE,
            TokenKind::Comment => COMMENT,
            TokenKind::Error => BOGUS_EXPR,
        }
    }
}

impl TokenKind {
    pub fn trivia_kind(self) -> Option<TriviaPieceKind> {
        TriviaPieceKind::try_from(MlkSyntaxKind::from(self)).ok()
    }

    pub fn is_right_paren(self) -> bool {
        matches!(
            self,
            TokenKind::RParen | TokenKind::RCurly | TokenKind::RBrack
        )
    }

    pub fn is_left_paren(self) -> bool {
        matches!(
            self,
            TokenKind::RParen | TokenKind::RCurly | TokenKind::RBrack
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token {
    pub kind: MlkSyntaxKind,
    pub range: TextRange,
}

impl Token {
    pub fn new(kind: MlkSyntaxKind, range: TextRange) -> Self {
        Self { kind, range }
    }
}

pub fn lex(source: &str) -> (Vec<Token>, Vec<Trivia>) {
    let all_tokens = TokenKind::lexer(source).spanned().map(|(token, span)| {
        (
            token.unwrap_or(TokenKind::Error),
            TextRange::new(
                TextSize::from(span.start as u32),
                TextSize::from(span.end as u32),
            ),
        )
    });

    let mut tokens = Vec::new();
    let mut trivias = Vec::new();
    let mut trailing = false;

    for (token_kind, range) in all_tokens {
        match token_kind.trivia_kind() {
            Some(trivia_kind) => {
                if trivia_kind.is_newline() {
                    trailing = false;
                }
                trivias.push(Trivia::new(trivia_kind, range, trailing))
            },
            None => {
                trailing = true;
                tokens.push(Token::new(token_kind.into(), range))
            },
        }
    }

    (tokens, trivias)
}
