//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(bad_style, missing_docs, unreachable_pub)]
#[doc = r" The kind of syntax node, e.g. `IDENT`, `FUNCTION_KW`, or `FOR_STMT`."]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
pub enum MlkSyntaxKind {
    #[doc(hidden)]
    TOMBSTONE,
    #[doc = r" Marks the end of the file. May have trivia attached"]
    EOF,
    #[doc = r" Any Unicode BOM character that may be present at the start of"]
    #[doc = r" a file."]
    UNICODE_BOM,
    AT,
    L_PAREN,
    R_PAREN,
    COMMA,
    COLON,
    EQ,
    ARROW,
    FUN_KW,
    IN_KW,
    LET_KW,
    TYPE_KW,
    INT_LITERAL,
    ERROR_TOKEN,
    IDENT,
    OPERATOR,
    WHITESPACE,
    NEWLINE,
    COMMENT,
    MULTILINE_COMMENT,
    APP_EXPR,
    ATTRIBUTE,
    BIN_EXPR,
    FUN_BODY,
    FUN_DECL,
    LET_EXPR,
    LITERAL,
    MODULE_ROOT,
    NAME,
    NAMED_TY,
    PARAMETER,
    PARAMETERS,
    PAREN_EXPR,
    TYPE_DECL,
    VAR_EXPR,
    ATTRIBUTE_LIST,
    MODULE_ITEM_LIST,
    PARAMETER_LIST,
    BOGUS,
    BOGUS_DECL,
    BOGUS_EXPR,
    BOGUS_TY,
    #[doc(hidden)]
    __LAST,
}
use self::MlkSyntaxKind::*;
impl MlkSyntaxKind {
    pub const fn is_punct(self) -> bool {
        matches!(self, AT | L_PAREN | R_PAREN | COMMA | COLON | EQ | ARROW)
    }
    pub const fn is_literal(self) -> bool {
        matches!(self, INT_LITERAL)
    }
    pub const fn is_list(self) -> bool {
        matches!(self, ATTRIBUTE_LIST | MODULE_ITEM_LIST | PARAMETER_LIST)
    }
    pub fn from_keyword(ident: &str) -> Option<Self> {
        let kw = match ident {
            "fun" => FUN_KW,
            "in" => IN_KW,
            "let" => LET_KW,
            "type" => TYPE_KW,
            _ => return None,
        };
        Some(kw)
    }
    pub const fn to_string(&self) -> Option<&'static str> {
        let tok = match self {
            AT => "@",
            L_PAREN => "(",
            R_PAREN => ")",
            COMMA => ",",
            COLON => ":",
            EQ => "=",
            ARROW => "->",
            FUN_KW => "fun",
            IN_KW => "in",
            LET_KW => "let",
            TYPE_KW => "type",
            EOF => "",
            _ => return None,
        };
        Some(tok)
    }
}
#[doc = r" Utility macro for creating a SyntaxKind through simple macro syntax"]
#[macro_export]
macro_rules ! T { [@] => { $ crate :: MlkSyntaxKind :: AT } ; ['('] => { $ crate :: MlkSyntaxKind :: L_PAREN } ; [')'] => { $ crate :: MlkSyntaxKind :: R_PAREN } ; [,] => { $ crate :: MlkSyntaxKind :: COMMA } ; [:] => { $ crate :: MlkSyntaxKind :: COLON } ; [=] => { $ crate :: MlkSyntaxKind :: EQ } ; [->] => { $ crate :: MlkSyntaxKind :: ARROW } ; [fun] => { $ crate :: MlkSyntaxKind :: FUN_KW } ; [in] => { $ crate :: MlkSyntaxKind :: IN_KW } ; [let] => { $ crate :: MlkSyntaxKind :: LET_KW } ; [type] => { $ crate :: MlkSyntaxKind :: TYPE_KW } ; [ident] => { $ crate :: MlkSyntaxKind :: IDENT } ; [EOF] => { $ crate :: MlkSyntaxKind :: EOF } ; [UNICODE_BOM] => { $ crate :: MlkSyntaxKind :: UNICODE_BOM } ; }
