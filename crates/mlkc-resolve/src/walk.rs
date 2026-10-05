//! The walk over the paths of a module: the module a path names, and the name it ends at.

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{
    EntityLoc, Export, Interface, ModuleId, ModuleIndex, ModuleLocator, Name, Namespace,
    PathAnchor, PathData, PathRoot, PlainPath, PlainPathId, ProjectGraph, ProjectId,
};

use crate::{
    closure::{Closure, Read},
    diagnostic::ResolveError,
    resolve::Resolution,
};

/// What a path denotes, in the terms a scope is written in.
///
/// A walk answers with a target; a caller that resolves a written path reads what it denotes
/// here, and continues from where the target stands when the path goes on ([ADR-0017]).
///
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Target {
    /// The entity the path denotes, where a type belongs.
    pub(crate) ty: Option<EntityLoc>,
    /// The entity the path denotes, where a value belongs.
    pub(crate) value: Option<EntityLoc>,
    /// The module, or the prefix of a module path, the path denotes.
    pub(crate) module: Option<ModuleLocator>,
    /// Where a module the path names stands: the project, and the segments it is called by.
    /// A path that goes on after the name is read from here.
    pub(crate) place: Option<(ProjectId, Vec<Name>)>,
}

impl Target {
    /// The entity the path denotes, where a type belongs.
    pub fn ty(&self) -> Option<&EntityLoc> {
        self.ty.as_ref()
    }

    /// The entity the path denotes, where a value belongs.
    pub fn value(&self) -> Option<&EntityLoc> {
        self.value.as_ref()
    }

    /// A target that denotes an entity, in the namespaces it denotes it in.
    ///
    /// A walk builds targets of its own; this is the constructor a caller needs when it builds
    /// a target by hand --- a test, or a host that answers a walk with what it holds.
    pub fn entity(ty: Option<EntityLoc>, value: Option<EntityLoc>) -> Self {
        Self {
            ty,
            value,
            ..Self::default()
        }
    }

    /// The module, or the prefix of a module path, the path denotes.
    pub fn module(&self) -> Option<&ModuleLocator> {
        self.module.as_ref()
    }

    /// Where the module the path names stands: the project, and the segments it is called by.
    pub fn place(&self) -> Option<&(ProjectId, Vec<Name>)> {
        self.place.as_ref()
    }

    /// Whether the path denotes nothing at all.
    pub fn is_empty(&self) -> bool {
        self.ty.is_none() && self.value.is_none() && self.module.is_none()
    }
}

impl Target {
    /// What a name of an interface denotes, before the path of a re-export is followed.
    pub(crate) fn of_export(export: &Export) -> Self {
        Self {
            ty: export.ty.clone(),
            value: export.value.clone(),
            ..Self::default()
        }
    }

    /// The target of a module a path names.
    pub(crate) fn of_module(project: ProjectId, segments: Vec<Name>, module: ModuleId) -> Self {
        Self {
            module: Some(ModuleLocator::Module(module)),
            place: Some((project, segments)),
            ..Self::default()
        }
    }

    /// The target of a prefix of a module path: a name no module has, and that modules stand
    /// under.
    pub(crate) fn prefix(project: ProjectId, segments: Vec<Name>) -> Self {
        let path = PlainPathId::new(PlainPath::from_root(PathRoot::Project, segments.clone()));

        Self {
            module: Some(ModuleLocator::Prefix {
                project: project.clone(),
                path,
            }),
            place: Some((project, segments)),
            ..Self::default()
        }
    }

    /// Fills what this target leaves out with what a name a re-export denotes.
    ///
    /// What a module declares wins over what it imports, and the two are told apart per
    /// namespace: the re-export is what the name denotes where the module declares nothing.
    pub(crate) fn fill(&mut self, other: Self) {
        self.ty = self.ty.take().or(other.ty);
        self.value = self.value.take().or(other.value);
        self.module = self.module.take().or(other.module);
        self.place = self.place.take().or(other.place);
    }
}

