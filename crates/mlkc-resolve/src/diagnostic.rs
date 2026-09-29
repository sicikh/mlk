//! What a resolution found, and where it is.
//!
//! An error is a value, not a sentence: the facts are the names the HIR knows, and where the
//! error is is a place in the HIR --- the entity, and the type among the entity's types. The
//! driver turns that into a span with the ranges it holds for a host.

use std::fmt;

use mlkc_diagnostics::{Category, DiagKind, Diagnostic, Level};
use mlkc_hir_def::{ItemLoc, ModuleId, Name, PlainPathId, ProjectId, dump::TypePlace};
use mlkc_span::Span;

/// What a walk could not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// A path is rooted at the project, and the module is in no project.
    ///
    /// The keyword `project` names the project the module is written in, and a module no
    /// project claims has none: the names after the keyword are names of nothing.
    NoProject,
    /// A path starts with a name that is no project the module may name.
    UnknownProject {
        /// The name written at the root of the path.
        name: Name,
    },
    /// A path names no module of the project it is read in.
    UnknownModule {
        /// The project whose modules the path was read among.
        project: ProjectId,
        /// The name of the module that is not there.
        name: Name,
    },
    /// A module a path reaches exports no name the path ends at.
    UnknownName {
        /// The path of the module, read inside its project.
        path: PlainPathId,
        /// The module the path names.
        module: ModuleId,
        /// The name the path ends at.
        name: Name,
    },
    /// A module a path reaches holds the name the path ends at and does not show it.
    ///
    /// A name a module keeps to itself is a name of the module all the same, and what a walk
    /// found where it ended is nothing: telling a reader why is a look at the module itself,
    /// and it is taken only after a walk has failed ([`crate::hidden_name`]).
    HiddenName {
        /// The path of the module, read inside its project.
        path: PlainPathId,
        /// The module the path names.
        module: ModuleId,
        /// The name the module holds and does not show.
        name: Name,
    },
    /// A re-export chain returns to where it started.
    ///
    /// A name a module re-exports is the path it wrote, and a chain of re-exports that comes
    /// back to a name it already follows denotes nothing: the name is not what it is.
    CyclicImport {
        /// The name the chain returns to.
        name: Name,
    },
    /// A name of the module resolves to nothing.
    ///
    /// The name is a name of the module --- an import brought it in, or the module declared
    /// it --- and what it denotes is nothing: a name imported and not resolved is what a use
    /// of it is told about, wherever it is used.
    UnresolvedName {
        /// The name that resolves to nothing.
        name: Name,
    },
    /// A name of the module is written where a type belongs, and a value is what it denotes.
    ///
    /// The namespaces of a module are apart, and a name is read in the one the place it is
    /// written in asks for: a name that denotes a value is not a type, however the path around
    /// it is written.
    NotAType {
        /// The name written where a type belongs.
        name: Name,
    },
}

impl ResolveError {
    /// The message of the error, in one line.
    pub fn message(&self) -> String {
        match self {
            Self::NoProject => {
                "the path is rooted at the project, and this module is in no project".to_owned()
            },
            Self::UnknownProject { name } => {
                format!("no project `{name}` is one this module may name")
            },
            Self::UnknownModule { project, name } => {
                format!("the project `{project}` holds no module `{name}`")
            },
            Self::UnknownName { path, name, .. } => {
                format!("the module `{path}` exports no name `{name}`")
            },
            Self::HiddenName { path, name, .. } => {
                format!("the module `{path}` holds the name `{name}` and does not show it")
            },
            Self::CyclicImport { name } => {
                format!("the name `{name}` is re-exported through a chain that returns to itself")
            },
            Self::UnresolvedName { name } => {
                format!("the name `{name}` resolves to nothing and is not a name of this module")
            },
            Self::NotAType { name } => {
                format!("the name `{name}` is a value of this module, and a type belongs here")
            },
        }
    }
}

impl DiagKind for ResolveError {
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
        match self {
            Self::NoProject { .. } => "01",
            Self::UnknownProject { .. } => "02",
            Self::UnknownModule { .. } => "03",
            Self::UnknownName { .. } => "04",
            Self::CyclicImport { .. } => "05",
            Self::UnresolvedName { .. } => "06",
            Self::NotAType { .. } => "07",
            Self::HiddenName { .. } => "08",
        }
    }
}

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

    /// The diagnostic a host renders: the error, its kind, and the span of the place.
    ///
    /// The span is the driver's: a pass knows the HIR and not the file, and where a node of
    /// the HIR is written is what the driver holds ([ADR-0009]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    pub fn to_diagnostic(&self, span: Span) -> Diagnostic {
        Diagnostic::from_kind(&self.error, self.error.message()).with_primary(span, "")
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
