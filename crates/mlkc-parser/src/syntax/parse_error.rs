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

/// The parser expected a name, which is what `fun`, `type`, `@` and a path segment need.
pub(crate) fn expected_name(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("name", range, p)
}

/// The parser found the project keyword where a name of a path belongs.
///
/// A path is rooted at the project --- `project.data` --- and the segment it starts at is the
/// only one that is written that way: after a dot, a name belongs. The keyword is read as the
/// segment it is written as all the same, so that a path a mistake is written in stays whole.
pub(crate) fn project_where_a_name_belongs(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new(
        format!(
            "Expected a name but instead found '{}': the keyword `project` roots a path, and a \
         name belongs after a dot.",
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

/// The parser expected a path, which is what a preamble and a type are written with.
pub(crate) fn expected_path(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("path", range, p)
}

/// The parser expected a pattern, which is what the left-hand side of `let` is.
pub(crate) fn expected_pattern(p: &MlkParser, range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node("pattern", range, p)
}
