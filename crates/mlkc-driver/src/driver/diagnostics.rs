//! The diagnostics of a file: the parts a host reads, in the order the stages reported them.

use std::sync::Arc;

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::ModuleId;
use mlkc_vfs::FileId;

use super::{Driver, none};

/// The diagnostics of one file, in the order the stages of the pipeline reported them.
///
/// The parts are the values the stages left: what the parser reported, what the lowering of the
/// module reported, what the resolution of it found, and what checking its types found. A part
/// that did not change is the value the driver already held, and no part is copied into another:
/// what a host reads is the parts in order ([`Diagnostics::iter`]).
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// What the parser reported.
    parse: Arc<[Diagnostic]>,
    /// What the lowering of the module reported.
    lowering: Arc<[Diagnostic]>,
    /// What the resolution of the module reported.
    resolution: Arc<[Diagnostic]>,
    /// What checking the types of the module's signatures and bodies reported.
    types: Arc<[Diagnostic]>,
}

impl Diagnostics {
    /// What the parser reported, as the stage left it.
    pub fn parse(&self) -> &Arc<[Diagnostic]> {
        &self.parse
    }

    /// What the lowering of the module reported, as the stage left it.
    pub fn lowering(&self) -> &Arc<[Diagnostic]> {
        &self.lowering
    }

    /// What the resolution of the module reported, as the stage left it.
    pub fn resolution(&self) -> &Arc<[Diagnostic]> {
        &self.resolution
    }

    /// What checking the types of the module reported, as the stage left it.
    pub fn types(&self) -> &Arc<[Diagnostic]> {
        &self.types
    }

    /// The diagnostics of the file, in the order the stages reported them.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.parse
            .iter()
            .chain(self.lowering.iter())
            .chain(self.resolution.iter())
            .chain(self.types.iter())
    }

    /// How many diagnostics the file has.
    pub fn len(&self) -> usize {
        self.parse.len() + self.lowering.len() + self.resolution.len() + self.types.len()
    }

    /// Whether the file has no diagnostics at all.
    pub fn is_empty(&self) -> bool {
        self.parse.is_empty()
            && self.lowering.is_empty()
            && self.resolution.is_empty()
            && self.types.is_empty()
    }
}

impl Driver {
    /// The diagnostics of `file`: what the parser reported, what the lowering of the module
    /// reported, what the resolution of it found, and what checking its types found
    /// ([ADR-0016], [ADR-0017]).
    ///
    /// Each stage's diagnostics are a value of their own, computed once per what their stage
    /// read and shared as an `Arc`: a mistake the parser reported is not a reason to render
    /// what the lowering and the resolution reported again.
    ///
    /// `None` when the file has no parse: see [`Driver::parse`].
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn diagnostics(&mut self, file: FileId) -> Option<Diagnostics> {
        let parse = self.parse_diagnostics(file)?;
        let lowered = self.lower(file);

        let lowering = lowered
            .as_ref()
            .map_or_else(none, |lowered| lowered.diagnostics().clone());
        let resolution = lowered
            .as_ref()
            .and_then(|_| self.resolution_diagnostics(ModuleId(file)))
            .unwrap_or_else(none);
        let types = lowered
            .as_ref()
            .and_then(|_| self.type_diagnostics(ModuleId(file)))
            .unwrap_or_else(none);

        Some(Diagnostics {
            parse,
            lowering,
            resolution,
            types,
        })
    }
}
