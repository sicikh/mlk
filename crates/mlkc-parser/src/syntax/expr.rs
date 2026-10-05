//! Expressions.
//!
//! Binary expressions are parsed by precedence climbing: [`parse_binary_expr`] parses the
//! left-hand side and then, while it is at an operator that binds at least as tightly as it
//! is allowed to, wraps everything parsed so far into the left child of a new `BinExpr` and
//! parses the right-hand side with the operators that bind tighter. The right-hand side
//! therefore stops at an operator of the same precedence, which is what makes the operators
//! left-associative: `1 - 2 - 3` is `(1 - 2) - 3`.
//!
//! The pipeline is the loosest operator of an expression, and what stands on its right is not
//! an expression: a step is a call, with a place for the value among its arguments, and the
//! climb reads one with [`parse_step`].
//!
//! The left-hand side a binary operator is applied to is read by [`parse_unary_expr`], which
//! is where a sign in front of an expression belongs: the ladder of the rules is
//! [`parse_binary_expr`], [`parse_unary_expr`], [`parse_postfix_expr`],
//! [`parse_primary_expr`], from the loosest to the tightest.

use mlkc_parser_core::{
    parse_lists::{ParseNodeList, ParseSeparatedList},
    parse_recovery::{ParseRecoveryTokenSet, RecoveryResult},
    parsed_syntax::ParsedSyntax::Present,
    prelude::*,
};
use mlkc_rowan::TextRange;
use mlkc_syntax::{
    SyntaxKind::{
        self, ARGUMENT_LIST, BIN_EXPR, BOGUS_DECL, BOGUS_EXPR, BOGUS_PARAMETER, BOGUS_PAT,
        BOOL_LITERAL, CALL_EXPR, ELSE_BRANCH, FALSE_KW, FIELD_EXPR, FN_KW, IDENT, IF_ARM,
        IF_ARM_LIST, IF_EXPR, IF_KW, INT_LITERAL, L_PAREN, LAMBDA_EXPR, LAMBDA_PARAMETER_LIST,
        LET_EXPR, LET_KW, LOCAL_EXPR, LOCAL_KW, MODULE_ITEM_LIST, PARAMETER, PAREN_EXPR, PATH_EXPR,
        PIPE_EXPR, PLACEHOLDER_EXPR, PROJECT_KW, STRING_LITERAL, TRUE_KW, UFCS_CALL, UNARY_EXPR,
        UNDERSCORE,
    },
    T,
};

use crate::{
    parser::MlkParser,
    syntax::{
        auxiliary::parse_name,
        module::{parse_module_item, parse_type_annotation},
        parse_error::{
            expected_call_after_a_pipe, expected_declaration, expected_expr, expected_name,
            expected_parameter, expected_path, expected_path_after_the_dot, expected_pattern,
            expected_place, place_outside_a_step,
        },
        pat::{parse_pat, parse_pat_or_recover},
        ty::parse_path,
    },
};

/// The precedence of the pipeline operator, which is the loosest operator of an expression.
///
/// A step takes what stands before it, `a + b` and all: `a + b |> f(_)` is a step of `f` on
/// the sum. What is written after a step, a binary operator included, is written after the
/// value the pipeline is: `x |> f(_) + 1` is a sum of what the step produced. A sequence,
/// when the language has one, is looser still ([ADR-0014]).
///
/// [ADR-0014]: ../../docs/adr/0014-syntactic-sugar.md
const PIPE_PRECEDENCE: u8 = 0;

/// The precedence the operators of the outermost expression start with: the loosest operator
/// the language has, which is the pipeline.
const MIN_PRECEDENCE: u8 = PIPE_PRECEDENCE;

/// The tokens a broken expression is recovered at: whatever ends the expression and starts
/// something else where it is written.
const EXPR_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![
    T![,],
    T![')'],
    T![']'],
    T![=],
    T![in],
    T![let],
    T![local],
    T![then],
    T![elif],
    T![else]
];

