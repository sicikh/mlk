//! The module: the root of the tree and the items it holds.

use mlkc_parser_core::{
    parse_lists::{ParseNodeList, ParseSeparatedList},
    parse_recovery::{ParseRecoveryTokenSet, RecoveryResult},
    parsed_syntax::ParsedSyntax::Present,
    prelude::*,
};
use mlkc_syntax::{
    SyntaxKind::{self, *},
    T,
};

use crate::{
    parser::MlkParser,
    syntax::{
        attribute::{is_at_declaration, parse_attribute_list},
        auxiliary::parse_name,
        expr::parse_expr,
        parse_error::{
            expected_declaration, expected_expr, expected_name, expected_parameter,
            expected_parameters, expected_type,
        },
        ty::parse_type,
    },
};

/// The tokens a broken module item is recovered at: the start of the next declaration.
const MODULE_ITEM_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![T![@], T![fun], T![type]];

/// The tokens a broken parameter is recovered at: the end of the parameter it was written
/// in, or the end of the list it belongs to.
const PARAMETER_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![T![,], T![')']];

/// Parses the root of the tree: the byte order mark if the file has one, the items of the
/// module, and the end of the file.
pub(crate) fn parse_module_root(p: &mut MlkParser) -> CompletedMarker {
    let m = p.start();

    // The mark is a token of the tree rather than trivia: it is the first thing in the
    // file, and a file that has it is written differently from one that does not.
    p.eat(T![UNICODE_BOM]);

    ModuleItemListParse.parse_list(p);

    // The list stops at the end of the file, so this eats the token the list left behind:
    // the tree holds the end of the file, and the parser is the one that knows where it is.
    p.expect(T![EOF]);

    m.complete(p, MODULE_ROOT)
}

/// The items of a module.
struct ModuleItemListParse;

impl ParseNodeList for ModuleItemListParse {
    type Kind = SyntaxKind;
    type Parser<'source> = MlkParser<'source>;

    const LIST_KIND: SyntaxKind = MODULE_ITEM_LIST;

    fn parse_element(&mut self, p: &mut MlkParser) -> ParsedSyntax {
        parse_module_item(p)
    }

    fn is_at_list_end(&self, p: &mut MlkParser) -> bool {
        p.at(T![EOF])
    }

    fn recover(&mut self, p: &mut MlkParser, parsed_element: ParsedSyntax) -> RecoveryResult {
        parsed_element.or_recover_with_token_set(
            p,
            &ParseRecoveryTokenSet::new(BOGUS_DECL, MODULE_ITEM_RECOVERY_SET),
            expected_declaration,
        )
    }
}

/// Parses a single item of a module.
fn parse_module_item(p: &mut MlkParser) -> ParsedSyntax {
    if is_at_declaration(p, FUN_KW) {
        parse_fun_decl(p)
    } else if is_at_declaration(p, TYPE_KW) {
        parse_type_decl(p)
    } else {
        ParsedSyntax::Absent
    }
}

/// Parses a function declaration.
///
/// Everything but the name and the parameter list is optional: a declaration that is only
/// an interface — a builtin, an external function — has neither a return type nor a body.
// test mlk a_function_of_the_prelude_has_no_body
// @builtin
// fun size-of(value: Int): Int
//
// test mlk a_function_has_a_body_holding_an_expression
// fun main(): Int =
//     42
//
// test mlk a_function_may_take_no_arguments_and_return_nothing
// fun main() =
//     42
fn parse_fun_decl(p: &mut MlkParser) -> ParsedSyntax {
    if !is_at_declaration(p, FUN_KW) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    parse_attribute_list(p);
    p.expect(T![fun]);
    parse_name(p).or_add_diagnostic(p, expected_name);
    parse_parameters(p).or_add_diagnostic(p, expected_parameters);
    parse_fun_return_type_annotation(p).ok();
    parse_fun_body(p).ok();

    Present(m.complete(p, FUN_DECL))
}

/// Parses a type declaration.
// test mlk a_type_declaration_names_a_type
// @builtin
// type Unit
fn parse_type_decl(p: &mut MlkParser) -> ParsedSyntax {
    if !is_at_declaration(p, TYPE_KW) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    parse_attribute_list(p);
    p.expect(T![type]);
    parse_name(p).or_add_diagnostic(p, expected_name);

    Present(m.complete(p, TYPE_DECL))
}

/// Parses the parameters of a function declaration, parentheses included.
// test mlk parameters_are_annotated_with_types
// fun applied(value: Map[Int, String], other: Int): Unit =
//     value
fn parse_parameters(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T!['(']) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T!['(']);
    ParameterListParse.parse_list(p);
    p.expect(T![')']);

    Present(m.complete(p, PARAMETERS))
}

/// The parameters of a function declaration.
///
/// The list may be empty: a function that takes no arguments is written `fun f()`, not
/// `fun f`.
struct ParameterListParse;

impl ParseSeparatedList for ParameterListParse {
    type Kind = SyntaxKind;
    type Parser<'source> = MlkParser<'source>;

    const LIST_KIND: SyntaxKind = PARAMETER_LIST;

    fn parse_element(&mut self, p: &mut MlkParser) -> ParsedSyntax {
        parse_parameter(p)
    }

    fn is_at_list_end(&self, p: &mut MlkParser) -> bool {
        p.at(T![')'])
    }

    fn recover(&mut self, p: &mut MlkParser, parsed_element: ParsedSyntax) -> RecoveryResult {
        parsed_element.or_recover_with_token_set(
            p,
            &ParseRecoveryTokenSet::new(BOGUS_PARAMETER, PARAMETER_RECOVERY_SET),
            expected_parameter,
        )
    }

    fn separating_element_kind(&mut self) -> SyntaxKind {
        T![,]
    }
}

/// Parses a single parameter: its name, and the type it takes if it is written down.
fn parse_parameter(p: &mut MlkParser) -> ParsedSyntax {
    let name = parse_name(p);

    if name.is_absent() {
        return ParsedSyntax::Absent;
    }

    let m = name.precede(p);

    parse_type_annotation(p).ok();

    Present(m.complete(p, PARAMETER))
}

/// Parses the type a parameter takes.
fn parse_type_annotation(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T![:]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T![:]);
    parse_type(p).or_add_diagnostic(p, expected_type);

    Present(m.complete(p, TYPE_ANNOTATION))
}

/// Parses the type a function returns.
fn parse_fun_return_type_annotation(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T![:]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T![:]);
    parse_type(p).or_add_diagnostic(p, expected_type);

    Present(m.complete(p, FUN_RETURN_TYPE_ANNOTATION))
}

/// Parses the body of a function declaration.
fn parse_fun_body(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T![=]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T![=]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);

    Present(m.complete(p, FUN_BODY))
}
