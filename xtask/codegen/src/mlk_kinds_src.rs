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
        ("#", "HASH"),
        ("@", "AT"),
        ("(", "L_PAREN"),
        (")", "R_PAREN"),
        ("[", "L_BRACK"),
        ("]", "R_BRACK"),
        (":", "COLON"),
        ("::", "COLON_COLON"),
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
        ("|>", "PIPE"),
    ],
    keywords: &[
        "as", "elif", "else", "false", "fn", "fun", "if", "in", "let", "local", "module",
        "project", "pub", "then", "true", "type", "use",
    ],
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
        "PATH_ROOT",
        "PROJECT",
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
        "BOOL_LITERAL",
        "PATH_EXPR",
        "CALL_EXPR",
        "UFCS_CALL",
        "FIELD_EXPR",
        "ARGUMENT_LIST",
        "UNARY_EXPR",
        "BIN_EXPR",
        "PIPE_EXPR",
        "PLACEHOLDER_EXPR",
        "IF_EXPR",
        "ELSE_BRANCH",
        "IF_ARM_LIST",
        "IF_ARM",
        "LET_EXPR",
        "LAMBDA_EXPR",
        "LAMBDA_PARAMETER_LIST",
        "LOCAL_EXPR",
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
                    ("|>", _) => "pipe",
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        AstEnumSrc, AstListSeparatorConfiguration, AstListSrc, AstNodeSrc, AstSrc, Field,
        MLK_KINDS_SRC, TokenKind,
    };
    use crate::language_kind::LanguageKind;

    fn token_field(name: &str) -> Field {
        Field::Token {
            name: name.to_string(),
            kind: TokenKind::Single(name.to_string()),
            optional: false,
            unordered: false,
        }
    }

    fn node_field(name: &str, ty: &str) -> Field {
        Field::Node {
            name: name.to_string(),
            ty: ty.to_string(),
            optional: false,
            unordered: false,
        }
    }

    fn ast_node(name: &str) -> AstNodeSrc {
        AstNodeSrc {
            name: name.to_string(),
            fields: vec![],
            dynamic: false,
        }
    }

    fn union(name: &str, variants: &[&str]) -> AstEnumSrc {
        AstEnumSrc {
            name: name.to_string(),
            variants: variants.iter().map(|variant| variant.to_string()).collect(),
        }
    }

    #[test]
    fn token_field_method_name_maps_punctuation_to_its_accessor_name() {
        for (token, expected) in [
            ("=", "eq_token"),
            (";", "semicolon_token"),
            ("->", "arrow_token"),
            ("::", "double_colon_token"),
            ("'('", "l_paren_token"),
            ("'['", "l_brack_token"),
        ] {
            let method_name = token_field(token).method_name(LanguageKind::Mlk);

            assert_eq!(method_name.to_string(), expected, "token `{token}`");
        }
    }

    #[test]
    fn token_field_method_name_marks_a_keyword_token() {
        let method_name = token_field("fn").method_name(LanguageKind::Mlk);

        assert_eq!(method_name.to_string(), "fn_token");
    }

    #[test]
    fn node_field_method_name_keeps_a_snake_case_name() {
        let method_name =
            node_field("path_qualifier", "PathQualifier").method_name(LanguageKind::Mlk);

        assert_eq!(method_name.to_string(), "path_qualifier");
    }

    #[test]
    fn node_field_method_name_calls_the_reserved_type_accessor_ty() {
        let method_name = node_field("type", "Type").method_name(LanguageKind::Mlk);

        assert_eq!(method_name.to_string(), "ty");
    }

    #[test]
    fn field_ty_is_syntax_token_for_tokens_and_the_stored_type_for_nodes() {
        assert_eq!(token_field("'('").ty().to_string(), "SyntaxToken");
        assert_eq!(node_field("name", "Name").ty().to_string(), "Name");
    }

    #[test]
    fn field_flags_report_optionality_and_order() {
        let field = Field::Token {
            name: "'+'".to_string(),
            kind: TokenKind::Single("'+'".to_string()),
            optional: true,
            unordered: true,
        };

        assert!(field.is_optional());
        assert!(field.is_unordered());

        let required = token_field("'+'");
        assert!(!required.is_optional());
        assert!(!required.is_unordered());
    }

    #[test]
    fn sort_orders_nodes_unions_variants_and_bogus_by_name() {
        let mut ast = AstSrc::default();
        ast.nodes.push(ast_node("Zeta"));
        ast.nodes.push(ast_node("Alpha"));
        ast.unions.push(union("Zeta", &["B", "A"]));
        ast.unions.push(union("Alpha", &["D", "C"]));
        ast.bogus.push("Zeta".to_string());
        ast.bogus.push("Alpha".to_string());

        ast.sort();

        let node_names: Vec<_> = ast.nodes.iter().map(|node| node.name.as_str()).collect();
        let union_names: Vec<_> = ast.unions.iter().map(|union| union.name.as_str()).collect();

        assert_eq!(node_names, ["Alpha", "Zeta"]);
        assert_eq!(union_names, ["Alpha", "Zeta"]);
        assert_eq!(ast.unions[0].variants, ["C", "D"]);
        assert_eq!(ast.unions[1].variants, ["A", "B"]);
        assert_eq!(ast.bogus, ["Alpha", "Zeta"]);
    }

    #[test]
    fn a_pushed_list_is_found_by_name_and_lists_iterate_in_name_order() {
        let mut ast = AstSrc::default();
        ast.push_list("Zeta", AstListSrc {
            element_name: "Z".to_string(),
            separator: None,
        });
        ast.push_list("Alpha", AstListSrc {
            element_name: "A".to_string(),
            separator: Some(AstListSeparatorConfiguration {
                separator_token: ",".to_string(),
                allow_trailing: true,
            }),
        });

        assert!(ast.is_list("Alpha"));
        assert!(!ast.is_list("Missing"));

        let names: Vec<_> = ast.lists().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["Alpha", "Zeta"]);
    }

    #[test]
    fn mlk_kinds_src_declares_each_kind_name_once() {
        let mut named: Vec<(&str, &str)> = Vec::new();
        named.extend(
            MLK_KINDS_SRC
                .punct
                .iter()
                .map(|(_, name)| ("punctuation", *name)),
        );
        named.extend(MLK_KINDS_SRC.keywords.iter().map(|name| ("keyword", *name)));
        named.extend(MLK_KINDS_SRC.literals.iter().map(|name| ("literal", *name)));
        named.extend(MLK_KINDS_SRC.tokens.iter().map(|name| ("token", *name)));
        named.extend(MLK_KINDS_SRC.nodes.iter().map(|name| ("node", *name)));

        let mut seen: HashMap<&str, &str> = HashMap::new();

        for (category, name) in named {
            if let Some(previous) = seen.insert(name, category) {
                panic!("`{name}` is declared both as a {previous} and as a {category}");
            }
        }
    }
}
