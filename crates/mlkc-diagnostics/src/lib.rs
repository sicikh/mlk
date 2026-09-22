pub mod ctx;
mod diagnostic;
mod ice;
mod render;

use mlkc_span::TextRange;

pub use crate::diagnostic::{Category, DiagKind, Diagnostic, Label, Level};

/// Conversion of a range-like value into an optional [TextRange].
pub trait AsRange {
    /// Returns the range, or `None` if the value carries no location at all.
    fn as_range(&self) -> Option<TextRange>;
}

impl AsRange for TextRange {
    fn as_range(&self) -> Option<TextRange> {
        Some(*self)
    }
}

impl AsRange for Option<TextRange> {
    fn as_range(&self) -> Option<TextRange> {
        *self
    }
}
