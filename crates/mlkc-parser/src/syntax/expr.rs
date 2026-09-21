//! Expressions.
//!
//! Binary expressions are parsed by precedence climbing: [`parse_binary_expr`] parses the
//! left-hand side and then, while it is at an operator that binds at least as tightly as it
//! is allowed to, wraps everything parsed so far into the left child of a new `BinExpr` and
//! parses the right-hand side with the operators that bind tighter. The right-hand side
//! therefore stops at an operator of the same precedence, which is what makes the operators
//! left-associative: `1 - 2 - 3` is `(1 - 2) - 3`.

use mlkc_parser_core::{
    parse_lists::ParseSeparatedList,
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
        auxiliary::parse_name,
        parse_error::{expected_expr, expected_pattern},
        pat::parse_pat,
    },
};

/// The precedence the operators of the outermost expression start with.
const MIN_PRECEDENCE: u8 = 0;

/// The tokens a broken expression is recovered at: whatever ends the expression and starts
/// something else where it is written.
const EXPR_RECOVERY_SET: TokenSet<SyntaxKind> =
    token_set![T![,], T![')'], T![']'], T![=], T![in], T![let]];

/// Parses an expression.
pub(crate) fn parse_expr(p: &mut MlkParser) -> ParsedSyntax {
    parse_binary_expr(p, MIN_PRECEDENCE)
}

/// The precedence of a binary operator, or `None` if the token is not an operator.
///
/// The higher the precedence, the tighter the operator binds: in `a || b + c` the sum is
/// computed first, so `+` outranks `||`. From the loosest to the tightest: `||`, `&&`,
/// the equality operators, the comparison operators, the sums, and the products.
// test mlk products_bind_tighter_than_sums
// fun products(): Int =
//     1 + 2 * 3
//
// test mlk comparisons_bind_looser_than_products
// fun comparisons(): Int =
//     1 + 2 * 3 <= 4 * 5 / 6
//
// test mlk logic_binds_looser_than_everything_else
// fun logic(): Int =
//     1 == 2 || 3 < 4 && 5 >= 6
fn binary_precedence(kind: SyntaxKind) -> Option<u8> {
    let precedence = match kind {
        T![||] => 1,
        T![&&] => 2,
        T![==] | T![!=] => 3,
        T![<] | T![<=] | T![>] | T![>=] => 4,
        T![+] | T![-] => 5,
        T![*] | T![/] => 6,
        _ => return None,
    };

    Some(precedence)
}

/// Parses a binary expression whose operators bind at least as tightly as `min_precedence`.
fn parse_binary_expr(p: &mut MlkParser, min_precedence: u8) -> ParsedSyntax {
    let lhs = parse_postfix_expr(p);

    let ParsedSyntax::Present(mut lhs) = lhs else {
        return ParsedSyntax::Absent;
    };

    while let Some(precedence) = binary_precedence(p.cur()) {
        if precedence < min_precedence {
            break;
        }

        let m = lhs.precede(p);

        p.bump_any();
        let rhs = parse_binary_expr(p, precedence + 1);
        rhs.or_add_diagnostic(p, expected_expr);

        lhs = m.complete(p, BIN_EXPR);
    }

    Present(lhs)
}

/// Parses an expression that may be applied to arguments: `f(a, b)(c)`.
// test mlk calls_take_arguments_and_are_applied_to_the_result
// fun calls(): Int =
//     f(g(1), h(2, 3))(4)
fn parse_postfix_expr(p: &mut MlkParser) -> ParsedSyntax {
    let mut expr = parse_primary_expr(p);

    while expr.is_present() && p.at(T!['(']) {
        let m = expr.precede(p);

        p.bump(T!['(']);
        ArgumentListParse.parse_list(p);
        p.expect(T![')']);

        expr = Present(m.complete(p, CALL_EXPR));
    }

    expr
}

/// Parses an expression that cannot take part in a binary expression itself.
fn parse_primary_expr(p: &mut MlkParser) -> ParsedSyntax {
    match p.cur() {
        INT_LITERAL | STRING_LITERAL => parse_literal(p),
        IDENT => parse_var_expr(p),
        LET_KW => parse_let_expr(p),
        L_PAREN => parse_paren_expr(p),
        _ => ParsedSyntax::Absent,
    }
}

/// Parses a literal, which is the one node of the grammar whose kind is the kind of the
/// token it holds: `IntLiteral = value: 'int_literal'`.
// test mlk literals_are_numbers_and_strings
// fun literals(): Int =
//     "a string"
fn parse_literal(p: &mut MlkParser) -> ParsedSyntax {
    let kind = p.cur();

    if !kind.is_literal() {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(kind);

    Present(m.complete(p, kind))
}

/// Parses an expression that is a name: a variable, or the function of a call.
fn parse_var_expr(p: &mut MlkParser) -> ParsedSyntax {
    parse_name(p).map(|name| name.precede(p).complete(p, VAR_EXPR))
}

/// Parses an expression in parentheses.
// test mlk parentheses_group
// fun grouped(): Int =
//     (1 + 2) * (3 - 4)
fn parse_paren_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T!['(']) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T!['(']);
    parse_expr(p).or_add_diagnostic(p, expected_expr);
    p.expect(T![')']);

    Present(m.complete(p, PAREN_EXPR))
}

/// Parses the expression that binds a name to a value and uses it.
///
/// The expression before `in` is parsed with [`parse_expr`]: `in` is not an operator and
/// not the start of one, so the expression ends at it on its own.
// test mlk let_in_binds_a_name_and_uses_it
// fun main(): Int =
//     let x = 42 in
//     x + 1
//
// test mlk let_in_nests_and_ignores_a_name
// fun nested(): Int =
//     let _ = 1 in
//     let x = 2 in
//     x
fn parse_let_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T![let]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T![let]);
    parse_pat(p).or_add_diagnostic(p, expected_pattern);
    p.expect(T![=]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);
    p.expect(T![in]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);

    Present(m.complete(p, LET_EXPR))
}

/// The arguments of a call: `f(a, b)`.
struct ArgumentListParse;

impl ParseSeparatedList for ArgumentListParse {
    type Kind = SyntaxKind;
    type Parser<'source> = MlkParser<'source>;

    const LIST_KIND: SyntaxKind = ARGUMENT_LIST;

    fn parse_element(&mut self, p: &mut MlkParser) -> ParsedSyntax {
        parse_expr(p)
    }

    fn is_at_list_end(&self, p: &mut MlkParser) -> bool {
        p.at(T![')'])
    }

    fn recover(&mut self, p: &mut MlkParser, parsed_element: ParsedSyntax) -> RecoveryResult {
        parsed_element.or_recover_with_token_set(
            p,
            &ParseRecoveryTokenSet::new(BOGUS_EXPR, EXPR_RECOVERY_SET),
            expected_expr,
        )
    }

    fn separating_element_kind(&mut self) -> SyntaxKind {
        T![,]
    }

    fn allow_trailing_separating_element(&self) -> bool {
        true
    }
}
