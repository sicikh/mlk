//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(bad_style, missing_docs, unreachable_pub)]
#[doc = r" The kind of syntax node, e.g. `IDENT`, `FUNCTION_KW`, or `FOR_STMT`."]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
pub enum SyntaxKind {
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
    L_BRACK,
    R_BRACK,
    COMMA,
    COLON,
    EQ,
    ARROW,
    UNDERSCORE,
    FUN_KW,
    IN_KW,
    LET_KW,
    TYPE_KW,
    INT_LITERAL,
    STRING_LITERAL,
    ERROR_TOKEN,
    IDENT,
    WHITESPACE,
    NEWLINE,
    COMMENT,
    MULTILINE_COMMENT,
    NAME,
    PATH,
    PATH_QUALIFIER,
    PATH_SEGMENT,
    TYPE_ARGS,
    TYPE_ARG_LIST,
    MODULE_ROOT,
    MODULE_PREAMBLE,
    MODULE_ITEM_LIST,
    MODULE_ITEM,
    ATTRIBUTE_LIST,
    ATTRIBUTE,
    TYPE_DECL,
    FUN_DECL,
    FUN_RETURN_TYPE_ANNOTATION,
    FUN_BODY,
    PARAMETERS,
    PARAMETER_LIST,
    PARAMETER,
    TYPE_ANNOTATION,
    EXPR,
    LITERAL,
    VAR_EXPR,
    CALL_EXPR,
    ARGUMENT_LIST,
    BIN_EXPR,
    LET_EXPR,
    PAREN_EXPR,
    TYPE,
    PATH_TYPE,
    INFER_TYPE,
    PAT,
    IDENT_PAT,
    WILDCARD_PAT,
    BOGUS,
    BOGUS_DECL,
    BOGUS_EXPR,
    BOGUS_PAT,
    BOGUS_TYPE,
    #[doc(hidden)]
    __LAST,
}
use self::SyntaxKind::*;
impl SyntaxKind {
    pub const fn is_punct(self) -> bool {
        matches!(
            self,
            AT | L_PAREN | R_PAREN | L_BRACK | R_BRACK | COMMA | COLON | EQ | ARROW | UNDERSCORE
        )
    }
    pub const fn is_literal(self) -> bool {
        matches!(self, INT_LITERAL | STRING_LITERAL)
    }
    pub const fn is_list(self) -> bool {
        matches!(
            self,
            TYPE_ARG_LIST | MODULE_ITEM_LIST | ATTRIBUTE_LIST | PARAMETER_LIST | ARGUMENT_LIST
        )
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
            L_BRACK => "]",
            R_BRACK => "[",
            COMMA => ",",
            COLON => ":",
            EQ => "=",
            ARROW => "->",
            UNDERSCORE => "_",
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
macro_rules ! T { [@] => { $ crate :: SyntaxKind :: AT } ; ['('] => { $ crate :: SyntaxKind :: L_PAREN } ; [')'] => { $ crate :: SyntaxKind :: R_PAREN } ; [']'] => { $ crate :: SyntaxKind :: L_BRACK } ; ['['] => { $ crate :: SyntaxKind :: R_BRACK } ; [,] => { $ crate :: SyntaxKind :: COMMA } ; [:] => { $ crate :: SyntaxKind :: COLON } ; [=] => { $ crate :: SyntaxKind :: EQ } ; [->] => { $ crate :: SyntaxKind :: ARROW } ; ["_"] => { $ crate :: SyntaxKind :: UNDERSCORE } ; [fun] => { $ crate :: SyntaxKind :: FUN_KW } ; [in] => { $ crate :: SyntaxKind :: IN_KW } ; [let] => { $ crate :: SyntaxKind :: LET_KW } ; [type] => { $ crate :: SyntaxKind :: TYPE_KW } ; [ident] => { $ crate :: SyntaxKind :: IDENT } ; [EOF] => { $ crate :: SyntaxKind :: EOF } ; [UNICODE_BOM] => { $ crate :: SyntaxKind :: UNICODE_BOM } ; }
