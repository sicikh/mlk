//! A path as it is written, and what its base resolved to.

use std::fmt;

use mlkc_intern::{Interned, impl_internable};
use mlkc_la_arena::Idx;

use crate::{
    body::PatId,
    def_map::{LocalScope, Namespace},
    id::{EntityLoc, LocalDefId, UseLoc},
    name::Name,
    type_ref::{TypeRef, TypeVarId},
};

/// Where a path of the project starts: what the names after it are names inside.
///
/// A path is written root first --- `project.data`, `std.core` --- and the root is
/// what says which project the names belong to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathRoot {
    /// `project`: the project the module that wrote the path is written in.
    ///
    /// A module is of a project without naming it, so a path that starts here means what it
    /// means without the name the project is declared under: what a manifest calls the
    /// project is a name no module depends on, and renaming it is not a change to them.
    Project,
    /// A name the module wrote: the name of a project, or a name of the project the module is
    /// in. Which of the two the name is is what the scopes of the whole project decide.
    Named(Name),
}

impl PathRoot {
    /// The name the root is written as, if it is written as one.
    pub fn name(&self) -> Option<&Name> {
        match self {
            Self::Named(name) => Some(name),
            Self::Project => None,
        }
    }
}

impl fmt::Display for PathRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => f.write_str("project"),
            Self::Named(name) => write!(f, "{name}"),
        }
    }
}

/// A path as it is written, without its resolution: where it starts, and the names after it.
///
/// Interned, so a path is one pointer to compare and to copy;
/// two revisions that wrote the same path share one allocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlainPath {
    root: PathRoot,
    segments: Vec<Name>,
}

impl PlainPath {
    /// A path of the names, the first of which is where it starts.
    ///
    /// A path of no names is a path the parser could not read: where it starts is a name that
    /// is not there.
    pub fn from_segments(segments: impl IntoIterator<Item = Name>) -> Self {
        let mut segments = segments.into_iter();
        let root = PathRoot::Named(segments.next().unwrap_or_else(Name::missing));

        Self {
            root,
            segments: segments.collect(),
        }
    }

    /// A path that starts at `root`, of the names after it.
    pub fn from_root(root: PathRoot, segments: impl IntoIterator<Item = Name>) -> Self {
        Self {
            root,
            segments: segments.into_iter().collect(),
        }
    }

    /// Where the path starts.
    pub fn root(&self) -> &PathRoot {
        &self.root
    }

    /// The names after the root, in the order they are written.
    pub fn segments(&self) -> &[Name] {
        &self.segments
    }

    /// The last name, which is the name an import brings in.
    pub fn last(&self) -> Option<&Name> {
        self.segments.last()
    }

    /// The only name, if the path is written as one name.
    pub fn as_ident(&self) -> Option<&Name> {
        match (self.root.name(), self.segments.as_slice()) {
            (Some(name), []) => Some(name),
            _ => None,
        }
    }

    /// Whether the path is written as one name.
    pub fn is_ident(&self) -> bool {
        self.as_ident().is_some()
    }
}

impl fmt::Display for PlainPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root)?;

        for segment in &self.segments {
            write!(f, ".{segment}")?;
        }

        Ok(())
    }
}

impl_internable!(PlainPath);

/// The handle of an interned path.
pub type PlainPathId = Interned<PlainPath>;

/// A path in a type or an expression: where it starts, what is written at it, and what the
/// root of the path denotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathData {
    /// Where the path starts, as it is written.
    ///
    /// A path that is rooted at the project is written with a keyword and no name there; the
    /// names of the path are what follows it.
    pub root: PathRoot,
    /// The type arguments written at the root, in the order they are written.
    ///
    /// A name a path starts at is applied to types the way the names after it are: `Map[Int]`
    /// and `std.Map[Int].Entry` are paths whose root carries an argument, and a path rooted at
    /// the project carries none.
    pub root_args: Vec<TypeRef>,
    /// The names after the root, in the order they are written.
    pub segments: Vec<PathSegmentData>,
    /// What the root denotes, as far as the module alone can tell.
    pub anchor: PathAnchor,
}

impl PathData {
    /// A path of one name, which is what an expression that names one thing is.
    pub fn ident(name: Name, anchor: PathAnchor) -> Self {
        Self {
            root: PathRoot::Named(name),
            root_args: Vec::new(),
            segments: Vec::new(),
            anchor,
        }
    }

