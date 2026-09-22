#[macro_use]
mod generated;
mod syntax_node;

use mlkc_rowan::{AstNode, RawSyntaxKind, SyntaxKind as SyntaxKindTrait, TokenText};
pub use mlkc_rowan::{TextLen, TextRange, TextSize, TokenAtOffset, TriviaPieceKind, WalkEvent};

pub use crate::{
    generated::{SyntaxKind::*, *},
    syntax_node::*,
};

impl From<u16> for SyntaxKind {
    fn from(d: u16) -> Self {
        assert!(d <= (__LAST as u16));
        unsafe { std::mem::transmute::<u16, Self>(d) }
    }
}

impl From<SyntaxKind> for u16 {
    fn from(kind: SyntaxKind) -> Self {
        kind as Self
    }
}

impl SyntaxKindTrait for SyntaxKind {
    const EOF: Self = EOF;
    const TOMBSTONE: Self = TOMBSTONE;

    fn is_bogus(&self) -> bool {
        matches!(
            self,
            BOGUS | BOGUS_DECL | BOGUS_EXPR | BOGUS_PAT | BOGUS_TYPE
        )
    }

    fn to_bogus(&self) -> Self {
        match self {
            kind if Expr::can_cast(*kind) => BOGUS_EXPR,
            kind if Pat::can_cast(*kind) => BOGUS_PAT,
            kind if Type::can_cast(*kind) => BOGUS_TYPE,
            kind if ModuleItem::can_cast(*kind) => BOGUS_DECL,
            _ => BOGUS,
        }
    }

    fn to_raw(&self) -> RawSyntaxKind {
        RawSyntaxKind(*self as u16)
    }

    fn from_raw(raw: RawSyntaxKind) -> Self {
        Self::from(raw.0)
    }

    fn is_root(&self) -> bool {
        matches!(self, MODULE_ROOT)
    }

    fn is_list(&self) -> bool {
        Self::is_list(*self)
    }

    fn is_trivia(self) -> bool {
        matches!(self, NEWLINE | WHITESPACE | COMMENT | MULTILINE_COMMENT)
    }

    fn to_string(&self) -> Option<&'static str> {
        Self::to_string(self)
    }
}

impl TryFrom<SyntaxKind> for TriviaPieceKind {
    type Error = ();

    fn try_from(value: SyntaxKind) -> Result<Self, Self::Error> {
        if value.is_trivia() {
            match value {
                NEWLINE => Ok(Self::Newline),
                WHITESPACE => Ok(Self::Whitespace),
                COMMENT => Ok(Self::SingleLineComment),
                MULTILINE_COMMENT => Ok(Self::MultiLineComment),
                _ => unreachable!("Not Trivia"),
            }
        } else {
            Err(())
        }
    }
}

pub fn inner_string_text(token: &SyntaxToken) -> TokenText {
    let mut text = token.token_text_trimmed();

    if matches!(token.kind(), STRING_LITERAL) {
        let range = TextRange::new(1.into(), text.len() - TextSize::from(1));
        text = text.slice(range);
    }

    text
}
