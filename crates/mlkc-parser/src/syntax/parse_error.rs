//! The diagnostics the parse rules report.
//!
//! A rule never builds a diagnostic inline: it asks one of these for what it expected, so
//! that the same mistake is described the same way everywhere, and so that the message can
//! depend on what the parser found (the token, or the end of the file).

use mlkc_parser_core::diagnostic::ParseDiagnostic;
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
