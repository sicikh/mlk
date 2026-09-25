//! A path as it is written, and what its base resolved to.

use std::fmt;

use mlkc_intern::{Interned, impl_internable};
use mlkc_la_arena::Idx;

use crate::{
    body::PatId,
    def_map::LocalScope,
    id::{EntityLoc, LocalDefId, UseLoc},
    name::Name,
    type_ref::{TypeRef, TypeVarId},
};

/// A path as it is written, without its resolution: the segments it names.
///
/// Interned, so a path is one pointer to compare and to copy;
/// two revisions that wrote the same path share one allocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlainPath {
    segments: Vec<Name>,
}

impl PlainPath {
    /// A path of the segments, in the order they are written.
    pub fn from_segments(segments: impl IntoIterator<Item = Name>) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    /// The segments of the path, in the order they are written.
    pub fn segments(&self) -> &[Name] {
        &self.segments
    }

    /// The number of segments.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Whether the path has no segments at all, which only broken syntax produces.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// The first segment.
    pub fn first(&self) -> Option<&Name> {
        self.segments.first()
    }

    /// The last segment.
    pub fn last(&self) -> Option<&Name> {
        self.segments.last()
    }

    /// The only segment, if the path is a single name.
    pub fn as_ident(&self) -> Option<&Name> {
        match self.segments.as_slice() {
            [name] => Some(name),
            _ => None,
        }
    }

    /// Whether the path is a single name.
    pub fn is_ident(&self) -> bool {
        self.as_ident().is_some()
    }
}

impl fmt::Display for PlainPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for segment in &self.segments {
            if !first {
                f.write_str(".")?;
            }
            first = false;
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}

impl_internable!(PlainPath);

/// The handle of an interned path.
pub type PlainPathId = Interned<PlainPath>;

/// A path in a type or an expression: the segments as written, with their arguments,
/// and what the base of the path resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathData {
    /// The segments of the path, in the order they are written.
    pub segments: Vec<PathSegmentData>,
    /// What the base of the path denotes, as far as the module alone can tell.
    pub anchor: PathAnchor,
}

impl PathData {
    /// A path of one segment, which is what an expression that names one thing is.
    pub fn ident(name: Name, anchor: PathAnchor) -> Self {
        Self {
            segments: vec![PathSegmentData {
                name,
                args: Vec::new(),
            }],
            anchor,
        }
    }

    /// Whether the path is a single name.
    pub fn is_ident(&self) -> bool {
        self.segments.len() == 1
    }

    /// The first segment.
    pub fn first(&self) -> Option<&PathSegmentData> {
        self.segments.first()
    }

    /// The name of the last segment, which is the name an import brings in.
    pub fn last_name(&self) -> Option<&Name> {
        self.segments.last().map(|segment| &segment.name)
    }

    /// Resolves the base of the path against the names of one module, and its arguments.
    ///
    /// This is a step of building and not an API of a later stage:
    /// the HIR is born with the anchors the module alone can give a path,
    /// and a stage that holds the scopes resolves the segments *after* the base,
    /// never the base itself.
    ///
    /// An anchor a caller already resolved is left alone,
    /// because a caller that knows about a binding or a type variable
    /// knows more than a module scope does.
    pub(crate) fn resolve(&mut self, scope: &LocalScope) {
        if matches!(self.anchor, PathAnchor::Unresolved)
            && let Some(first) = self.segments.first()
        {
            self.anchor = scope.anchor(&first.name);
        }

        for segment in &mut self.segments {
            for arg in &mut segment.args {
                arg.resolve(scope);
            }
        }
    }
}

/// One segment of a path: a name, and the type arguments written at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegmentData {
    /// The name of the segment.
    pub name: Name,
    /// The type arguments written at the segment, in the order they are written.
    pub args: Vec<TypeRef>,
}

/// What the base of a path denotes inside the module that wrote it.
///
/// The segments after the base are resolved by the stage that holds the scopes:
/// nothing in the HIR points at an entity of another module,
/// and a name that belongs to one is kept as the path the module wrote ([ADR-0010]).
///
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathAnchor {
    /// An entity the module declares.
    Item(EntityLoc),
    /// An entry of the module's import table.
    Use(UseLoc),
    /// An entity declared inside the enclosing body.
    Local(LocalDefId),
    /// A type variable of an entity.
    TypeVar(TypeVarId),
    /// A binding of the enclosing body: a parameter or a `let`.
    Binding(PatId),
    /// A name that the module alone could not resolve:
    /// a path that starts with a project name, or a name that is not there at all.
    ///
    /// A stage that holds the scopes and the module paths of the project decides which of the two
    /// it is; the HIR does not.
    Unresolved,
}

/// The id of a path inside the arena of one body.
pub type PathId = Idx<PathData>;

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;
    use crate::{
        def_map::LocalEntry,
        id::{FunctionLoc, ItemLoc, ItemLocData, ModuleId},
        type_ref::TypeRef,
    };

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn item(name: &str) -> EntityLoc {
        EntityLoc {
            module: module(),
            item: ItemLoc::Function(FunctionLoc(ItemLocData {
                name: Some(Name::new(name)),
                disambiguator: 0,
            })),
        }
    }

    fn scope_with(name: &str) -> LocalScope {
        let mut scope = LocalScope::default();
        scope.declare(Name::new(name), LocalEntry::Item(item(name)));
        scope
    }

    fn path_of(name: &str) -> PathData {
        PathData::ident(Name::new(name), PathAnchor::Unresolved)
    }

    #[test]
    fn a_path_resolves_against_the_names_of_the_module() {
        let mut path = path_of("foo");
        path.resolve(&scope_with("foo"));

        assert_eq!(path.anchor, PathAnchor::Item(item("foo")));
    }

    #[test]
    fn a_path_that_names_nothing_stays_unresolved() {
        let mut path = path_of("bar");
        path.resolve(&scope_with("foo"));

        assert_eq!(path.anchor, PathAnchor::Unresolved);
    }

    #[test]
    fn arguments_of_a_path_are_resolved_too() {
        let mut path = PathData {
            segments: vec![PathSegmentData {
                name: Name::new("foo"),
                args: vec![TypeRef::Path(path_of("bar"))],
            }],
            anchor: PathAnchor::Unresolved,
        };
        path.resolve(&scope_with("bar"));

        assert_eq!(path.anchor, PathAnchor::Unresolved);
        assert_eq!(path.segments[0].args, [TypeRef::Path(PathData {
            segments: vec![PathSegmentData {
                name: Name::new("bar"),
                args: Vec::new(),
            }],
            anchor: PathAnchor::Item(item("bar")),
        })],);
    }

    #[test]
    fn an_anchor_a_caller_resolved_is_left_alone() {
        // A caller that knows about a binding or a type variable resolves the path itself,
        // and a module scope has no business overriding it.
        let mut path = PathData::ident(Name::new("foo"), PathAnchor::Item(item("foo")));
        let mut scope = LocalScope::default();
        scope.declare(Name::new("foo"), LocalEntry::Item(item("other")));
        path.resolve(&scope);

        assert_eq!(path.anchor, PathAnchor::Item(item("foo")));
    }

    #[test]
    fn the_text_of_a_path_is_the_segments_joined() {
        let path = PlainPath::from_segments([Name::new("project"), Name::new("module")]);

        assert_eq!(path.to_string(), "project.module");
    }
}