/// Where a walk stands: what the names written after the segments read so far are read in.
///
/// A path is read from the project it is written in; a name that denotes a module is read from
/// that module, and a name that denotes a prefix of module paths from that prefix. What a walk
/// reads under a place is the same in the three cases --- the modules the names name, the names
/// a module holds, and the prefixes that are what is left --- and what differs is where the walk
/// stood before, and what a name that is not there is reported as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// The project itself: the root of a path is a name of it.
    Project,
    /// A module of a project: the names after it are read in its interface.
    Module(ModuleId),
    /// A prefix of module paths: the names after it are the modules that stand under it.
    Prefix,
}

/// The tables a walk reads: the module indexes of the projects a path may name, and the
/// interfaces of the modules the walk reaches.
///
/// A walk is the shared machinery of every stage that resolves a path: the resolution of a
/// module walks the paths of its surface ([ADR-0016]), a check walks the paths of its bodies
/// ([ADR-0017]), and both walk over the closure the driver gathered for them ([ADR-0009]).
///
/// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
pub struct Walk<'a> {
    /// The projects, what each of them depends on, and the project of every module.
    graph: &'a ProjectGraph,
    /// The module indexes of the projects a path of the module may name.
    indexes: &'a BTreeMap<ProjectId, Arc<ModuleIndex>>,
    /// The interfaces the walk reached.
    interfaces: BTreeMap<ModuleId, Arc<Interface>>,
    /// The entries of the indexes the walk read, in the order it read them.
    reads: Vec<Read>,
    /// How a module the walk has not read yet is fetched: the driver pulls its interface.
    /// A walk over a closure that was gathered already reads only what the closure holds.
    fetch: Option<&'a mut dyn FnMut(ModuleId) -> Option<Arc<Interface>>>,
}

impl<'a> Walk<'a> {
    /// A walk that gathers what it reads, fetching the interfaces it has not read yet.
    pub(crate) fn gathering(
        graph: &'a ProjectGraph,
        indexes: &'a BTreeMap<ProjectId, Arc<ModuleIndex>>,
        fetch: &'a mut dyn FnMut(ModuleId) -> Option<Arc<Interface>>,
    ) -> Self {
        Self {
            graph,
            indexes,
            interfaces: BTreeMap::new(),
            reads: Vec::new(),
            fetch: Some(fetch),
        }
    }

    /// A walk over the closure a resolution was handed.
    pub fn of(graph: &'a ProjectGraph, closure: &'a Closure) -> Self {
        Self {
            graph,
            indexes: closure.indexes(),
            interfaces: closure.interfaces().clone(),
            reads: Vec::new(),
            fetch: None,
        }
    }

    /// What the walk read: the interfaces it reached, and the entries it read.
    pub(crate) fn into_closure(self) -> Closure {
        Closure::new(self.indexes.clone(), self.interfaces, self.reads)
    }

    /// The project a module is in: a project the graph holds.
    ///
    /// A module the graph does not hold a project for --- a file a host pushed, which no
    /// manifest claimed --- is a module of no project, and so is a module whose project the
    /// graph no longer has.
    pub fn project_of(&self, module: ModuleId) -> Option<ProjectId> {
        let project = self.graph.project_of(module)?;

        self.graph.project(project).map(|_| project.clone())
    }

