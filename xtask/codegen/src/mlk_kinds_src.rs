use std::collections::BTreeMap;

use quote::format_ident;

use crate::{
    kind_src::KindsSrc,
    language_kind::{LANGUAGE_PREFIXES, LanguageKind},
};

pub const MLK_KINDS_SRC: KindsSrc = KindsSrc {
    punct: &[
        (".", "DOT"),
        (",", "COMMA"),
        ("{", "L_CURLY"),
        ("}", "R_CURLY"),
        ("@", "AT"),
        ("(", "L_PAREN"),
        (")", "R_PAREN"),
        ("[", "L_BRACK"),
        ("]", "R_BRACK"),
        (":", "COLON"),
        (";", "SEMICOLON"),
        ("=", "EQ"),
        ("->", "ARROW"),
        ("_", "UNDERSCORE"),
        ("+", "PLUS"),
        ("-", "MINUS"),
        ("*", "STAR"),
        ("/", "SLASH"),
        ("==", "EQ2"),
        ("!=", "BANG_EQ"),
        ("<", "LT"),
        (">", "GT"),
        ("<=", "LT_EQ"),
        (">=", "GT_EQ"),
        ("&&", "AND2"),
        ("||", "OR2"),
    ],
    keywords: &["as", "fun", "in", "let", "module", "pub", "type", "use"],
    literals: &["INT_LITERAL", "STRING_LITERAL"],
    tokens: &[
        "ERROR_TOKEN",
        "IDENT",
        "WHITESPACE",
        "NEWLINE",
        "COMMENT",
        "MULTILINE_COMMENT",
    ],
    nodes: &[
        "NAME",
        "PATH",
        "PATH_QUALIFIER",
        "PATH_SEGMENT",
        "TYPE_ARGS",
        "TYPE_ARG_LIST",
        "MODULE_ROOT",
        "MODULE_PREAMBLE",
        "MODULE_ITEM_LIST",
        "MODULE_ITEM",
        "ATTRIBUTE_LIST",
        "ATTRIBUTE",
        "TYPE_DECL",
        "FUN_DECL",
        "USE_DECL",
        "USE_ALIAS",
        "FUN_RETURN_TYPE_ANNOTATION",
        "FUN_BODY",
        "PARAMETERS",
        "PARAMETER_LIST",
        "ANY_PARAMETER",
        "PARAMETER",
        "TYPE_ANNOTATION",
        "EXPR",
        "LITERAL",
        "PATH_EXPR",
        "CALL_EXPR",
        "ARGUMENT_LIST",
        "UNARY_EXPR",
        "BIN_EXPR",
        "LET_EXPR",
        "PAREN_EXPR",
        "TYPE",
        "PATH_TYPE",
        "INFER_TYPE",
        "PAT",
        "IDENT_PAT",
        "WILDCARD_PAT",
        // Bogus nodes
        "BOGUS",
        "BOGUS_DECL",
        "BOGUS_EXPR",
        "BOGUS_PARAMETER",
        "BOGUS_PAT",
        "BOGUS_TYPE",
    ],
};

#[derive(Default, Debug)]
pub struct AstSrc {
    pub nodes: Vec<AstNodeSrc>,
    pub unions: Vec<AstEnumSrc>,
    pub lists: BTreeMap<String, AstListSrc>,
    pub bogus: Vec<String>,
}

impl AstSrc {
    pub fn push_list(&mut self, name: &str, src: AstListSrc) {
        self.lists.insert(String::from(name), src);
    }

    pub fn lists(&self) -> std::collections::btree_map::Iter<'_, String, AstListSrc> {
        self.lists.iter()
    }

    pub fn is_list(&self, name: &str) -> bool {
        self.lists.contains_key(name)
    }

    /// Sorts all nodes, enums, etc. for a stable code gen result
    pub fn sort(&mut self) {
        // No need to sort lists, they're stored in a btree
        self.nodes.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        self.unions.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        self.bogus.sort_unstable();

        for union in self.unions.iter_mut() {
            union.variants.sort_unstable();
        }
    }
}

#[derive(Debug)]
pub struct AstListSrc {
    pub element_name: String,
    pub separator: Option<AstListSeparatorConfiguration>,
}

#[derive(Debug)]
pub struct AstListSeparatorConfiguration {
    /// Name of the separator token
    pub separator_token: String,
    /// Whatever the list allows a trailing comma or not
    pub allow_trailing: bool,
}

