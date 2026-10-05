use crate::diagnostic::DiagKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagCtx<D: DiagKind> {
    diagnostics: Vec<D>,
    error_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagMarker {
    index: usize,
    error_count: usize,
}

impl<D: DiagKind> Default for DiagCtx<D> {
    fn default() -> Self {
        Self {
            diagnostics: Vec::new(),
            error_count: 0,
        }
    }
}

impl<D: DiagKind> DiagCtx<D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit(&mut self, diag: D) {
        if diag.level().is_error() {
            self.error_count += 1;
        }
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn drain(&mut self) -> Vec<D> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn mark(&self) -> DiagMarker {
        DiagMarker {
            index: self.diagnostics.len(),
            error_count: self.error_count,
        }
    }

    pub fn has_new_since(&self, marker: DiagMarker) -> bool {
        self.diagnostics.len() > marker.index
    }

    pub fn has_errors_since(&self, marker: DiagMarker) -> bool {
        self.error_count > marker.error_count
    }

    pub fn count_since(&self, marker: DiagMarker) -> usize {
        self.diagnostics.len() - marker.index
    }

    pub fn since(&self, marker: DiagMarker) -> &[D] {
        &self.diagnostics[marker.index..]
    }

    pub fn drain_since(&mut self, marker: DiagMarker) -> Vec<D> {
        let drained = self.diagnostics.split_off(marker.index);
        self.error_count = marker.error_count;
        drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Category, DiagKind, Level};

    /// A diagnostic a test controls, where the real ones come from a pass.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestDiag {
        level: Level,
        message: &'static str,
    }

    impl DiagKind for TestDiag {
        fn level(&self) -> Level {
            self.level
        }

        fn category(&self) -> Category {
            Category::Parser
        }

        fn code(&self) -> &'static str {
            "01"
        }
    }

    fn error(message: &'static str) -> TestDiag {
        TestDiag {
            level: Level::Error,
            message,
        }
    }

    fn warning(message: &'static str) -> TestDiag {
        TestDiag {
            level: Level::Warning,
            message,
        }
    }

    #[test]
    fn a_new_context_holds_nothing() {
        let ctx = DiagCtx::<TestDiag>::new();

        assert!(!ctx.has_errors());
        assert_eq!(ctx.error_count(), 0);
        assert_eq!(ctx, DiagCtx::default());
    }

    #[test]
    fn only_errors_are_counted() {
        let mut ctx = DiagCtx::new();
        ctx.emit(warning("a warning"));
        ctx.emit(error("an error"));
        ctx.emit(warning("another warning"));

        assert!(ctx.has_errors());
        assert_eq!(ctx.error_count(), 1);
    }

    #[test]
    fn emit_keeps_every_diagnostic_in_order() {
        let mut ctx = DiagCtx::new();
        let marker = ctx.mark();
        ctx.emit(error("first"));
        ctx.emit(warning("second"));

        assert_eq!(ctx.since(marker), &[error("first"), warning("second")]);
    }

    #[test]
    fn a_mark_sees_only_what_comes_after_it() {
        let mut ctx = DiagCtx::new();
        ctx.emit(error("before"));
        let marker = ctx.mark();

        assert!(!ctx.has_new_since(marker));
        assert_eq!(ctx.count_since(marker), 0);
        assert!(ctx.since(marker).is_empty());

        ctx.emit(warning("after"));

        assert!(ctx.has_new_since(marker));
        assert_eq!(ctx.count_since(marker), 1);
        assert_eq!(ctx.since(marker), &[warning("after")]);
    }

    #[test]
    fn a_mark_does_not_see_errors_that_came_before_it() {
        let mut ctx = DiagCtx::new();
        ctx.emit(error("before"));
        let marker = ctx.mark();

        assert!(ctx.has_errors());
        assert!(!ctx.has_errors_since(marker));

        ctx.emit(warning("a warning does not count"));
        assert!(!ctx.has_errors_since(marker));

        ctx.emit(error("after"));
        assert!(ctx.has_errors_since(marker));
    }

    #[test]
    fn drain_takes_every_diagnostic_and_leaves_the_buffer_empty() {
        let mut ctx = DiagCtx::new();
        ctx.emit(error("first"));
        ctx.emit(warning("second"));

        assert_eq!(ctx.drain(), [error("first"), warning("second")]);
        assert!(ctx.drain().is_empty());
    }

    #[test]
    fn drain_keeps_the_count_of_the_errors_it_has_seen() {
        let mut ctx = DiagCtx::new();
        ctx.emit(error("an error"));

        ctx.drain();

        // Draining hands the buffer over; the context still knows the work it collected
        // failed, and only `drain_since` rolls the count back to a mark.
        assert!(ctx.has_errors());
        assert_eq!(ctx.error_count(), 1);
    }

    #[test]
    fn drain_since_takes_only_what_came_after_the_mark_and_rolls_the_count_back() {
        let mut ctx = DiagCtx::new();
        ctx.emit(error("before"));
        let marker = ctx.mark();
        ctx.emit(error("after"));
        ctx.emit(warning("also after"));

        assert_eq!(ctx.drain_since(marker), [
            error("after"),
            warning("also after")
        ]);

        assert_eq!(
            ctx.error_count(),
            1,
            "the error before the mark is still counted"
        );
        assert!(!ctx.has_errors_since(marker));
        assert!(!ctx.has_new_since(marker));
        assert_eq!(ctx.count_since(marker), 0);
        assert!(ctx.since(marker).is_empty());
        assert_eq!(ctx.drain(), [error("before")]);
    }
}
