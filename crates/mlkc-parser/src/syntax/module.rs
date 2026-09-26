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
        attribute::{is_at_declaration, parse_attribute_list, parse_visibility},
        auxiliary::parse_name,
        expr::parse_expr,
        parse_error::{
            expected_declaration, expected_expr, expected_name, expected_parameter,
            expected_parameters, expected_path, expected_type,
        },
        pat::parse_pat_or_recover,
        ty::{parse_path, parse_type},
    },
};

/// The tokens a broken module item is recovered at: the start of the next declaration or
/// import, which is `@`, `pub`, or the keyword of one of the items the language has.
const MODULE_ITEM_RECOVERY_SET: TokenSet<SyntaxKind> =
    token_set![T![@], T![pub], T![fun], T![type], T![use]];

/// The tokens a broken parameter is recovered at: the end of the parameter it was written
/// in, or the end of the list it belongs to.
const PARAMETER_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![T![,], T![')']];

/// Parses the root of the tree: the byte order mark if the file has one, the preamble of the
/// module, the items of the module, and the end of the file.
// test mlk a_module_declares_the_path_it_is_of
// module project.main-module
//
// fun main(): Int =
//     1
pub(crate) fn parse_module_root(p: &mut MlkParser) -> CompletedMarker {
    let m = p.start();

    // The mark is a token of the tree rather than trivia: it is the first thing in the
    // file, and a file that has it is written differently from one that does not.
    p.eat(T![UNICODE_BOM]);

    parse_module_preamble(p).ok();

    ModuleItemListParse.parse_list(p);

    // The list stops at the end of the file, so this eats the token the list left behind:
    // the tree holds the end of the file, and the parser is the one that knows where it is.
    p.expect(T![EOF]);

    m.complete(p, MODULE_ROOT)
}

/// Parses the preamble of a module: the path it declares itself as.
///
/// A module is written in a file, and the preamble is the path of that file in the project:
/// `module project.main-module`. A file that has no preamble declares no path, and the
/// project it belongs to is what says what it is called.
///
/// The attributes a module carries are written in front of the keyword, and an attribute list
/// belongs to the preamble only when the keyword follows it: the same `@name` in front of a
/// declaration is the declaration's own ([`is_at_declaration`]).
// test mlk a_module_may_carry_an_attribute
// @no-prelude
// module project.main-module
//
// fun main(): Int =
//     1
//
// test mlk an_attribute_belongs_to_the_declaration_it_stands_in_front_of
// @builtin
// type Unit
fn parse_module_preamble(p: &mut MlkParser) -> ParsedSyntax {
    if !at_module_preamble(p) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    parse_attribute_list(p);
    p.bump(T![module]);
    parse_path(p).or_add_diagnostic(p, expected_path);

    Present(m.complete(p, MODULE_PREAMBLE))
}

/// Whether the parser is at the preamble of a module: the keyword, or the attributes written
/// in front of it.
///
/// The keyword cannot simply be looked at when the preamble starts with `@`, because the same
/// attribute list could belong to the declaration that follows it. Walking the attributes is
/// possible because an attribute is always `@` followed by a name, and an attribute list that
/// is not followed by the keyword is the next item's, which is what the caller reads it as.
fn at_module_preamble(p: &mut MlkParser) -> bool {
    let mut index = 0;

    while p.nth_at(index, AT) && p.nth_at(index + 1, IDENT) {
        index += 2;
    }

    p.nth_at(index, MODULE_KW)
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
    if is_at_use(p) {
        parse_use_decl(p)
    } else if is_at_declaration(p, FUN_KW) {
        parse_fun_decl(p)
    } else if is_at_declaration(p, TYPE_KW) {
        parse_type_decl(p)
    } else {
        ParsedSyntax::Absent
    }
}

/// Whether the parser is at an import: `use`, possibly with `pub` in front of it.
///
/// An import carries no attributes, so this is not [`is_at_declaration`]: a `@` in front of a
/// `use` is a mistake the reader made, and the tokens of it are foreign to an import.
fn is_at_use(p: &mut MlkParser) -> bool {
    p.at(T![use]) || (p.at(T![pub]) && p.nth_at(1, T![use]))
}

/// Parses an import: a path of the project, and the name it is brought in under.
// test mlk an_import_brings_a_name_in
// use std.core.Int
//
// test mlk an_import_may_be_renamed
// use std.core.Int as Integer
//
// test mlk an_import_may_be_public
// pub use std.core.Int
fn parse_use_decl(p: &mut MlkParser) -> ParsedSyntax {
    if !is_at_use(p) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    parse_visibility(p);
    p.expect(T![use]);
    parse_path(p).or_add_diagnostic(p, expected_path);
    parse_use_alias(p).ok();

    Present(m.complete(p, USE_DECL))
}

/// Parses the name an import is renamed to: `as name`.
fn parse_use_alias(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T![as]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T![as]);
    parse_name(p).or_add_diagnostic(p, expected_name);

    Present(m.complete(p, USE_ALIAS))
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
//
// test mlk a_function_may_be_public
// pub fun main(): Int =
//     42
fn parse_fun_decl(p: &mut MlkParser) -> ParsedSyntax {
    if !is_at_declaration(p, FUN_KW) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    parse_attribute_list(p);
    parse_visibility(p);
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
//
// test mlk a_type_declaration_may_be_public
// pub type Unit
fn parse_type_decl(p: &mut MlkParser) -> ParsedSyntax {
    if !is_at_declaration(p, TYPE_KW) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    parse_attribute_list(p);
    parse_visibility(p);
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

/// Parses a single parameter: the pattern it binds, and the type it takes if it is written.
///
/// A parameter is a pattern rather than a name: a function that ignores an argument writes
/// the wildcard where a function that uses it writes a name, and the type is what a caller
/// reads either way.
// test mlk parameters_are_patterns_with_types
// fun ignored(_: Int, value: Int): Int =
//     value
fn parse_parameter(p: &mut MlkParser) -> ParsedSyntax {
    let pat = parse_pat_or_recover(p);

    if pat.is_absent() {
        return ParsedSyntax::Absent;
    }

    let m = pat.precede(p);

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
