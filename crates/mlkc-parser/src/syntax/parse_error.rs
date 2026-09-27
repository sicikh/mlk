//! The diagnostics the parse rules report.
//!
//! A rule never builds a diagnostic inline: it asks one of these for what it expected, so
//! that the same mistake is described the same way everywhere, and so that the message can
//! depend on what the parser found (the token, or the end of the file).

use mlkc_parser_core::{Parser, diagnostic::ParseDiagnostic};
use mlkc_rowan::TextRange;

use crate::parser::MlkParser;

/// The parser expected a declaration: `fun`, `type`, an attribute list in front of one of
/// them, or `pub`.
pub(crate) fn expected_declaration(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("declaration", range, p)
}

/// The parser expected a name, which is what `fun`, `type`, an attribute, a field, and a path
/// segment need.
pub(crate) fn expected_name(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("name", range, p)
}

/// The parser found the project keyword where a name of a path belongs.
///
/// A path is rooted at the project --- `project::data` --- and the segment it starts at is the
/// only one that is written that way: after a `::`, a name belongs. The keyword is read as the
/// segment it is written as all the same, so that a path a mistake is written in stays whole.
pub(crate) fn project_where_a_name_belongs(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new(
        format!(
            "Expected a name but instead found '{}': the keyword `project` roots a path, and a \
         name belongs after a `::`.",
            p.text(range)
        ),
        range,
    )
}

/// The parser expected the parameter list of a function declaration.
pub(crate) fn expected_parameters(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("parameter list", range, p)
}

/// The parser expected a parameter of a function declaration.
pub(crate) fn expected_parameter(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("parameter", range, p)
}

/// The parser expected an expression.
pub(crate) fn expected_expr(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("expression", range, p)
}

/// The parser expected a type.
pub(crate) fn expected_type(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("type", range, p)
}

/// The parser expected a path, which is what a preamble, a type, and the callee of a call
/// written with a dot are written with.
pub(crate) fn expected_path(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("path", range, p)
}

/// The parser expected the callee of the call a dot makes, and the dot is what it reports on.
///
/// The callee is what stands after the dot, and the token that stands where it belongs is
/// very often the next declaration --- a body that ends with `value.` is followed by the `fun`
/// of the one after it --- so what is said is that the dot has no callee, rather than that the
/// token a reader would be pointed at is one the path cannot be made of.
pub(crate) fn expected_path_after_the_dot(range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new(
        "Expected a path after the `.`: a dot is written with the call it makes.",
        range,
    )
}

/// The parser expected a pattern, which is what the left-hand side of `let` is.
pub(crate) fn expected_pattern(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("pattern", range, p)
}

/// The parser expected the call a step is, and found what is not one.
///
/// What stands on the right of a `|>` is a step: a call, with a `_` among its arguments for
/// the value the pipeline passes. A value, a sign, a parenthesis --- anything that is not
/// a call --- has nowhere to put the value, and the tokens are read as a step that is not
/// there rather than as a step of something else.
///
/// The message quotes no piece of the text, because what it is reported at is not always a piece
/// of a step: where the tokens are not one, the `|>` is what the reader is pointed at.
pub(crate) fn expected_call_after_a_pipe(range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new(
        "Expected a call after the `|>`: a step is a call, and a `_` among its arguments is \
         where the value the pipeline passes goes.",
        range,
    )
}

/// The parser expected a place for the value a pipeline passes, and the call it read writes
/// none.
///
/// A step that writes no `_` is a call the value cannot reach, and the call is what the reader
/// is told about: `x |> f(a)` is a step only as `x |> f(a, _)`.
pub(crate) fn expected_place(range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new(
        "Expected a `_` among the arguments: a step passes the value the pipeline gives it \
         into the call it is, and a `_` is where that value goes.",
        range,
    )
}

/// The parser found a `_` where no pipeline passes a value.
///
/// A `_` is a place for the value a pipeline passes, and it is read only among the arguments
/// of the call on the right of a `|>`: anywhere else it is not a value, and what stands where
/// one belongs is read as an expression that is not there.
pub(crate) fn place_outside_a_step(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new(
        format!(
            "Expected a value but instead found '{}': a `_` is the place a pipeline passes its \
             value into, and it is written among the arguments of the call on the right of \
             a `|>`.",
            p.text(range)
        ),
        range,
    )
}
