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
    DOT,
    COMMA,
    L_CURLY,
    R_CURLY,
    HASH,
    AT,
    L_PAREN,
    R_PAREN,
    L_BRACK,
    R_BRACK,
    COLON,
    COLON_COLON,
    SEMICOLON,
    EQ,
    ARROW,
    UNDERSCORE,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    EQ2,
    BANG_EQ,
    LT,
    GT,
    LT_EQ,
    GT_EQ,
    AND2,
    OR2,
    PIPE,
    AS_KW,
    ELIF_KW,
    ELSE_KW,
    FALSE_KW,
    FN_KW,
    FUN_KW,
    IF_KW,
    IN_KW,
    LET_KW,
    LOCAL_KW,
    MODULE_KW,
    PROJECT_KW,
    PUB_KW,
    THEN_KW,
    TRUE_KW,
    TYPE_KW,
    USE_KW,
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
    PATH_ROOT,
    PROJECT,
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
    USE_DECL,
    USE_ALIAS,
    FUN_RETURN_TYPE_ANNOTATION,
    FUN_BODY,
    PARAMETERS,
    PARAMETER_LIST,
    ANY_PARAMETER,
    PARAMETER,
    TYPE_ANNOTATION,
    EXPR,
    LITERAL,
    BOOL_LITERAL,
    PATH_EXPR,
    CALL_EXPR,
    UFCS_CALL,
    FIELD_EXPR,
    ARGUMENT_LIST,
    UNARY_EXPR,
    BIN_EXPR,
    PIPE_EXPR,
    PLACEHOLDER_EXPR,
    IF_EXPR,
    ELSE_BRANCH,
    IF_ARM_LIST,
    IF_ARM,
    LET_EXPR,
    LAMBDA_EXPR,
    LAMBDA_PARAMETER_LIST,
    LOCAL_EXPR,
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
    BOGUS_PARAMETER,
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
            DOT | COMMA
                | L_CURLY
                | R_CURLY
                | HASH
                | AT
                | L_PAREN
                | R_PAREN
                | L_BRACK
                | R_BRACK
                | COLON
                | COLON_COLON
                | SEMICOLON
                | EQ
                | ARROW
                | UNDERSCORE
                | PLUS
                | MINUS
                | STAR
                | SLASH
                | EQ2
                | BANG_EQ
                | LT
                | GT
                | LT_EQ
                | GT_EQ
                | AND2
                | OR2
                | PIPE
        )
    }
    pub const fn is_literal(self) -> bool {
        matches!(self, INT_LITERAL | STRING_LITERAL)
    }
    pub const fn is_list(self) -> bool {
        matches!(
            self,
            TYPE_ARG_LIST
                | MODULE_ITEM_LIST
                | ATTRIBUTE_LIST
                | PARAMETER_LIST
                | ARGUMENT_LIST
                | IF_ARM_LIST
                | LAMBDA_PARAMETER_LIST
        )
    }
    pub fn from_keyword(ident: &str) -> Option<Self> {
        let kw = match ident {
            "as" => AS_KW,
            "elif" => ELIF_KW,
            "else" => ELSE_KW,
            "false" => FALSE_KW,
            "fn" => FN_KW,
            "fun" => FUN_KW,
            "if" => IF_KW,
            "in" => IN_KW,
            "let" => LET_KW,
            "local" => LOCAL_KW,
            "module" => MODULE_KW,
            "project" => PROJECT_KW,
            "pub" => PUB_KW,
            "then" => THEN_KW,
            "true" => TRUE_KW,
            "type" => TYPE_KW,
            "use" => USE_KW,
            _ => return None,
        };
        Some(kw)
    }
    pub const fn to_string(&self) -> Option<&'static str> {
        let tok = match self {
            DOT => ".",
            COMMA => ",",
            L_CURLY => "{",
            R_CURLY => "}",
            HASH => "#",
            AT => "@",
            L_PAREN => "(",
            R_PAREN => ")",
            L_BRACK => "[",
            R_BRACK => "]",
            COLON => ":",
            COLON_COLON => "::",
            SEMICOLON => ";",
            EQ => "=",
            ARROW => "->",
            UNDERSCORE => "_",
            PLUS => "+",
            MINUS => "-",
            STAR => "*",
            SLASH => "/",
            EQ2 => "==",
            BANG_EQ => "!=",
            LT => "<",
            GT => ">",
            LT_EQ => "<=",
            GT_EQ => ">=",
            AND2 => "&&",
            OR2 => "||",
            PIPE => "|>",
            AS_KW => "as",
            ELIF_KW => "elif",
            ELSE_KW => "else",
            FALSE_KW => "false",
            FN_KW => "fn",
            FUN_KW => "fun",
            IF_KW => "if",
            IN_KW => "in",
            LET_KW => "let",
            LOCAL_KW => "local",
            MODULE_KW => "module",
            PROJECT_KW => "project",
            PUB_KW => "pub",
            THEN_KW => "then",
            TRUE_KW => "true",
            TYPE_KW => "type",
            USE_KW => "use",
            EOF => "",
            _ => return None,
        };
        Some(tok)
    }
}
#[doc = r" Utility macro for creating a SyntaxKind through simple macro syntax"]
#[macro_export]
macro_rules ! T { [.] => { $ crate :: SyntaxKind :: DOT } ; [,] => { $ crate :: SyntaxKind :: COMMA } ; ['{'] => { $ crate :: SyntaxKind :: L_CURLY } ; ['}'] => { $ crate :: SyntaxKind :: R_CURLY } ; [#] => { $ crate :: SyntaxKind :: HASH } ; [@] => { $ crate :: SyntaxKind :: AT } ; ['('] => { $ crate :: SyntaxKind :: L_PAREN } ; [')'] => { $ crate :: SyntaxKind :: R_PAREN } ; ['['] => { $ crate :: SyntaxKind :: L_BRACK } ; [']'] => { $ crate :: SyntaxKind :: R_BRACK } ; [:] => { $ crate :: SyntaxKind :: COLON } ; [::] => { $ crate :: SyntaxKind :: COLON_COLON } ; [;] => { $ crate :: SyntaxKind :: SEMICOLON } ; [=] => { $ crate :: SyntaxKind :: EQ } ; [->] => { $ crate :: SyntaxKind :: ARROW } ; ["_"] => { $ crate :: SyntaxKind :: UNDERSCORE } ; [+] => { $ crate :: SyntaxKind :: PLUS } ; [-] => { $ crate :: SyntaxKind :: MINUS } ; [*] => { $ crate :: SyntaxKind :: STAR } ; [/] => { $ crate :: SyntaxKind :: SLASH } ; [==] => { $ crate :: SyntaxKind :: EQ2 } ; [!=] => { $ crate :: SyntaxKind :: BANG_EQ } ; [<] => { $ crate :: SyntaxKind :: LT } ; [>] => { $ crate :: SyntaxKind :: GT } ; [<=] => { $ crate :: SyntaxKind :: LT_EQ } ; [>=] => { $ crate :: SyntaxKind :: GT_EQ } ; [&&] => { $ crate :: SyntaxKind :: AND2 } ; [||] => { $ crate :: SyntaxKind :: OR2 } ; [|>] => { $ crate :: SyntaxKind :: PIPE } ; [as] => { $ crate :: SyntaxKind :: AS_KW } ; [elif] => { $ crate :: SyntaxKind :: ELIF_KW } ; [else] => { $ crate :: SyntaxKind :: ELSE_KW } ; [false] => { $ crate :: SyntaxKind :: FALSE_KW } ; [fn] => { $ crate :: SyntaxKind :: FN_KW } ; [fun] => { $ crate :: SyntaxKind :: FUN_KW } ; [if] => { $ crate :: SyntaxKind :: IF_KW } ; [in] => { $ crate :: SyntaxKind :: IN_KW } ; [let] => { $ crate :: SyntaxKind :: LET_KW } ; [local] => { $ crate :: SyntaxKind :: LOCAL_KW } ; [module] => { $ crate :: SyntaxKind :: MODULE_KW } ; [project] => { $ crate :: SyntaxKind :: PROJECT_KW } ; [pub] => { $ crate :: SyntaxKind :: PUB_KW } ; [then] => { $ crate :: SyntaxKind :: THEN_KW } ; [true] => { $ crate :: SyntaxKind :: TRUE_KW } ; [type] => { $ crate :: SyntaxKind :: TYPE_KW } ; [use] => { $ crate :: SyntaxKind :: USE_KW } ; [ident] => { $ crate :: SyntaxKind :: IDENT } ; [EOF] => { $ crate :: SyntaxKind :: EOF } ; [UNICODE_BOM] => { $ crate :: SyntaxKind :: UNICODE_BOM } ; }
