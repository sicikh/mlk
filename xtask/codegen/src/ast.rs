//! Generate SyntaxKind definitions as well as typed AST definitions for nodes and tokens.
//! This is derived from biomejs/xtask/codegen

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Write,
    str::FromStr,
    vec,
};

use mlkc_string_case::Case;
use mlkc_ungrammar::{Grammar, Rule, Token};
use xtask_glue::{Result, project_root};

use super::{
    Mode,
    mlk_kinds_src::{AstSrc, Field},
};
use crate::{
    generate_macros::generate_macros,
    generate_node_factory::generate_node_factory,
    generate_nodes::generate_nodes,
    generate_nodes_mut::generate_nodes_mut,
    generate_syntax_factory::generate_syntax_factory,
    generate_syntax_kinds::generate_syntax_kinds,
    language_kind::{ALL_LANGUAGE_KIND, LanguageKind},
    mlk_kinds_src::{AstEnumSrc, AstListSeparatorConfiguration, AstListSrc, AstNodeSrc, TokenKind},
    termcolorful::{Color, println_string_with_fg_color},
    update,
};

// these node won't generate any code
pub const SYNTAX_ELEMENT_TYPE: &str = "SyntaxElement";

pub fn generate_ast(mode: Mode, language_kind_list: Vec<String>) -> Result<()> {
    let codegen_language_kinds = if language_kind_list.is_empty() {
        ALL_LANGUAGE_KIND.clone().to_vec()
    } else {
        language_kind_list
            .iter()
            .filter_map(|kind| {
                match LanguageKind::from_str(kind) {
                    Ok(kind) => Some(kind),
                    Err(err) => {
                        println_string_with_fg_color(err, Color::Red);
                        None
                    },
                }
            })
            .collect::<Vec<_>>()
    };
    for kind in codegen_language_kinds {
        println_string_with_fg_color(
            format!("-------------------Generating Grammar for {kind}-------------------"),
            Color::Green,
        );
        let ast = load_ast(kind);
        generate_syntax(ast, &mode, kind)?;
    }

    Ok(())
}

pub(crate) fn load_ast(language: LanguageKind) -> AstSrc {
    let grammar_src = language.load_grammar();
    let grammar: Grammar = grammar_src.parse().unwrap();
    let mut ast: AstSrc = make_ast(&grammar);
    if language == LanguageKind::Mlk {
        check_unions(&ast.unions);
    }
    ast.sort();
    ast
}

pub(crate) fn generate_syntax(ast: AstSrc, mode: &Mode, language_kind: LanguageKind) -> Result<()> {
    let syntax_generated_path = project_root()
        .join("crates")
        .join("mlkc-syntax")
        .join("src/generated");
    let factory_generated_path = project_root()
        .join("crates")
        .join("mlkc-syntax-factory")
        .join("src/generated");

    let kind_src = language_kind.kinds();

    let ast_nodes_file = syntax_generated_path.join("nodes.rs");
    let contents = generate_nodes(&ast, language_kind)?;
    update(ast_nodes_file.as_path(), &contents, mode)?;

    let ast_nodes_mut_file = syntax_generated_path.join("nodes_mut.rs");
    let contents = generate_nodes_mut(&ast, language_kind)?;
    update(ast_nodes_mut_file.as_path(), &contents, mode)?;

    let syntax_kinds_file = syntax_generated_path.join("kind.rs");
    let contents = generate_syntax_kinds(kind_src, language_kind)?;
    update(syntax_kinds_file.as_path(), &contents, mode)?;

    let syntax_factory_file = factory_generated_path.join("syntax_factory.rs");
    let contents = generate_syntax_factory(&ast, language_kind)?;
    update(syntax_factory_file.as_path(), &contents, mode)?;

    let node_factory_file = factory_generated_path.join("node_factory.rs");
    let contents = generate_node_factory(&ast, language_kind)?;
    update(node_factory_file.as_path(), &contents, mode)?;

    let ast_macros_file = syntax_generated_path.join("macros.rs");
    let contents = generate_macros(&ast, language_kind)?;
    update(ast_macros_file.as_path(), &contents, mode)?;

    Ok(())
}

