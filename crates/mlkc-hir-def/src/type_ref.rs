//! A reference to a type, and a type variable.

use crate::{
    def_map::{LocalScope, Namespace},
    id::EntityLoc,
    path::PathData,
};

/// A type as it is written.
///
/// The value owns what it names: a local target of a path is an [`EntityLoc`] of the
/// module that wrote the type, and a target in another module is the path itself,
/// unresolved. A type reference is therefore comparable, retainable, and serializable
/// on its own, without the item tree it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    /// A path to a type.
    Path(PathData),
    /// The `_` of a type that is inferred.
    Infer,
    /// A type that is missing: the syntax was broken.
    Missing,
}

impl TypeRef {
    /// Resolves the anchors the paths of this type left unresolved.
    ///
    /// A type is read where a type belongs, so the names of it are looked for in the type
    /// namespace of the module that wrote them.
    pub(crate) fn resolve(&mut self, scope: &LocalScope) {
        match self {
            Self::Path(path) => path.resolve(scope, Namespace::Ty),
            Self::Infer | Self::Missing => {},
        }
    }
}

/// A type variable of an entity, named by its owner and its index in the declaration.
///
/// The name is stable, and the entity that owns it is what a dependent records.
/// Where the variables are declared belongs with the syntax that declares them,
/// which the language does not have yet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeVarId {
    /// The entity the variable belongs to.
    pub owner: EntityLoc,
    /// Which variable of that entity this is, counted from the start of its declaration.
    pub index: u32,
}