    /// The entity a path written in a body denotes, in `namespace`.
    ///
    /// The anchor of a path is what the stage that lowered it could tell of its root: a name of
    /// the module, an entry of its import table, the project it is written in, or something the
    /// body's own stage answers for ([`PathAnchor::Binding`], a name declared inside the body).
    /// What is beyond the module --- the names after an import, and a path rooted at a project ---
    /// is walked here, over the closure the walk was handed ([ADR-0016]).
    ///
    /// The namespace is what turns the answer into a fact: a path is written where a type belongs
    /// or where a value belongs, and a name that denotes the other is [`ResolveError::NotAType`]
    /// or [`ResolveError::NotAValue`]. A name written after an entity is
    /// [`ResolveError::NestedName`]: the language has no members yet.
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn entity_of(
        &mut self,
        module: ModuleId,
        resolution: &Resolution,
        path: &PathData,
        namespace: Namespace,
    ) -> (Option<EntityLoc>, Vec<ResolveError>) {
        match path.anchor.clone() {
            // An entity of the module: what the lowering anchored. A name written after it is
            // the name of a member, and the language has no members yet.
            PathAnchor::Item(entity) => {
                match path.segments.first() {
                    Some(segment) => {
                        (None, vec![ResolveError::NestedName {
                            name: segment.name.clone(),
                        }])
                    },
                    None => (Some(entity), Vec::new()),
                }
            },
            // An entry of the import table: what the resolution resolved the import to.
            PathAnchor::Use(import) => {
                match resolution.imports().get(&import).cloned() {
                    // An import that resolved to nothing is what the resolution reported.
                    None => (None, Vec::new()),
                    Some(target) => self.continue_from(target, path, namespace),
                }
            },
            // A path rooted at a project: the names after the root are the modules of it, and
            // the walk of the whole path is the walk of those names.
            PathAnchor::Project(project) => {
                let project = project.or_else(|| self.project_of(module));
                let Some(project) = project else {
                    return (None, vec![ResolveError::NoProject]);
                };

                let names: Vec<Name> = path
                    .segments
                    .iter()
                    .map(|segment| segment.name.clone())
                    .collect();
                let mut seen = Vec::new();

                match self.walk_in(&project, &names, &mut seen) {
                    Ok(target) => take(&target, namespace, path.name()),
                    Err(error) => (None, vec![error]),
                }
            },
            // A binding, an entity of the body, a type variable: the stage that owns the body
            // answers for those itself. A name nothing resolved was reported by the lowering.
            PathAnchor::Binding(_)
            | PathAnchor::Local(_)
            | PathAnchor::TypeVar(_)
            | PathAnchor::Unresolved => (None, Vec::new()),
        }
    }

    /// The entity a path denotes, given what a walk found where the path stands.
    fn continue_from(
        &mut self,
        target: Target,
        path: &PathData,
        namespace: Namespace,
    ) -> (Option<EntityLoc>, Vec<ResolveError>) {
        if path.segments.is_empty() {
            return take(&target, namespace, path.name());
        }

        // A name written after an entity is the name of a member of it, and the language has no
        // members yet.
        if target.ty().is_some() || target.value().is_some() {
            return (None, vec![ResolveError::NestedName {
                name: path.segments[0].name.clone(),
            }]);
        }

        // A name written after a module is read inside it; a name written after nothing at all
        // is what the walk reported.
        let (Some((project, base)), Some(locator)) = (target.place(), target.module()) else {
            return (None, Vec::new());
        };

        let names: Vec<Name> = path
            .segments
            .iter()
            .map(|segment| segment.name.clone())
            .collect();
        let mut seen = Vec::new();

        match self.walk_after(project, base, locator, &names, &mut seen) {
            Ok(found) => take(&found, namespace, path.name()),
            Err(error) => (None, vec![error]),
        }
    }

    /// The module index of a project, if the walk was handed one.
    fn index(&self, project: &ProjectId) -> Option<&Arc<ModuleIndex>> {
        self.indexes.get(project)
    }

    /// Reads a path of a project's module index, and remembers what it found.
    fn read_module(&mut self, project: &ProjectId, segments: &[Name]) -> Option<ModuleId> {
        let found = self.index(project).and_then(|index| index.get(segments));

        self.reads.push(Read::Module {
            project: project.clone(),
            path: path_of(segments),
            found,
        });

        found
    }

    /// Reads whether modules of a project stand under a path, and remembers what it found.
    fn read_prefix(&mut self, project: &ProjectId, segments: &[Name]) -> bool {
        let found = self
            .index(project)
            .is_some_and(|index| index.has_prefix(segments));

        self.reads.push(Read::Prefix {
            project: project.clone(),
            path: path_of(segments),
            found,
        });

        found
    }

    /// The interface of a module, fetched through the walk's own map when it is not there.
    fn interface(&mut self, module: ModuleId) -> Option<Arc<Interface>> {
        if let Some(interface) = self.interfaces.get(&module) {
            return Some(interface.clone());
        }

        let interface = (self.fetch.as_mut()?)(module)?;
        self.interfaces.insert(module, interface.clone());

        Some(interface)
    }