fn check_unions(unions: &[AstEnumSrc]) {
    // Setup a map to find the unions quickly
    let union_map: HashMap<_, _> = unions.iter().map(|en| (&en.name, en)).collect();

    // Iterate over all unions
    for union in unions {
        let mut stack_string = format!(
            "\n******** START ERROR STACK ********\nChecking {}, variants : {:?}",
            union.name, union.variants
        );
        let mut union_set: HashSet<_> = HashSet::from([&union.name]);
        let mut union_queue: VecDeque<_> = VecDeque::new();

        // Init queue for BFS
        union_queue.extend(&union.variants);

        // Loop over the queue getting the first variant
        while let Some(variant) = union_queue.pop_front() {
            if union_map.contains_key(variant) {
                // The variant is a compound variant
                // Get the struct from the map
                let current_union = union_map[variant];
                write!(
                    stack_string,
                    "\nSUB-ENUM CHECK : {}, variants : {:?}",
                    current_union.name, current_union.variants
                )
                .unwrap();
                // Try to insert the current variant into the set
                if union_set.insert(&current_union.name) {
                    // Add all variants into the BFS queue
                    union_queue.extend(&current_union.variants);
                } else {
                    // We either have a circular dependency or 2 variants referencing the same type
                    println!("{stack_string}");
                    panic!("Variant '{variant}' used twice or circular dependency");
                }
            } else {
                // The variant isn't another enum
                // stack_string.push_str(&format!());
                write!(stack_string, "\nBASE-VAR CHECK : {variant}").unwrap();
                if !union_set.insert(variant) {
                    // The variant already used
                    println!("{stack_string}");
                    panic!("Variant '{variant}' used twice");
                }
            }
        }
    }
}

pub(crate) fn append_css_property_value_implied_alternatives(variants: Vec<String>) -> Vec<String> {
    let mut cloned = variants.clone();
    if !cloned.iter().any(|v| v == "CssWideKeyword") {
        cloned.push(String::from("CssWideKeyword"));
    }
    if !cloned.iter().any(|v| v == "CssUnknownPropertyValue") {
        cloned.push(String::from("CssUnknownPropertyValue"));
    }
    if !cloned.iter().any(|v| v == "CssBogusPropertyValue") {
        cloned.push(String::from("CssBogusPropertyValue"));
    }
    cloned
}

fn make_ast(grammar: &Grammar) -> AstSrc {
    let mut ast = AstSrc::default();

    for node in grammar.iter() {
        let name = grammar[node].name.clone();
        if name == SYNTAX_ELEMENT_TYPE {
            continue;
        }

        let rule = &grammar[node].rule;

        match classify_node_rule(grammar, rule, &name) {
            NodeRuleClassification::Union(variants) => {
                // TODO: This is CSS-specific and would be better handled with a per-language
                // method for classifying or modifying rules before generation.
                let variants = if name.trim().starts_with("AnyCss")
                    && name.trim().ends_with("PropertyValue")
                {
                    append_css_property_value_implied_alternatives(variants)
                } else {
                    variants
                };

                ast.unions.push(AstEnumSrc { name, variants })
            },
            NodeRuleClassification::Node => {
                let mut fields = vec![];
                handle_rule(&mut fields, grammar, rule, None, false, false);
                let is_dynamic = fields.iter().any(|field| field.is_unordered());
                ast.nodes.push(AstNodeSrc {
                    name,
                    fields,
                    dynamic: is_dynamic,
                })
            },
            NodeRuleClassification::DynamicNode => {
                let mut fields = vec![];
                handle_rule(&mut fields, grammar, rule, None, false, true);
                ast.nodes.push(AstNodeSrc {
                    name,
                    fields,
                    dynamic: true,
                })
            },
            NodeRuleClassification::Bogus => ast.bogus.push(name),
            NodeRuleClassification::List {
                separator,
                element_name,
            } => {
                ast.push_list(name.as_str(), AstListSrc {
                    element_name,
                    separator,
                });
            },
        }
    }

    ast
}

/// Classification of a node rule.
/// Determined by matching the top level production of any node.
enum NodeRuleClassification {
    /// Union of the form `A = B | C`
    Union(Vec<String>),

    /// Regular node containing tokens or sub nodes of the form `A = B 'c'
    Node,

    /// Node containing tokens or sub nodes where at least some of the children
    /// can be unordered, such as the form `A = E '#' (B && C && D)?`. If any
    /// children of a node are unordered, the entire node becomes dynamically ordered
    DynamicNode,

