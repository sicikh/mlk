#![allow(bad_style)]

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
pub enum MlkSyntaxKind {
    #[doc(hidden)]
    TOMBSTONE,
    /// Marks the end of the file. May have trivia attached
    EOF,
    WHITESPACE,
    NEWLINE,
    COMMENT,
    MULTILINE_COMMENT,
    ERROR_TOKEN,
    L_PAREN,
    R_PAREN,
    L_CURLY,
    R_CURLY,
    L_BRACK,
    R_BRACK,
    NUMBER_LITERAL,
    BOOLEAN_LITERAL,
    STRING_LITERAL,
    IDENT,
    NUMBER_LITERAL_ATOM,
    BOOLEAN_LITERAL_ATOM,
    STRING_LITERAL_ATOM,
    SYMBOL_ATOM,
    BOGUS_EXPR,
    EXPR_LIST,
    LIST_EXPR,
    ROOT_MODULE,
    #[doc(hidden)]
    __LAST,
}

use self::MlkSyntaxKind::*;

impl MlkSyntaxKind {
    pub const fn is_punct(self) -> bool {
        matches!(
            self,
            L_PAREN | R_PAREN | L_CURLY | R_CURLY | L_BRACK | R_BRACK
        )
    }

    pub const fn is_literal(self) -> bool {
        matches!(self, NUMBER_LITERAL | BOOLEAN_LITERAL | STRING_LITERAL)
    }

    pub const fn is_list(self) -> bool {
        matches!(self, EXPR_LIST)
    }

    pub const fn to_string(self) -> Option<&'static str> {
        let tok = match self {
            L_PAREN => "(",
            R_PAREN => ")",
            L_CURLY => "{",
            R_CURLY => "}",
            L_BRACK => "[",
            R_BRACK => "]",
            _ => return None,
        };
        Some(tok)
    }

    pub const fn rev_paren(&self) -> Option<Self> {
        let tok = match self {
            L_PAREN => R_PAREN,
            R_PAREN => L_PAREN,
            L_CURLY => R_CURLY,
            R_CURLY => L_CURLY,
            L_BRACK => R_BRACK,
            R_BRACK => L_BRACK,
            _ => return None,
        };
        Some(tok)
    }

    pub const fn is_left_paren(self) -> bool {
        matches!(self, L_PAREN | L_CURLY | L_BRACK)
    }

    pub const fn is_right_paren(self) -> bool {
        matches!(self, R_PAREN | R_CURLY | R_BRACK)
    }
}

/// Utility macro for creating a SyntaxKind through simple macro syntax
#[macro_export]
macro_rules! T {
    ['('] => { $crate::MlkSyntaxKind::L_PAREN };
    [')'] => { $crate::MlkSyntaxKind::R_PAREN };
    ['{'] => { $crate::MlkSyntaxKind::L_CURLY };
    ['}'] => { $crate::MlkSyntaxKind::R_CURLY };
    ['['] => { $crate::MlkSyntaxKind::L_BRACK };
    [']'] => { $crate::MlkSyntaxKind::R_BRACK };
    [ident] => { $crate::MlkSyntaxKind::IDENT };
    [EOF] => { $crate::MlkSyntaxKind::EOF };
}