    /// Walks a path as the module that wrote it wrote it: the root is a project or the name
    /// of a module of `context`, and the names after it are read inside what the root denotes.
    ///
    /// `context` is the project the path is read in: the project of the module the path
    /// belongs to, or the project of the module a re-export was written in. A path of no
    /// context is a path of a module of no project, which names the projects of the graph and
    /// no module of its own.
    ///
    /// A name that is no project of the module is read as the first name of a path inside
    /// `context`: a path of a module of the project the module is in ([ADR-0016]). The one name
    /// that is not read that way is the name the project is declared under, which no module of
    /// it may write --- it calls its own project by the keyword --- and which names no project
    /// of the module.
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn walk_plain(
        &mut self,
        context: Option<&ProjectId>,
        path: &PlainPath,
        seen: &mut Vec<(ModuleId, Name)>,
    ) -> Result<Target, ResolveError> {
        match path.root() {
            PathRoot::Project => {
                let Some(project) = context else {
                    return Err(ResolveError::NoProject);
                };

                self.walk_in(project, path.segments(), seen)
            },
            PathRoot::Named(name) => {
                if let Some(project) = self.project_named(context, name) {
                    return self.walk_in(&project, path.segments(), seen);
                }

                // The name is not one of a project: it starts a path inside the project the
                // module is in, or it is nothing at all. A name that names the project the
                // module is in is neither: a module calls its own project by the keyword and by
                // nothing else.
                let Some(project) = context else {
                    return Err(ResolveError::UnknownProject { name: name.clone() });
                };

                if *project == ProjectId::new(name.as_str()) {
                    return Err(ResolveError::UnknownProject { name: name.clone() });
                }

                let mut segments = Vec::with_capacity(path.segments().len() + 1);
                segments.push(name.clone());
                segments.extend(path.segments().iter().cloned());

                self.walk_in(project, &segments, seen)
            },
        }
    }

    /// Walks the names after a root, inside `project`.
    ///
    /// A path written with the keyword `project` or rooted at the name of a project is walked
    /// this way: the anchor the lowering gave the root already says which project it is.
    pub fn walk_in(
        &mut self,
        project: &ProjectId,
        segments: &[Name],
        seen: &mut Vec<(ModuleId, Name)>,
    ) -> Result<Target, ResolveError> {
        self.walk_names(project, Vec::new(), Place::Project, segments, seen)
    }

    /// Walks the names a path writes after a name the module bound: the names after a module,
    /// or after a prefix of module paths.
    ///
    /// The place a name denotes is where the names after it are read: a module holds names, and
    /// a prefix is what the modules under it stand on.
    pub fn walk_after(
        &mut self,
        project: &ProjectId,
        base: &[Name],
        locator: &ModuleLocator,
        segments: &[Name],
        seen: &mut Vec<(ModuleId, Name)>,
    ) -> Result<Target, ResolveError> {
        let place = match locator {
            ModuleLocator::Module(module) => Place::Module(*module),
            ModuleLocator::Prefix { .. } => Place::Prefix,
        };

        self.walk_names(project, base.to_vec(), place, segments, seen)
    }