    /// A bogus node of the form `A = SyntaxElement*`
    Bogus,

    /// A list node of the form `A = B*` or `A = (B (',' B)*)` or `A = (B (',' B)* ','?)`
    List {
        /// Name of the nodes stored in this list (`B` in the example above)
        element_name: String,

        /// [None] if this is a node list or [Some] if this is a separated list
        separator: Option<AstListSeparatorConfiguration>,
    },
}

fn classify_node_rule(grammar: &Grammar, rule: &Rule, _name: &str) -> NodeRuleClassification {
    match rule {
        // this is for enums
        Rule::Alt(alternatives) => {
            let mut all_alternatives = vec![];
            for alternative in alternatives {
                match alternative {
                    Rule::Node(it) => all_alternatives.push(grammar[*it].name.clone()),
                    Rule::Token(it) if grammar[*it].name == ";" => (),
                    _ => return NodeRuleClassification::Node,
                }
            }
            NodeRuleClassification::Union(all_alternatives)
        },
        // A*
        Rule::Rep(rule) => {
            let element_type = match rule.as_ref() {
                Rule::Node(node) => &grammar[*node].name,
                _ => {
                    panic!("Lists should only be over node types");
                },
            };

            if element_type == SYNTAX_ELEMENT_TYPE {
                NodeRuleClassification::Bogus
            } else {
                NodeRuleClassification::List {
                    separator: None,
                    element_name: element_type.clone(),
                }
            }
        },
        Rule::Seq(rules) => {
            // (T (',' T)* ','?)
            // (T (',' T)*)
            if let Some(comma_list) = handle_comma_list(grammar, rules.as_slice()) {
                NodeRuleClassification::List {
                    separator: Some(AstListSeparatorConfiguration {
                        allow_trailing: comma_list.trailing_separator,
                        separator_token: comma_list.separator_name.to_string(),
                    }),
                    element_name: comma_list.node_name.to_string(),
                }
            } else {
                NodeRuleClassification::Node
            }
        },
        Rule::UnorderedAll(_) | Rule::UnorderedSome(_) => NodeRuleClassification::DynamicNode,
        _ => NodeRuleClassification::Node,
    }
}

fn clean_token_name(grammar: &Grammar, token: &Token) -> String {
    let mut name = grammar[*token].name.clone();

    // These tokens, when parsed to proc_macro2::TokenStream, generates a stream of bytes
    // that can't be recognized by [quote].
    // Hence, they need to be decorated with single quotes.
    if "[]{}()`".contains(&name) {
        name = format!("'{name}'");
    }
    name
}

fn handle_rule(
    fields: &mut Vec<Field>,
    grammar: &Grammar,
    rule: &Rule,
    label: Option<&str>,
    optional: bool,
    unordered: bool,
) {
    match rule {
        Rule::Labeled { label, rule } => {
            // Some methods need to be manually implemented because they need some custom logic;
            // we use the prefix "manual__" to exclude labelled nodes.

            if handle_tokens_in_unions(fields, grammar, rule, label, optional, unordered) {
                return;
            }

            handle_rule(fields, grammar, rule, Some(label), optional, unordered)
        },
        Rule::Node(node) => {
            let ty = grammar[*node].name.clone();
            let name = label.map_or_else(|| Case::Snake.convert(&ty), String::from);
            let field = Field::Node {
                name,
                ty,
                optional,
                unordered,
            };
            fields.push(field);
        },
        Rule::Token(token) => {
            let name = clean_token_name(grammar, token);

            if name == "''" {
                // array hole
                return;
            }

            let field = Field::Token {
                name: label.map_or_else(|| name.clone(), String::from),
                kind: TokenKind::Single(name),
                optional,
                unordered,
            };
            fields.push(field);
        },

        Rule::Rep(rule) => {
            panic!("Create a list node for *many* children {label:?} {rule:?}");
        },
        Rule::Opt(rule) => {
            handle_rule(fields, grammar, rule, label, true, false);
        },
        Rule::Alt(rules) => {
            // Alts must be required. We don't support alternated rules nested
            // within an Opt, like `(A | B)?`. For those, make a new Rule.
            if optional {
                panic!(
                    "Alternates cannot be nested within an optional Rule. Use a new Node to contain the alternate {label:?}"
                );
            }
            for rule in rules {
                handle_rule(fields, grammar, rule, label, false, false);
            }
        },
        Rule::Seq(rules) => {
            for rule in rules {
                // Sequences can be optional if they are wrapped by an Opt rule, so
                // it is inherited
                handle_rule(fields, grammar, rule, label, optional, false);
            }
        },
        Rule::UnorderedAll(rules) => {
            for rule in rules {
                // UnorderedAll only implies each contained rule is unordered, while
                // optionality is inherited from the parent.
                handle_rule(fields, grammar, rule, label, optional, true);
            }
        },
        Rule::UnorderedSome(rules) => {
            for rule in rules {
                // UnorderedSome implies each contained rule is unordered _and_ optional.
                handle_rule(fields, grammar, rule, label, true, true);
            }
        },
    };
}