#[derive(Debug)]
pub struct AstNodeSrc {
    pub documentation: Vec<String>,
    pub name: String,
    // pub traits: Vec<String>,
    pub fields: Vec<Field>,
    /// Whether the fields of the node should be ordered dynamically using a
    /// slot map for accesses.
    pub dynamic: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TokenKind {
    Single(String),
    Many(Vec<String>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Field {
    Token {
        name: String,
        kind: TokenKind,
        optional: bool,
        unordered: bool,
    },
    Node {
        name: String,
        ty: String,
        optional: bool,
        unordered: bool,
    },
}

#[derive(Debug, Clone)]
pub struct AstEnumSrc {
    pub documentation: Vec<String>,
    pub name: String,
    // pub traits: Vec<String>,
    pub variants: Vec<String>,
}

impl Field {
    pub fn method_name(&self, language_kind: LanguageKind) -> proc_macro2::Ident {
        match self {
            Self::Token { name, .. } => {
                let name = match (name.as_str(), language_kind) {
                    (";", _) => "semicolon",
                    ("'{'", _) => "l_curly",
                    ("'}'", _) => "r_curly",
                    ("'('", _) => "l_paren",
                    ("')'", _) => "r_paren",
                    ("'['", _) => "l_brack",
                    ("']'", _) => "r_brack",
                    ("'`'", _) => "backtick",
                    ("<", _) => "l_angle",
                    (">", _) => "r_angle",
                    ("=", _) => "eq",
                    ("!", _) => "excl",
                    ("*", _) => "star",
                    ("&", _) => "amp",
                    (".", _) => "dot",
                    ("...", _) => "dotdotdot",
                    ("->", _) => "arrow",
                    ("=>", _) => "fat_arrow",
                    (":", _) => "colon",
                    ("::", _) => "double_colon",
                    ("?", _) => "question_mark",
                    ("+", _) => "plus",
                    ("-", _) => "minus",
                    ("#", _) => "hash",
                    ("@", _) => "at",
                    ("+=", _) => "add_assign",
                    ("-=", _) => "subtract_assign",
                    ("*=", _) => "times_assign",
                    ("%=", _) => "remainder_assign",
                    ("**=", _) => "exponent_assign",
                    (">>=", _) => "left_shift_assign",
                    ("<<=", _) => "right_shift_assign",
                    (">>>=", _) => "unsigned_right_shift_assign",
                    ("~", _) => "bitwise_not",
                    ("&=", _) => "bitwise_and_assign",
                    ("&&=", _) => "bitwise_logical_and_assign",
                    ("||=", _) => "bitwise_logical_or_assign",
                    ("??=", _) => "bitwise_nullish_coalescing_assign",
                    ("++", _) => "increment",
                    ("--", _) => "decrement",
                    ("<=", _) => "less_than_equal",
                    (">=", _) => "greater_than_equal",
                    ("==", _) => "equality",
                    ("===", _) => "strict_equality",
                    ("!=", _) => "inequality",
                    ("!==", _) => "strict_inequality",
                    ("/", _) => "slash",
                    ("%", _) => "remainder",
                    ("**", _) => "exponent",
                    ("<<", _) => "left_shift",
                    (">>", _) => "right_shift",
                    (">>>", _) => "unsigned_right_shift",
                    ("|", _) => "bitwise_or",
                    ("^", _) => "bitwise_xor",
                    ("??", _) => "nullish_coalescing",
                    ("||", _) => "logical_or",
                    ("&&", _) => "logical_and",
                    ("$=", _) => "suffix",
                    ("~=", _) => "whitespace_like",
                    (",", _) => "comma",
                    ("'", _) => "single_quote",
                    ("\"", _) => "double_quote",
                    ("_", _) => "underscore",
                    _ => name,
                };

                let kind_source = language_kind.kinds();

                // we need to replace "-" with "_" for the keywords
                // e.g. we have `color-profile` in css but it's an invalid ident in rust code
                if kind_source.keywords.contains(&name) {
                    format_ident!("{}_token", name.replace('-', "_").trim_matches('_'))
                } else {
                    format_ident!("{}_token", name)
                }
            },
            Self::Node { name, .. } => {
                let (prefix, tail) = name.split_once('_').unwrap_or(("", name));
                let final_name = if LANGUAGE_PREFIXES.contains(&prefix) {
                    tail
                } else {
                    name.as_str()
                };

                // this check here is to avoid emitting methods called "type()",
                // where "type" is a reserved word
                if final_name == "type" {
                    format_ident!("ty")
                } else {
                    format_ident!("{}", final_name)
                }
            },
        }
    }

    pub fn ty(&self) -> proc_macro2::Ident {
        match self {
            Self::Token { .. } => format_ident!("SyntaxToken"),
            Self::Node { ty, .. } => format_ident!("{}", ty),
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            Self::Node { optional, .. } => *optional,
            Self::Token { optional, .. } => *optional,
        }
    }

    pub fn is_unordered(&self) -> bool {
        match self {
            Self::Node { unordered, .. } => *unordered,
            Self::Token { unordered, .. } => *unordered,
        }
    }
}
