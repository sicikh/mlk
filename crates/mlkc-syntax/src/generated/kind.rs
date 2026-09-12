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
    ARROW,
    #[doc(hidden)]
    __LAST,
}
use self::MlkSyntaxKind::*;
impl MlkSyntaxKind {
    pub const fn is_punct(self) -> bool {
        matches!(self, ARROW)
    }
    pub const fn is_literal(self) -> bool {
        matches!(self,)
    }
    pub const fn is_list(self) -> bool {
        matches!(self,)
    }
    pub fn from_keyword(_ident: &str) -> Option<Self> {
        None
    }
    pub const fn to_string(&self) -> Option<&'static str> {
        let tok = match self {
            ARROW => "->",
            EOF => "",
            MLK_STRING_LITERAL => "string literal",
            _ => return None,
        };
        Some(tok)
    }
}
#[doc = r" Utility macro for creating a SyntaxKind through simple macro syntax"]
#[macro_export]
macro_rules ! T { [->] => { $ crate :: MlkSyntaxKind :: ARROW } ; [ident] => { $ crate :: MlkSyntaxKind :: IDENT } ; [EOF] => { $ crate :: MlkSyntaxKind :: EOF } ; [UNICODE_BOM] => { $ crate :: MlkSyntaxKind :: UNICODE_BOM } ; [#] => { $ crate :: MlkSyntaxKind :: HASH } ; }
