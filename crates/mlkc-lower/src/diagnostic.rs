//! What lowering reports, and what it reports it about.

use std::fmt;

use mlkc_span::Span;

/// A problem lowering met, and where it is.
///
/// A lowering diagnostic is not a syntax error: the parser has already reported what it
/// could not read, and a node it could not read becomes a `Missing` node of the HIR.
/// What is left for lowering to report is what the HIR cannot hold --- an integer literal
/// that does not fit the value the HIR stores it in, a name the module declares twice ---
/// and it is a value of its own, so that a caller can hold it, compare it, and render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringDiag {
    message: String,
    span: Span,
}

impl LoweringDiag {
    /// A problem at `span`, described in one line.
    pub(crate) fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// What the problem is.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where the problem is.
    pub fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for LoweringDiag {
    /// The message, headed by the range it is at.
    ///
    /// The file is [`LoweringDiag::span`] and not part of the text: a caller that renders
    /// what it lowered knows which file it asked about.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.span.range, self.message)
    }
}