#[derive(Debug)]
struct CommaList<'a> {
    node_name: &'a str,
    separator_name: &'a str,
    trailing_separator: bool,
}

// (T (',' T)* ','?)
// (T (',' T)*)
fn handle_comma_list<'a>(grammar: &'a Grammar, rules: &[Rule]) -> Option<CommaList<'a>> {
    // Does it match (T * ',')?
    let (node, repeat, trailing_separator) = match rules {
        [
            Rule::Node(node),
            Rule::Rep(repeat),
            Rule::Opt(trailing_separator),
        ] => (node, repeat, Some(trailing_separator)),
        [Rule::Node(node), Rule::Rep(repeat)] => (node, repeat, None),
        _ => return None,
    };

    // Is the repeat a ()*?
    let repeat = match &**repeat {
        Rule::Seq(it) => it,
        _ => return None,
    };

    // Does the repeat match (token)
    let comma = match repeat.as_slice() {
        [comma, Rule::Node(n)] => {
            let separator_matches_trailing = if let Some(trailing) = trailing_separator {
                &**trailing == comma
            } else {
                true
            };

            if n != node || !separator_matches_trailing {
                return None;
            }

            comma
        },
        _ => return None,
    };

    let separator_name = match comma {
        Rule::Token(token) => &grammar[*token].name,
        _ => panic!("The separator in rule {rules:?} must be a token"),
    };

    Some(CommaList {
        node_name: &grammar[*node].name,
        trailing_separator: trailing_separator.is_some(),
        separator_name,
    })
}