    /// Walks the names of a path inside a project, from where the walk stands.
    ///
    /// The names are read as a module path, longest first: what names a module is read as one,
    /// and what is left is either the name the path ends at or a prefix of a module path. What
    /// the walk reads of the index are the entries under the place, and not the place itself:
    /// the place is what answered already, and the entries read are what the key of a resolution
    /// holds ([ADR-0009]).
    ///
    /// A name that denotes a module or a prefix is where the names after it are read, and a name
    /// that denotes an entity is where the walk ends: the names after an entity are the names of
    /// its members, and a member is not a name the text decides ([ADR-0016]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    fn walk_names(
        &mut self,
        project: &ProjectId,
        base: Vec<Name>,
        place: Place,
        segments: &[Name],
        seen: &mut Vec<(ModuleId, Name)>,
    ) -> Result<Target, ResolveError> {
        let mut project = project.clone();
        let mut base = base;
        let mut place = place;
        let mut segments = segments;

        loop {
            let Some((first, _)) = segments.split_first() else {
                // A path of no names names nothing: the parser reported what it could not read.
                return match place {
                    Place::Project => {
                        Err(ResolveError::UnknownModule {
                            project,
                            name: Name::missing(),
                        })
                    },
                    Place::Module(module) => Ok(Target::of_module(project, base, module)),
                    Place::Prefix => Ok(Target::prefix(project, base)),
                };
            };

            // What names a module is read as one, and the longest of them is what the names
            // mean. A module the walk stands in is a module of the path already, and so is the
            // shortest of them: what a name after it is, is what the loop below reads.
            let mut names = base.clone();
            let mut found = match place {
                Place::Module(module) => Some((0, module)),
                Place::Project | Place::Prefix => None,
            };

            for (index, segment) in segments.iter().enumerate() {
                names.push(segment.clone());

                if let Some(module) = self.read_module(&project, &names) {
                    found = Some((index + 1, module));
                }
            }

            if let Some((end, module)) = found {
                if end == segments.len() {
                    return Ok(Target::of_module(project, names, module));
                }

                if let Some(target) = self.name_in_module(module, &segments[end], seen)? {
                    // The name is the end of the path: what it denotes is what the path means.
                    if end + 1 == segments.len() {
                        return Ok(target);
                    }

                    // The path goes on: what follows a name is read where the name denotes.
                    let Some(locator) = target.module.clone() else {
                        // An entity, and the names after it are the names of its members:
                        // nothing the text decides, and nothing that is reported.
                        return Ok(target);
                    };
                    let Some((next, next_base)) = target.place.clone() else {
                        return Ok(target);
                    };

                    place = match locator {
                        ModuleLocator::Module(module) => Place::Module(module),
                        ModuleLocator::Prefix { .. } => Place::Prefix,
                    };
                    project = next;
                    base = next_base;
                    segments = &segments[end + 1..];
                    continue;
                }
            }

            // What is left names no name a module holds: the path may name a prefix of a module
            // path, which is what the names after it are read against.
            if self.read_prefix(&project, &names) {
                return Ok(Target::prefix(project, names));
            }

            let place_len = base.len();

            return Err(match found {
                // A module the path reached holds no such name.
                Some((end, module)) if end + 1 == segments.len() => {
                    ResolveError::UnknownName {
                        path: path_of(&names[..place_len + end]),
                        module,
                        name: segments[end].clone(),
                    }
                },
                // The name after the module a path reached names no module either.
                Some((end, _)) => {
                    ResolveError::UnknownModule {
                        project,
                        name: segments[end].clone(),
                    }
                },
                None => {
                    ResolveError::UnknownModule {
                        project,
                        name: first.clone(),
                    }
                },
            });
        }
    }

    /// Resolves the name a path ends at in the interface of `module`.
    ///
    /// A name a module re-exports is the path the module wrote, read in the project of that
    /// module; the walk continues there, and a chain that comes back to a name it already
    /// follows resolves to nothing.
    fn name_in_module(
        &mut self,
        module: ModuleId,
        name: &Name,
        seen: &mut Vec<(ModuleId, Name)>,
    ) -> Result<Option<Target>, ResolveError> {
        if seen
            .iter()
            .any(|(seen_module, seen_name)| *seen_module == module && seen_name == name)
        {
            return Err(ResolveError::CyclicImport { name: name.clone() });
        }

        // A module the input does not hold is a bug of the input function rather than a fact
        // about the module: what a bug there looks like is a name that resolves to nothing
        // ([ADR-0009]).
        let Some(interface) = self.interface(module) else {
            return Ok(None);
        };

        let Some(export) = interface.export(name) else {
            return Ok(None);
        };

        let mut target = Target::of_export(export);

        if let Some(path) = &export.reexport {
            seen.push((module, name.clone()));
            let context = self.project_of(module);
            let reexported = self.walk_plain(context.as_ref(), path, seen)?;
            seen.pop();

            target.fill(reexported);
        }

        Ok(Some(target))
    }

    /// The project a name at the root of a path names, if it names one.
    ///
    /// A name names a project when it is a project the context project depends on; a module of
    /// no project names the projects of the graph, which is all there is to declare a dependency
    /// on. What a module calls its own project by is the keyword, and not a name: a name the
    /// project is declared under is not one a module of it may write ([ADR-0016]).
    ///
    /// A path of the surface of the module the resolution is of is not read this way: the
    /// lowering of the module is handed the projects it may name, and which project a root of
    /// such a path names is an anchor the lowering set ([`crate::ResolveDeps`]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    fn project_named(&self, context: Option<&ProjectId>, name: &Name) -> Option<ProjectId> {
        let Some(project) = context else {
            // A module of no project may name every project there is, each of them by its own
            // name.
            let project = ProjectId::new(name.as_str());

            return self.graph.project(&project).map(|_| project);
        };

        self.graph.project(project)?.dependencies.get(name).cloned()
    }
}

