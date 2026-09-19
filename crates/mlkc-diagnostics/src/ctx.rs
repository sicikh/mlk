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