// handle cases like:  `op: ('-' | '+' | '*')`
fn handle_tokens_in_unions(
    fields: &mut Vec<Field>,
    grammar: &Grammar,
    rule: &Rule,
    label: &str,
    optional: bool,
    unordered: bool,
) -> bool {
    let (rule, optional) = match rule {
        Rule::Opt(rule) => (&**rule, true),
        _ => (rule, optional),
    };

    let rule = match rule {
        Rule::Alt(rule) => rule,
        _ => return false,
    };

    let mut token_kinds = vec![];
    for rule in rule.iter() {
        match rule {
            Rule::Token(token) => token_kinds.push(clean_token_name(grammar, token)),
            _ => return false,
        }
    }

    let field = Field::Token {
        name: label.to_string(),
        kind: TokenKind::Many(token_kinds),
        optional,
        unordered,
    };
    fields.push(field);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ast_src(src: &str) -> AstSrc {
        make_ast(
            &src.parse::<Grammar>()
                .expect("the grammar literal to parse"),
        )
    }

    fn node_named<'a>(ast: &'a AstSrc, name: &str) -> &'a AstNodeSrc {
        ast.nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("expected a node named `{name}`"))
    }

    #[test]
    fn a_union_rule_is_read_as_its_node_variants() {
        let ast = ast_src("Choice = First | Second\nFirst = 'a'\nSecond = 'b'");

        assert_eq!(ast.unions.len(), 1);
        assert_eq!(ast.unions[0].name, "Choice");
        assert_eq!(ast.unions[0].variants, ["First", "Second"]);
    }

    #[test]
    fn a_repetition_rule_is_read_as_a_list_of_its_element() {
        let ast = ast_src("Items = Item*\nItem = 'i'");

        assert!(ast.is_list("Items"));

        let list = &ast.lists["Items"];
        assert_eq!(list.element_name, "Item");
        assert!(list.separator.is_none());
    }

    #[test]
    fn a_comma_separated_rule_is_read_as_a_list_with_its_separator() {
        let ast = ast_src("Items = (Item (',' Item)*)\nItem = 'i'");

        let separator = ast.lists["Items"].separator.as_ref().expect("a separator");
        assert_eq!(separator.separator_token, ",");
        assert!(!separator.allow_trailing);
    }

    #[test]
    fn a_trailing_comma_in_the_rule_is_read_on_the_list() {
        let ast = ast_src("Items = (Item (',' Item)* ','?)\nItem = 'i'");

        let separator = ast.lists["Items"].separator.as_ref().expect("a separator");
        assert!(separator.allow_trailing);
    }

    #[test]
    fn a_syntax_element_repetition_is_read_as_a_bogus_node() {
        let ast = ast_src("Bogus = SyntaxElement*\nSyntaxElement = SyntaxElement");

        assert_eq!(ast.bogus, ["Bogus"]);
        assert!(ast.nodes.is_empty());
    }

    #[test]
    fn a_node_rule_takes_its_fields_from_labels_and_positions() {
        let ast = ast_src("Pair = left: Value ',' right: Value?\nValue = 'v'");

        let expected = [
            Field::Node {
                name: "left".into(),
                ty: "Value".into(),
                optional: false,
                unordered: false,
            },
            Field::Token {
                name: ",".into(),
                kind: TokenKind::Single(",".into()),
                optional: false,
                unordered: false,
            },
            Field::Node {
                name: "right".into(),
                ty: "Value".into(),
                optional: true,
                unordered: false,
            },
        ];

        assert_eq!(node_named(&ast, "Pair").fields, expected);
    }

    #[test]
    fn an_unordered_rule_is_read_as_a_dynamic_node() {
        let ast = ast_src(
            "Required = a: Value && b: Value\nOptional = a: Value || b: Value\nValue = 'v'",
        );

        let required = node_named(&ast, "Required");
        assert!(required.dynamic);
        assert!(
            required
                .fields
                .iter()
                .all(|field| field.is_unordered() && !field.is_optional())
        );

        let optional = node_named(&ast, "Optional");
        assert!(optional.dynamic);
        assert!(
            optional
                .fields
                .iter()
                .all(|field| field.is_unordered() && field.is_optional())
        );
    }

    #[test]
    fn a_labeled_alternation_of_tokens_is_read_as_one_field_of_many_kinds() {
        let ast = ast_src("Unary = op: ('-' | '+')\n");

        let expected = [Field::Token {
            name: "op".into(),
            kind: TokenKind::Many(vec!["-".into(), "+".into()]),
            optional: false,
            unordered: false,
        }];

        assert_eq!(node_named(&ast, "Unary").fields, expected);
    }

    #[test]
    #[should_panic(expected = "used twice")]
    fn a_repeated_union_variant_is_rejected() {
        check_unions(&[AstEnumSrc {
            name: "Choice".to_string(),
            variants: vec!["Value".to_string(), "Value".to_string()],
        }]);
    }

    #[test]
    #[should_panic(expected = "circular dependency")]
    fn a_cycle_between_unions_is_rejected() {
        check_unions(&[
            AstEnumSrc {
                name: "A".to_string(),
                variants: vec!["B".to_string()],
            },
            AstEnumSrc {
                name: "B".to_string(),
                variants: vec!["A".to_string()],
            },
        ]);
    }

    #[test]
    fn the_mlk_grammar_parses_into_the_expected_ast_source() {
        let ast = load_ast(LanguageKind::Mlk);

        assert!(ast.nodes.iter().any(|node| node.name == "IfExpr"));
        assert_eq!(ast.bogus, [
            "Bogus",
            "BogusDecl",
            "BogusExpr",
            "BogusParameter",
            "BogusPat",
            "BogusType"
        ]);

        let parameter_list = &ast.lists["ParameterList"];
        assert_eq!(parameter_list.element_name, "AnyParameter");

        let separator = parameter_list
            .separator
            .as_ref()
            .expect("a comma separator");
        assert_eq!(separator.separator_token, ",");
        assert!(!separator.allow_trailing);

        let type_arg_list = &ast.lists["TypeArgList"];
        let separator = type_arg_list.separator.as_ref().expect("a comma separator");
        assert!(separator.allow_trailing);
    }
}
