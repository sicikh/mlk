use mlkc_la_arena::Idx;

use crate::{
    body::PatId,
    id::{DefId, TypeVarId, UseId},
    name::Name,
    path_syntax::PathSyntax,
};

pub type PathId = Idx<PathData>;

/// Resolved base of a path inside a module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathBase {
    /// Reference to a `use`.
    Use(UseId),
    /// Reference to a definition.
    Def(DefId),
    /// Reference to a type variable.
    TypeVar(TypeVarId),
    /// Reference to a local pattern.
    Binding(PatId),
    /// Unresolved name.
    Unresolved(Name),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathData {
    /// Resolved base of a path.
    pub base: PathBase,
    /// Original unresolved path.
    pub syntax: PathSyntax,
}
