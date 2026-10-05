//! Diagnostics as data: what a stage reports about the input it was given.
//!
//! A pass does not render ([ADR-0009][adr-0009]): it builds a [`Diagnostic`] --- a level, the
//! category of the stage that speaks, a code of the kind within it, a message, and the labels
//! that point at the source --- and the host decides what a person reads.
//! The category and the code are stable values rather than strings a stage formats,
//! because a document or an editor matches on them.
//!
//! [`Ice`] is the other kind of report: a bug of the compiler rather than a mistake of the
//! program. A pass never panics on invalid input, so a panic that reaches the driver is an
//! internal compiler exception, and it is caught and carried like any other report.
//!
//! [adr-0009]: ../../docs/adr/0009-pass-contract.md

pub mod ctx;
mod diagnostic;
mod ice;

use mlkc_span::TextRange;

#[doc(hidden)]
pub use crate::ice::ice_impl;
pub use crate::{
    diagnostic::{Category, DiagKind, Diagnostic, Label, Level},
    ice::Ice,
};

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

#[cfg(test)]
mod tests {
    use mlkc_span::{TextRange, TextSize};

    use super::AsRange;

    fn range() -> TextRange {
        TextRange::new(TextSize::from(2), TextSize::from(5))
    }

    #[test]
    fn a_text_range_is_its_own_range() {
        assert_eq!(range().as_range(), Some(range()));
    }

    #[test]
    fn an_optional_range_is_what_it_carries() {
        assert_eq!(Some(range()).as_range(), Some(range()));
        assert_eq!(None::<TextRange>.as_range(), None);
    }
}