/// What a target denotes in the namespace a place asks for.
fn take(
    target: &Target,
    namespace: Namespace,
    name: Name,
) -> (Option<EntityLoc>, Vec<ResolveError>) {
    let entity = match namespace {
        Namespace::Ty => target.ty(),
        Namespace::Value => target.value(),
        Namespace::Module => None,
    };

    match entity {
        Some(entity) => (Some(entity.clone()), Vec::new()),
        // A name that denotes nothing at all was reported by the resolution; one that denotes
        // something else --- a module, or an entity of the other namespace --- is reported here.
        None if target.is_empty() => (None, Vec::new()),
        None => {
            (None, vec![match namespace {
                Namespace::Value => ResolveError::NotAValue { name },
                _ => ResolveError::NotAType { name },
            }])
        },
    }
}

/// The path a walk read of an index, as a path of the project the index holds.
///
/// The path is rooted at the keyword `project`: a project is the outermost name of it, so a
/// path inside a project is written without one.
fn path_of(segments: &[Name]) -> PlainPathId {
    PlainPathId::new(PlainPath::from_root(PathRoot::Project, segments.to_vec()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mlkc_hir_def::{
        Attributes, ClassData, ClassLoc, EntityData, EntityLoc, ItemLoc, ItemLocLike,
        ItemSyntaxLoc, ItemTreeBuilder, ModuleId, ModuleScope, Name, Pat, PathRoot, PlainPath,
        PlainPathId, Prelude, ProjectGraph, UseLoc, Visibility, path::PathSegmentData,
    };
    use mlkc_lower::{lower_body, lower_module};
    use mlkc_parser::parse;
    use mlkc_syntax::ModuleRoot;
    use mlkc_vfs::{FileId, RelPathBuf};

    use super::{Namespace, PathAnchor, PathData, ResolveError, Target, Walk};
    use crate::{Closure, Resolution};

    /// The module the tests of the walk are written in.
    const MAIN: ModuleId = ModuleId(FileId::from_raw(0));

    /// A module of another project, where the classes of the tests are declared.
    const STD: ModuleId = ModuleId(FileId::from_raw(1));

    /// The surface of the module of the tests: one import, and a body that binds a name.
    const SOURCE: &str = "use project::std::Int\n\nfun main(x: Int): Int = x\n";

    /// The module of the tests, lowered.
    fn lowered() -> mlkc_lower::LoweredModule {
        let parsed = parse(SOURCE);
        let root = parsed.tree::<ModuleRoot>();
        let relative = RelPathBuf::try_from("main.mlk").expect("a relative path");

        lower_module(MAIN, &root, &Prelude::none(), &[], relative.as_path())
    }

    /// The class `Int` of the standard library of the tests.
    fn class() -> EntityLoc<ClassLoc> {
        let mut builder = ItemTreeBuilder::new(
            STD,
            PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("std")])),
        );
        builder.declare(
            Some(Name::new("Int")),
            EntityData::Class(ClassData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(0),
        );

        let tree = builder.finish();
        let (item, _) = tree.entities().next().expect("the class to be declared");

        EntityLoc {
            module: tree.module(),
            item: ClassLoc::try_from(item).expect("a class"),
        }
    }

    /// The import of the module of the tests.
    fn import() -> UseLoc {
        lowered()
            .item_tree
            .entities()
            .find_map(|(item, _)| {
                match item {
                    ItemLoc::Use(import) if item.name() == Some(&Name::new("Int")) => Some(import),
                    _ => None,
                }
            })
            .expect("the import to be declared")
    }

    /// The resolution of the module of the tests: the imports a test hands over.
    fn resolution(imports: &[(UseLoc, Target)]) -> Resolution {
        Resolution::new(
            Arc::new(ModuleScope::default()),
            imports.iter().cloned().collect(),
            Arc::from(Vec::new()),
        )
    }

    /// A path of one name, anchored the way a test anchors it.
    fn path(name: &str, anchor: PathAnchor) -> PathData {
        PathData::ident(Name::new(name), anchor)
    }

    #[test]
    fn a_name_the_lowering_anchored_is_answered_as_it_is() {
        let class = class();
        let path = path("Int", PathAnchor::Item(EntityLoc::from(class.clone())));
        let graph = ProjectGraph::default();
        let closure = Closure::default();
        let mut walk = Walk::of(&graph, &closure);

        let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Ty);

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(entity, Some(EntityLoc::from(class)));
    }

    #[test]
    fn a_name_written_after_an_entity_is_not_a_member_yet() {
        let class = class();
        let mut path = path("Int", PathAnchor::Item(EntityLoc::from(class)));
        path.segments.push(PathSegmentData {
            name: Name::new("field"),
            args: Vec::new(),
        });
        let graph = ProjectGraph::default();
        let closure = Closure::default();
        let mut walk = Walk::of(&graph, &closure);

        let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Ty);

        assert_eq!(entity, None);
        assert_eq!(errors, [ResolveError::NestedName {
            name: Name::new("field"),
        }],);
    }

    #[test]
    fn an_import_is_read_as_what_the_resolution_resolved() {
        let class = class();
        let import = import();
        let imports = [(
            import.clone(),
            Target::entity(Some(EntityLoc::from(class.clone())), None),
        )];
        let path = path("Int", PathAnchor::Use(import));
        let graph = ProjectGraph::default();
        let closure = Closure::default();
        let mut walk = Walk::of(&graph, &closure);

        let (entity, errors) = walk.entity_of(MAIN, &resolution(&imports), &path, Namespace::Ty);

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(entity, Some(EntityLoc::from(class)));

        // The same path read where a value belongs: a class is not a value.
        let (entity, errors) = walk.entity_of(MAIN, &resolution(&imports), &path, Namespace::Value);

        assert_eq!(entity, None);
        assert_eq!(errors, [ResolveError::NotAValue {
            name: Name::new("Int"),
        }],);
    }

    #[test]
    fn a_binding_is_answered_by_the_stage_that_owns_the_body() {
        let lowered = lowered();
        let decl = lowered.bodies.first().expect("a body to be declared");
        let body = lower_body(&lowered.item_tree, &decl.decl).expect("a body to be lowered");
        let pat = body.body.params()[0];

        assert!(matches!(body.body[pat], Pat::Bind(_)));

        let path = path("x", PathAnchor::Binding(pat));
        let graph = ProjectGraph::default();
        let closure = Closure::default();
        let mut walk = Walk::of(&graph, &closure);

        let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Value);

        assert_eq!(entity, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn a_path_of_a_module_of_no_project_is_unresolved() {
        let path = PathData {
            root: PathRoot::Project,
            root_args: Vec::new(),
            segments: vec![PathSegmentData {
                name: Name::new("core"),
                args: Vec::new(),
            }],
            anchor: PathAnchor::Project(None),
        };
        let graph = ProjectGraph::default();
        let closure = Closure::default();
        let mut walk = Walk::of(&graph, &closure);

        let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Ty);

        assert_eq!(entity, None);
        assert_eq!(errors, [ResolveError::NoProject]);
    }
}