    /// Resolves the base of the path against the names of one module, and its arguments.
    ///
    /// This is a step of building and not an API of a later stage:
    /// the HIR is born with the anchors the module alone can give a path,
    /// and a stage that holds the scopes resolves the segments *after* the base,
    /// never the base itself.
    ///
    /// `namespace` is where the name is looked for: the root of a path written where a type
    /// belongs is read in the type namespace of the module, and the root of one written where a
    /// value belongs in its value namespace.
    ///
    /// A path that is rooted at the project has no name to look up: what its root is is the
    /// project the module is in, which no scope of the module decides.
    ///
    /// An anchor a caller already resolved is left alone,
    /// because a caller that knows about a binding or a type variable
    /// knows more than a module scope does.
    pub(crate) fn resolve(&mut self, scope: &LocalScope, namespace: Namespace) {
        if matches!(self.anchor, PathAnchor::Unresolved)
            && let Some(name) = self.root.name()
        {
            self.anchor = scope.anchor(name, namespace);
        }

        // The arguments written at a name are types, wherever the path itself is written:
        // they are read in the type namespace of the module.
        for arg in &mut self.root_args {
            arg.resolve(scope);
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
    /// The project the module is written in, which is what `project` names at the root of a
    /// path.
    ///
    /// Nothing of the path is resolved here: the module is of a project, and the names after
    /// the keyword are names inside that project, which is what the stage that holds the
    /// module paths of the project reads.
    Project,
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
        def_map::{LocalTarget, Namespace},
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

    /// The name of a class, which is a name of the type namespace.
    fn class(name: &str) -> EntityLoc {
        EntityLoc {
            module: module(),
            item: ItemLoc::Class(crate::id::ClassLoc(ItemLocData {
                name: Some(Name::new(name)),
                disambiguator: 0,
            })),
        }
    }

    fn scope_with(name: &str) -> LocalScope {
        let mut scope = LocalScope::default();
        scope.declare(Name::new(name), LocalTarget::Item(class(name)));
        scope
    }

    fn path_of(name: &str) -> PathData {
        PathData::ident(Name::new(name), PathAnchor::Unresolved)
    }

    #[test]
    fn a_path_resolves_against_the_names_of_the_module() {
        let mut path = path_of("foo");
        path.resolve(&scope_with("foo"), Namespace::Ty);

        assert_eq!(path.anchor, PathAnchor::Item(class("foo")));
    }

    #[test]
    fn a_path_that_names_nothing_stays_unresolved() {
        let mut path = path_of("bar");
        path.resolve(&scope_with("foo"), Namespace::Ty);

        assert_eq!(path.anchor, PathAnchor::Unresolved);
    }

    #[test]
    fn a_path_is_resolved_in_the_namespace_it_is_read_in() {
        // A class is a type, not a value: a path written where a value belongs is not
        // answered for by a name of the type namespace.
        let mut path = path_of("foo");
        path.resolve(&scope_with("foo"), Namespace::Value);

        assert_eq!(path.anchor, PathAnchor::Unresolved);
    }

    #[test]
    fn arguments_of_a_path_are_resolved_too() {
        let mut path = PathData {
            root: PathRoot::Named(Name::new("foo")),
            root_args: vec![TypeRef::Path(path_of("bar"))],
            segments: Vec::new(),
            anchor: PathAnchor::Unresolved,
        };
        path.resolve(&scope_with("bar"), Namespace::Ty);

        assert_eq!(path.anchor, PathAnchor::Unresolved);
        assert_eq!(path.root_args, [TypeRef::Path(PathData::ident(
            Name::new("bar"),
            // An argument is a type wherever the path itself is written, so it is read in
            // the type namespace of the module.
            PathAnchor::Item(class("bar")),
        ))],);
    }

    #[test]
    fn an_anchor_a_caller_resolved_is_left_alone() {
        // A caller that knows about a binding or a type variable resolves the path itself,
        // and a module scope has no business overriding it.
        let mut path = PathData::ident(Name::new("foo"), PathAnchor::Item(item("foo")));
        let mut scope = LocalScope::default();
        scope.declare(Name::new("foo"), LocalTarget::Item(item("other")));
        path.resolve(&scope, Namespace::Value);

        assert_eq!(path.anchor, PathAnchor::Item(item("foo")));
    }

    #[test]
    fn a_path_rooted_at_the_project_is_not_a_question_for_the_module() {
        // The names a path rooted at the project is made of are read inside the project, so a
        // name of the module that happens to be written the same way is not what they denote.
        let mut path = PathData {
            root: PathRoot::Project,
            root_args: Vec::new(),
            segments: vec![PathSegmentData {
                name: Name::new("foo"),
                args: Vec::new(),
            }],
            anchor: PathAnchor::Project,
        };
        path.resolve(&scope_with("foo"), Namespace::Ty);

        assert_eq!(path.anchor, PathAnchor::Project);
    }

    #[test]
    fn the_text_of_a_path_is_its_root_and_the_names_after_it() {
        let path = PlainPath::from_segments([Name::new("std"), Name::new("module")]);

        assert_eq!(path.root(), &PathRoot::Named(Name::new("std")));
        assert_eq!(path.segments(), [Name::new("module")]);
        assert_eq!(path.to_string(), "std.module");
    }

    #[test]
    fn a_path_may_be_rooted_at_the_project_it_is_written_in() {
        let path = PlainPath::from_root(PathRoot::Project, [Name::new("module")]);

        assert_eq!(path.segments(), [Name::new("module")]);
        assert_eq!(path.last(), Some(&Name::new("module")));
        assert_eq!(path.to_string(), "project.module");
        // A path of one name is a root and no name after it, and the keyword is not a name.
        assert!(!path.is_ident());
    }
}
