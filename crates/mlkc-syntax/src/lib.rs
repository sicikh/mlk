#[macro_use]
mod kind;
mod factory;
mod nodes;
mod syntax_node;

pub use kind::*;
use mlkc_rowan::{
    AstNode, RawSyntaxKind, SyntaxKind, TextRange, TextSize, TokenText, TreeBuilder,
    TriviaPieceKind,
};
pub use nodes::*;
pub use syntax_node::*;

pub use crate::{MlkSyntaxKind::*, factory::MlkSyntaxFactory};

impl From<u16> for MlkSyntaxKind {
    fn from(d: u16) -> MlkSyntaxKind {
        assert!(d <= (__LAST as u16));
        unsafe { std::mem::transmute::<u16, Self>(d) }
    }
}

impl SyntaxKind for MlkSyntaxKind {
    const EOF: Self = EOF;
    const TOMBSTONE: Self = TOMBSTONE;

    fn is_bogus(&self) -> bool {
        matches!(self, BOGUS_EXPR)
    }

    fn to_bogus(&self) -> Self {
        BOGUS_EXPR
    }

    fn to_raw(&self) -> RawSyntaxKind {
        RawSyntaxKind(*self as u16)
    }

    fn from_raw(raw: RawSyntaxKind) -> Self {
        Self::from(raw.0)
    }

    fn is_root(&self) -> bool {
        RootModule::can_cast(*self)
    }

    fn is_list(&self) -> bool {
        Self::is_list(*self)
    }

    fn is_trivia(self) -> bool {
        matches!(self, NEWLINE | WHITESPACE | COMMENT | MULTILINE_COMMENT)
    }

    fn to_string(&self) -> Option<&'static str> {
        Self::to_string(*self)
    }
}
impl TryFrom<MlkSyntaxKind> for TriviaPieceKind {
    type Error = ();

    fn try_from(value: MlkSyntaxKind) -> Result<Self, Self::Error> {
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

pub type MlkSyntaxTreeBuilder = TreeBuilder<'static, MlkLanguage, MlkSyntaxFactory>;
