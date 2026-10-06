//! What a resolution found, and where it is.
//!
//! An error is a value, not a sentence: the facts are the names the HIR knows, and where the
//! error is is a place in the HIR --- the entity, and the type among the entity's types. The
//! driver turns that into a span with the ranges it holds for a host.

use std::fmt;

use mlkc_diagnostics::{Category, DiagKind, Diagnostic, Level};
use mlkc_hir_def::{ItemLoc, ResolveError, dump::TypePlace};
use mlkc_span::Span;

/// Where a resolution error is: the entity it is in, and the type among the entity's types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvePlace {
    /// The entity the error is in.
    item: ItemLoc,
    /// Which type of the entity it is, when the error is about a type the entity writes.
    place: Option<TypePlace>,
}

impl ResolvePlace {
    /// A place in the surface of a module.
    pub(crate) fn new(item: ItemLoc, place: Option<TypePlace>) -> Self {
        Self { item, place }
    }

    /// The entity the error is in.
    pub fn item(&self) -> &ItemLoc {
        &self.item
    }

    /// Which type of the entity it is, when the error is about a type the entity writes.
    pub fn type_place(&self) -> Option<TypePlace> {
        self.place
    }
}

/// A resolution error, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveDiag {
    error: ResolveError,
    place: ResolvePlace,
}

impl ResolveDiag {
    /// An error at `place`.
    pub(crate) fn new(error: ResolveError, place: ResolvePlace) -> Self {
        Self { error, place }
    }

    /// What the resolution found.
    pub fn error(&self) -> &ResolveError {
        &self.error
    }

    /// Where it is.
    pub fn place(&self) -> &ResolvePlace {
        &self.place
    }

    /// The same place, with the error a look found instead of what the walk did.
    ///
    /// A name the walk could not find may be a name a module holds and does not show, and the
    /// driver tells about it by rendering the error of the look
    /// ([`hidden_name`](crate::hidden_name)) in the place of the walk's.
    pub fn with_error(&self, error: ResolveError) -> Self {
        Self {
            error,
            place: self.place.clone(),
        }
    }

    /// The diagnostic a host renders: the error, its kind, and the span of the place.
    ///
    /// The span is the driver's: a pass knows the HIR and not the file, and where a node of
    /// the HIR is written is what the driver holds ([ADR-0009]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    pub fn to_diagnostic(&self, span: Span) -> Diagnostic {
        Diagnostic::from_kind(self, self.error.message()).with_primary(span, "")
    }
}

impl DiagKind for ResolveDiag {
    fn level(&self) -> Level {
        Level::Error
    }

    /// Resolving the names of a module against the rest of the project is the global half of
    /// name resolution ([ADR-0005][adr-0005]).
    ///
    /// [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md
    fn category(&self) -> Category {
        Category::Resolver
    }

    /// Two digits of the kind of the error, which stay the same however a message is worded.
    fn code(&self) -> &'static str {
        match self.error {
            ResolveError::NoProject => "01",
            ResolveError::UnknownProject { .. } => "02",
            ResolveError::UnknownModule { .. } => "03",
            ResolveError::UnknownName { .. } => "04",
            ResolveError::CyclicImport { .. } => "05",
            ResolveError::UnresolvedName { .. } => "06",
            ResolveError::NotAType { .. } => "07",
            ResolveError::HiddenName { .. } => "08",
            ResolveError::NotAValue { .. } => "09",
            ResolveError::NestedName { .. } => "10",
        }
    }
}

impl fmt::Display for ResolveDiag {
    /// The message of the error, headed by the entity it is in.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.place.type_place() {
            Some(place) => {
                write!(
                    f,
                    "{:?} [{place:?}]: {}",
                    self.place.item(),
                    self.error.message()
                )
            },
            None => write!(f, "{:?}: {}", self.place.item(), self.error.message()),
        }
    }
}