/// The tokens a broken step is recovered at: what stands after a step, and what ends the
/// expression the pipeline is written in.
///
/// A step ends with the argument list of its call, so a dot, a `|>`, or a binary operator
/// written where a step belongs is written after the value the pipeline is. What starts the
/// next item of the module ends it as well, which is what keeps a broken step from reading
/// the declaration that follows it.
///
/// What the mistake costs is the step: the pipeline around it is read whole, and so is the
/// module.
const STEP_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![
    T![,],
    T![')'],
    T![']'],
    T![=],
    T![in],
    T![let],
    T![local],
    T![|>],
    T![+],
    T![-],
    T![*],
    T![/],
    T![==],
    T![!=],
    T![<],
    T![<=],
    T![>],
    T![>=],
    T![&&],
    T![||],
    T![#],
    T![pub],
    T![fun],
    T![type],
    T![use]
];

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
///
/// The pipeline is one of the operators read here, and it is the loosest one: a step takes
/// what stands before it, so `a + b |> f(_)` is read as a step of the sum rather than as a sum
/// of a step, and what is written after the step --- another `|>`, a binary operator --- is
/// read as written after the value the pipeline is.
fn parse_binary_expr(p: &mut MlkParser, min_precedence: u8) -> ParsedSyntax {
    let lhs = parse_unary_expr(p);

    let ParsedSyntax::Present(mut lhs) = lhs else {
        return ParsedSyntax::Absent;
    };

    loop {
        if p.at(T![|>]) {
            if PIPE_PRECEDENCE < min_precedence {
                break;
            }

            let m = lhs.precede(p);

            // Where the `|>` is written: a step that is not there at all is reported there,
            // because the next declaration is very often what stands where the step belongs.
            let written = p.cur_range();
            p.bump(T![|>]);
            parse_step_or_recover(p, written);

            // What is written after a step is written after the value the pipeline is:
            // `.prepare()` after one calls `prepare` on what the step produced. The chain is
            // applied to a value that is there, so it hands one back.
            let piped = m.complete(p, PIPE_EXPR);
            lhs = parse_postfixes(p, Present(piped)).unwrap();

            continue;
        }

        let Some(precedence) = binary_precedence(p.cur()) else {
            break;
        };

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

/// Parses an expression a sign may be written in front of.
///
/// A sign binds tighter than any binary operator and looser than a call: `-f(1) + 2` is
/// `(-f(1)) + 2`, and `-1 - 2` is `(-1) - 2`. What a sign is applied to may be signed as
/// well, so `- -1` is `-(-1)`.
// test mlk a_sign_binds_tighter_than_a_binary_operator
// fun signed(): Int =
//     -1 + 2
//
// test mlk a_sign_may_be_written_twice
// fun doubly_signed(): Int =
//     - -1
//
// test mlk a_sign_applies_to_the_result_of_a_call
// fun negated(): Int =
//     -f(1)
fn parse_unary_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !(p.at(T![-]) || p.at(T![+])) {
        return parse_postfix_expr(p);
    }

    let m = p.start();

    p.bump_any();
    parse_unary_expr(p).or_add_diagnostic(p, expected_expr);

    Present(m.complete(p, UNARY_EXPR))
}

/// Parses an expression that may be applied to arguments and to the operators of a chain:
/// `f(a, b)(c)`, `value.function(a)`, `data@field`.
// test mlk calls_take_arguments_and_are_applied_to_the_result
// fun calls(): Int =
//     f(g(1), h(2, 3))(4)
//
// test mlk a_value_may_be_passed_as_the_first_argument_of_a_call
// fun passed(): Int =
//     value.function(1, 2)
//
// test mlk a_call_may_be_chained_to_another
// fun chained(): Int =
//     value.load().validate()
//
// test mlk a_field_is_read_with_an_at_sign
// fun field(): Int =
//     data@field
fn parse_postfix_expr(p: &mut MlkParser) -> ParsedSyntax {
    let expr = parse_primary_expr(p);

    parse_postfixes(p, expr)
}

/// Parses what may be written after a value, applied to `expr`: a call, a call with the
/// receiver written before the callee, a field read.
///
/// Every form is postfix: what stands before the operator is what it applies to, and the
/// forms chain left to right, so `config.load().validate()` calls `validate` on the value of
/// the call of `load`, and `config@server@port` reads a field of a field.
///
/// A rule that reads a value of its own applies the chain itself: what is written after the
/// step of a pipeline is written after the value the pipeline is, so a dot after a step is
/// a dot called on what the step produced.
fn parse_postfixes(p: &mut MlkParser, expr: ParsedSyntax) -> ParsedSyntax {
    let ParsedSyntax::Present(mut expr) = expr else {
        return ParsedSyntax::Absent;
    };

    loop {
        if p.at(T!['(']) {
            // A call: the arguments, applied to what stands before them.
            let m = expr.precede(p);

            parse_call_arguments(p);

            expr = m.complete(p, CALL_EXPR);
        } else if p.at(T![.]) {
            // A call with the receiver written before the callee: `value.function(a)` is
            // `function(value, a)`. What stands after the dot is the callee the call means,
            // which is a name or a path.
            let m = expr.precede(p);

            let dot = p.cur_range();
            p.bump(T![.]);

            if parse_path(p).is_present() {
                parse_call_arguments(p);
            } else {
                // The callee is not there, and the dot is what the mistake is reported at:
                // the token that stands where the callee belongs is very often the next
                // declaration, and a reader who wrote `value.` is told about the dot. The
                // arguments are not asked for on top of that: the call is broken once.
                p.error(expected_path_after_the_dot(dot));
                parse_missing_arguments(p);
            }

            expr = m.complete(p, UFCS_CALL);
        } else if p.at(T![@]) {
            // A field read: the value, and the name of the field.
            let m = expr.precede(p);

            p.bump(T![@]);
            parse_name(p).or_add_diagnostic(p, expected_name);

            expr = m.complete(p, FIELD_EXPR);
        } else {
            break;
        }
    }

    Present(expr)
}

/// Parses the step of a pipeline: the call the value goes into, and the place it goes in.
///
/// A step is a call: a callee, and an argument list with a `_` among its arguments. The callee
/// is a name the module holds, a path, or --- a call written with the dot --- the callee of a
/// call whose receiver is a name or a path as well: `data |> map.insert("k", _)` is a step of
/// `insert`, with `map` and `"k"` before the value. The receiver is passed first, as the
/// receiver of any call written with the dot is.
///
/// A step is one call, and it ends with the argument list of it: a dot, another `|>`, or
/// a binary operator written after the step is written after the value the pipeline is, and
/// a call on the result of another call goes inside the arguments instead.
// test mlk a_pipeline_passes_its_value_to_a_call
// fun passed(): Int =
//     data |> f(1, _)
//
// test mlk a_step_may_be_written_with_the_dot
// fun inserted(): Int =
//     data |> map.insert("k", _)
fn parse_step(p: &mut MlkParser) -> ParsedSyntax {
    // What the step calls, or the receiver of the call it is written with.
    let receiver = parse_path_expr(p);

    // A step is a call, and what stands where one belongs and is not a call is not a step:
    // the caller recovers from the tokens as from a step that is not there.
    let ParsedSyntax::Present(receiver) = receiver else {
        return ParsedSyntax::Absent;
    };

    let called = receiver.precede(p);

    // A step written with the dot calls the callee after it, and passes the receiver first;
    // any other step is the callee and the arguments of one call.
    let with_the_dot = p.at(T![.]);

    if with_the_dot {
        p.bump(T![.]);
        parse_path(p).or_add_diagnostic(p, expected_path);
    }

    // Whether the call writes the arguments a step needs: a call that writes none has no
    // place either, and what is wrong with it has been reported already.
    let arguments_written = p.at(T!['(']);
    let places = parse_step_arguments(p);

    let kind = if with_the_dot { UFCS_CALL } else { CALL_EXPR };
    let step = called.complete(p, kind);

    // A step from which the place is missing has nowhere to put the value, and the call is
    // what a reader is told about: `x |> f(a)` is a step only as `x |> f(a, _)`.
    if arguments_written && places == 0 {
        p.error(expected_place(step.range(p)));
    }

    Present(step)
}

/// Reads the step of a pipeline, or what stands where one belongs and is not one.
///
/// A caller has nothing in its hands when the rule is done: the step is a child of the
/// pipeline the caller reads, and the tokens that are not a step are kept in a bogus
/// expression where the step would be, so that what the mistake costs is the step.
fn parse_step_or_recover(p: &mut MlkParser, written: TextRange) {
    if parse_step(p).is_present() {
        return;
    }

    let recovery = ParseRecoveryTokenSet::new(BOGUS_EXPR, STEP_RECOVERY_SET);

    // What is written where the step belongs is what a reader is pointed at, and where there
    // is nothing of a step at all --- the next declaration is not one --- the mistake is the
    // `|>` that names a step and holds none.
    let range = match recovery.recover(p) {
        Ok(step) => step.range(p),
        Err(_) => written,
    };

    p.error(expected_call_after_a_pipe(range));
}

/// Parses the arguments of a step's call, parens included: `(a, _)`, and tells how many places
/// the step wrote.
///
/// The arguments are read as the arguments of any call are, and one thing is read among them
/// alone: a `_`, which is where the value the pipeline passes goes.
fn parse_step_arguments(p: &mut MlkParser) -> usize {
    let (_, places) = p.in_step(parse_call_arguments);

    places
}

/// Parses the arguments of a call, parens included: `(a, b)`.
///
/// The rule is read from the two places a call is written: after a callee, and after the
/// callee a dot names. A dot commits the parser to a call, so a `(` that is not there is
/// a mistake rather than a reason not to read a call, while a plain call is only read where
/// the `(` already stands.
fn parse_call_arguments(p: &mut MlkParser) -> CompletedMarker {
    if !p.expect(T!['(']) {
        return parse_missing_arguments(p);
    }

    let arguments = ArgumentListParse.parse_list(p);
    p.expect(T![')']);

    arguments
}

/// Completes a call that has no arguments written: `value.` and `value.function`.
///
/// The call has a slot for the argument list, and a slot without a list is read as a missing
/// one by everything that reads the tree, so the empty list is created even when the parens
/// are not written. Nothing is reported here: the caller has reported the mistake the call is
/// broken by, if there is one.
fn parse_missing_arguments(p: &mut MlkParser) -> CompletedMarker {
    let m = p.start();
    m.complete(p, ARGUMENT_LIST)
}

/// Parses an expression that cannot take part in a binary expression itself.
fn parse_primary_expr(p: &mut MlkParser) -> ParsedSyntax {
    match p.cur() {
        INT_LITERAL | STRING_LITERAL => parse_literal(p),
        TRUE_KW | FALSE_KW => parse_bool_literal(p),
        // A name and the project keyword are what a path expression starts with: the
        // keyword is a root, and the path rule is what reads it where it belongs.
        IDENT | PROJECT_KW => parse_path_expr(p),
        IF_KW => parse_if_expr(p),
        LET_KW => parse_let_expr(p),
        FN_KW => parse_lambda_expr(p),
        LOCAL_KW => parse_local_expr(p),
        L_PAREN => parse_paren_expr(p),
        UNDERSCORE => parse_placeholder_expr(p),
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

/// Parses a truth value: `true` or `false`.
///
/// A truth value is a literal of a node of its own rather than a token that is its kind: the
/// two words are keywords, and one node holds whichever of them is written.
// test mlk a_truth_value_is_a_literal
// fun yes(): Bool =
//     true
//
// test mlk a_truth_value_may_be_anything_a_value_may_be
// fun choose(flag: Bool): Bool =
//     flag && true
fn parse_bool_literal(p: &mut MlkParser) -> ParsedSyntax {
    if !(p.at(T![true]) || p.at(T![false])) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump_any();

    Present(m.complete(p, BOOL_LITERAL))
}

/// Parses a `_` written where a value belongs: `x |> f(_)`.
///
/// A `_` is a place for the value a pipeline passes, and the parser reads one only among the
/// arguments of the call the step of a pipeline is: anywhere else a `_` is not a value, and
/// what stands where one belongs is read as an expression that is not there.
// test mlk a_place_is_written_among_the_arguments_of_a_step
// fun passed(): Int =
//     data |> f(_, 1)
fn parse_placeholder_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(UNDERSCORE) {
        return ParsedSyntax::Absent;
    }

    if !p.reads_places() {
        p.error(place_outside_a_step(p, p.cur_range()));

        return ParsedSyntax::Absent;
    }

    p.count_place();

    let m = p.start();

    p.bump(UNDERSCORE);

    Present(m.complete(p, PLACEHOLDER_EXPR))
}

/// Parses an expression that is a path: a name the body refers to, or a name the module or
/// the project declares under a path.
///
/// A path of one segment is what a body binds or what the module declares; a path of several
/// segments starts at a name of the module, and the names after it are names inside what the
/// first one denotes, which is what the stage that holds the scopes of the project reads.
// test mlk a_value_may_be_named_by_a_qualified_path
// fun main(): Int =
//     data::main-module::start-app(1)
fn parse_path_expr(p: &mut MlkParser) -> ParsedSyntax {
    parse_path(p).map(|path| path.precede(p).complete(p, PATH_EXPR))
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

/// Parses an `if`: a condition, the expression it selects, the `elif` arms written after it,
/// and the expression selected when no condition holds.
///
/// Every part is an expression, and no keyword of the rule is an operator: a condition ends
/// at the `then` written after it, and a branch ends at the `elif` or the `else` after it, so
/// the rule reads the arms in the order they are written without looking past one. The `else`
/// may be left unwritten, and an `if` without one is read with the branch that is not there.
// test mlk an_if_selects_an_expression
// fun pick(flag: Bool): Int =
//     if flag then 1 else 2
//
// test mlk an_if_reads_its_elif_arms_in_order
// fun pick(low: Bool, high: Bool): Int =
//     if low then 1 elif high then 2 else 3
//
// test mlk an_if_may_select_nothing
// fun log(flag: Bool): Unit =
//     if flag then
//         log(flag)
//
// test mlk a_branch_may_be_anything_an_expression_may_be
// fun pick(flag: Bool): Int =
//     if flag then
//         let x = 1 in
//         x + 1
//     else
//         -1
fn parse_if_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(T![if]) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(T![if]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);
    p.expect(T![then]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);

    // The arms are a list of their own, and the list is written even when no `elif` is: what
    // the grammar gives the node is a slot for the arms, and a slot without a list is read as
    // a missing one by everything that reads the tree.
    let arms = p.start();

    while p.at(T![elif]) {
        let arm = p.start();

        p.bump(T![elif]);
        parse_expr(p).or_add_diagnostic(p, expected_expr);
        p.expect(T![then]);
        parse_expr(p).or_add_diagnostic(p, expected_expr);

        arm.complete(p, IF_ARM);
    }

    arms.complete(p, IF_ARM_LIST);

    // The `else` and the expression it selects are one node: an `if` without one selects no
    // value, which is what a reader of the tree sees as the branch that is not there.
    if p.at(T![else]) {
        let otherwise = p.start();

        p.bump(T![else]);
        parse_expr(p).or_add_diagnostic(p, expected_expr);

        otherwise.complete(p, ELSE_BRANCH);
    }

    Present(m.complete(p, IF_EXPR))
}

/// Parses the expression that binds a name to a value and uses it.
///
/// The expression before `in` is parsed with [`parse_expr`]: `in` is not an operator and
/// not the start of one, so the expression ends at it on its own. What a `let` binds is
/// a pattern, and what is written where one belongs and is not one is read as a pattern that
/// is not there, so that the `=` and the `in` around the mistake are read where they are.
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
    parse_pat_or_recover(p).ok();
    p.expect(T![=]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);
    p.expect(T![in]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);

    Present(m.complete(p, LET_EXPR))
}

/// The tokens a broken parameter of a lambda is recovered at: what follows a pattern in the
/// parameter list, which is the comma that separates the parameters, the `)` that ends the
/// list, and the `->` that opens the body. The arrow belongs to the rule that reads it, so it
/// ends the pattern rather than being read into it.
const LAMBDA_PARAMETER_RECOVERY_SET: TokenSet<SyntaxKind> = token_set![T![->], T![,], T![')']];

/// Parses a lambda: a function written where a value belongs, `fn(a, b) -> expr`.
///
/// The parameters are the parameters of a function declaration: patterns, and the types they
/// take where the module writes them. The body is parsed with [`parse_expr`]: it extends as
/// far as it can, so what is written after the body is written after the value the lambda is,
/// and a lambda inside another expression ends where that expression ends.
// test mlk a_lambda_is_a_function_written_where_a_value_belongs
// fun added(value: Int): Int =
//     fn(x) -> x + value
//
// test mlk a_lambda_body_is_an_expression
// fun bound(value: Int): Int =
//     fn(x) -> let y = x + value in y
//
// test mlk a_lambda_may_take_parameters_and_be_applied_at_once
// fun applied(value: Int): Int =
//     (fn(x, y) -> x + y)(value, 1)
//
// test mlk a_lambda_parameter_may_write_the_type_it_takes
// fun annotated(value: Int): Int =
//     fn(x: Int, y: Int) -> x + y + value
fn parse_lambda_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(FN_KW) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(FN_KW);
    p.expect(T!['(']);
    LambdaParameterListParse.parse_list(p);
    p.expect(T![')']);
    p.expect(T![->]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);

    Present(m.complete(p, LAMBDA_EXPR))
}

/// The parameters of a lambda: the parameters between the parentheses.
///
/// A parameter is the parameter of a function declaration: a pattern, and the type it takes
/// where the module writes one. The list may be empty: a lambda that takes no arguments is
/// written `fn()`, not `fn`.
struct LambdaParameterListParse;

impl ParseSeparatedList for LambdaParameterListParse {
    type Kind = SyntaxKind;
    type Parser<'source> = MlkParser<'source>;

    const LIST_KIND: SyntaxKind = LAMBDA_PARAMETER_LIST;

    fn parse_element(&mut self, p: &mut MlkParser) -> ParsedSyntax {
        parse_lambda_parameter(p)
    }

    fn is_at_list_end(&self, p: &mut MlkParser) -> bool {
        p.at(T![')'])
    }

    fn recover(&mut self, p: &mut MlkParser, parsed_element: ParsedSyntax) -> RecoveryResult {
        parsed_element.or_recover_with_token_set(
            p,
            &ParseRecoveryTokenSet::new(BOGUS_PARAMETER, LAMBDA_PARAMETER_RECOVERY_SET),
            expected_parameter,
        )
    }

    fn separating_element_kind(&mut self) -> SyntaxKind {
        T![,]
    }
}

/// Parses one parameter of a lambda: a pattern the body binds, and the type it takes if one is
/// written.
///
/// A lambda parameter is a pattern, as the parameter of a function declaration is: a lambda
/// that ignores an argument writes the wildcard where one that uses it writes a name. What
/// a broken parameter is recovered at is the `->` of the body as well as the comma and the `)`
/// of the list, so that the arrow is left for the rule that reads it rather than being read
/// into the pattern.
fn parse_lambda_parameter(p: &mut MlkParser) -> ParsedSyntax {
    let pat = parse_pat(p).or_recover_with_token_set(
        p,
        &ParseRecoveryTokenSet::new(BOGUS_PAT, LAMBDA_PARAMETER_RECOVERY_SET),
        expected_pattern,
    );

    let Ok(pat) = pat else {
        return ParsedSyntax::Absent;
    };

    let m = pat.precede(p);

    parse_type_annotation(p).ok();

    Present(m.complete(p, PARAMETER))
}

/// The tokens a broken item of a `local` is recovered at: what follows an item, which is the
/// `in` that ends the list, and what starts the next declaration of a module.
const LOCAL_ITEM_RECOVERY_SET: TokenSet<SyntaxKind> =
    token_set![T![in], T![#], T![pub], T![fun], T![type], T![use]];

/// Parses a `local`: the items it declares, and the expression they are visible in,
/// `local fun f(x) = body in expr`.
///
/// The items are the items of a module, and each of them is read by the rule that reads one in
/// a module: the list is the list of the module's items, and it ends at the `in`. The expression
/// after it is parsed with [`parse_expr`]: a `local` extends as far as it can, the way a `let`
/// and a lambda do.
// test mlk a_local_declares_a_function_where_a_value_belongs
// fun applied(value: Int): Int =
//     local fun double(x: Int): Int = x * 2 in
//     double(value)
//
// test mlk a_local_may_declare_several_functions
// fun applied(value: Int): Int =
//     local
//         fun first(x: Int): Int = x + 1
//         fun second(x: Int): Int = first(x) + 1
//     in
//         second(value)
//
// test mlk the_items_of_a_local_are_the_items_of_a_module
// fun applied(value: Int): Int =
//     local
//         type Unit
//         use std::core::Int as Integer
//         fun double(x: Int): Int = x * 2
//     in
//         double(value)
//
// test mlk a_local_is_a_value_like_any_other
// fun applied(value: Int): Int =
//     (local fun double(x: Int): Int = x * 2 in double)(value)
fn parse_local_expr(p: &mut MlkParser) -> ParsedSyntax {
    if !p.at(LOCAL_KW) {
        return ParsedSyntax::Absent;
    }

    let m = p.start();

    p.bump(LOCAL_KW);
    LocalItemListParse.parse_list(p);
    p.expect(T![in]);
    parse_expr(p).or_add_diagnostic(p, expected_expr);

    Present(m.complete(p, LOCAL_EXPR))
}

/// The items of a `local`: the declarations written between the keyword and the `in`.
///
/// The list is the list of the items of a module, read by the rule that reads them there, and
/// it ends where a module's list does not: at the `in` that opens the expression the names of
/// the items are visible in. The list may be empty, which declares nothing and reads as an
/// expression like any other.
struct LocalItemListParse;

impl ParseNodeList for LocalItemListParse {
    type Kind = SyntaxKind;
    type Parser<'source> = MlkParser<'source>;

    const LIST_KIND: SyntaxKind = MODULE_ITEM_LIST;

    fn parse_element(&mut self, p: &mut MlkParser) -> ParsedSyntax {
        parse_module_item(p)
    }

    fn is_at_list_end(&self, p: &mut MlkParser) -> bool {
        // A `local` whose `in` is not written ends at the end of the file: the list has to stop
        // somewhere, and what is left is what the `in` is reported by.
        p.at(T![in]) || p.at(T![EOF])
    }

    fn recover(&mut self, p: &mut MlkParser, parsed_element: ParsedSyntax) -> RecoveryResult {
        parsed_element.or_recover_with_token_set(
            p,
            &ParseRecoveryTokenSet::new(BOGUS_DECL, LOCAL_ITEM_RECOVERY_SET),
            expected_declaration,
        )
    }
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
