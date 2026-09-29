//! The resolution of one module: what its names denote, and what the walk found wrong.

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{
    EntityData, EntityLoc, ItemLoc, ItemLocLike, ItemTree, LocalTarget, ModuleId, ModuleScope,
    Name, Namespace, PathAnchor, PathData, PerNs, PlainPathId, ProjectGraph, ProjectId, TypeRef,
    UseData, UseLoc, Visibility, dump::TypePlace, path::PathSegmentData,
};

use crate::{
    Closure,
    diagnostic::{ResolveDiag, ResolveError, ResolvePlace},
    walk::{Target, Walk},
};

/// What resolving a module reads of the rest of the project ([ADR-0009]).
///
/// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
#[derive(Debug, Clone)]
pub struct ResolveDeps {
    /// The projects, what each of them depends on, and the project of every module.
    pub graph: Arc<ProjectGraph>,
    /// The modules the walk reached, and the entries of the module indexes it read.
    pub closure: Closure,
}

/// The error a walk should have reported, when the name it could not find is a name the module
/// keeps to itself ([ADR-0016]).
///
/// The interface of a module holds the names it shows, so a name it holds and does not show ---
/// a declaration that is not `pub`, or an import it does not re-export --- is not in it, and a
/// walk that ends at the name finds nothing. Telling a reader why is a look at the module
/// itself: its item tree is what its interface is a projection of, and this is the only thing
/// read of a module beside the interface.
///
/// The look is a value of its own, and it is taken only when a walk has already failed: what a
/// resolution that resolved cleanly read is the interface and the entries of the indexes, and
/// its key is not widened by this ([ADR-0008], [ADR-0009]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
/// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
pub fn hidden_name(
    path: &PlainPathId,
    module: ModuleId,
    name: &Name,
    tree: &ItemTree,
) -> Option<ResolveError> {
    let entry = tree.scope().get(name)?;

    // The walk found no name in the interface of the module, so a name the module holds --- a
    // name of the module, or an import of it --- is a name it does not show.
    if entry.is_none() {
        return None;
    }

    Some(ResolveError::HiddenName {
        path: path.clone(),
        module,
        name: name.clone(),
    })
}

/// What resolving a module produced: the names of the module, and what the walk found wrong.
///
/// The pass answers with the two parts ([`resolve_module`]); this is the two of them as one
/// value, which is what a driver keeps per module and compares with the one it held ([ADR-0008]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// What each name of the module denotes: a declaration of it, or what an import resolved to.
    scope: Arc<ModuleScope>,
    /// What the walk found, in the order of the module.
    diagnostics: Arc<[ResolveDiag]>,
}

impl Resolution {
    /// A resolution of the scope a module resolved to, with what its walk found wrong.
    pub fn new(scope: Arc<ModuleScope>, diagnostics: Arc<[ResolveDiag]>) -> Self {
        Self { scope, diagnostics }
    }

    /// What each name of the module denotes.
    pub fn scope(&self) -> &Arc<ModuleScope> {
        &self.scope
    }

    /// What the walk found, in the order of the module.
    pub fn diagnostics(&self) -> &Arc<[ResolveDiag]> {
        &self.diagnostics
    }
}

/// Resolves the names of a module: its import table, and the paths its own surface writes.
///
/// A module is resolved against the interfaces its own paths name ([ADR-0016]): the paths of a
/// body are not resolved here, since a body is a unit of its own and the stage that checks it
/// resolves its paths against the same scope ([ADR-0005]).
///
/// [ADR-0005]: ../../docs/adr/0005-compiler-pipeline.md
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
pub fn resolve_module(
    module: ModuleId,
    tree: &ItemTree,
    deps: &ResolveDeps,
) -> (ModuleScope, Vec<ResolveDiag>) {
    let mut walk = Walk::of(&deps.graph, &deps.closure);
    let mut resolver = Resolver::new(module, tree, &mut walk);
    resolver.run();

    (resolver.scope(), resolver.diagnostics)
}

/// Walks every path of a module, gathering what the walk reads.
///
/// The closure of a resolution is what a resolution reads, gathered by making the walk once:
/// the pass that follows makes the same walk over the value it was handed, so the two agree by
/// construction ([ADR-0009]).
///
/// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
pub(crate) fn walk_all<'a>(module: ModuleId, tree: &'a ItemTree, walk: &mut Walk<'a>) {
    let mut resolver = Resolver::new(module, tree, walk);
    resolver.run();
}

/// One module, as it is resolved.
struct Resolver<'w, 'a> {
    /// The module being resolved.
    module: ModuleId,
    /// The surface of that module.
    tree: &'a ItemTree,
    /// The tables the walk reads.
    walk: &'w mut Walk<'a>,
    /// What each import of the module resolved to.
    imports: BTreeMap<UseLoc, Target>,
    /// What the walk found wrong, in the order of the module.
    diagnostics: Vec<ResolveDiag>,
}

impl<'w, 'a> Resolver<'w, 'a> {
    /// A resolution of `module` over the tables a walk reads.
    fn new(module: ModuleId, tree: &'a ItemTree, walk: &'w mut Walk<'a>) -> Self {
        Self {
            module,
            tree,
            walk,
            imports: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Resolves the module: its import table, and then the paths of its surface.
    fn run(&mut self) {
        self.imports();
        self.surface();
    }

    /// Resolves the imports of the module, and reports the ones that name nothing.
    ///
    /// An import the module did not write --- one the prelude brought in --- is not reported:
    /// it has no place in the module to point at ([ADR-0011]). A use of a name it would have
    /// brought in is reported where the module wrote the use.
    ///
    /// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    fn imports(&mut self) {
        let tree = self.tree;
        let context = self.walk.project_of(self.module);

        for (item, _) in tree.entities() {
            let ItemLoc::Use(import) = item.clone() else {
                continue;
            };

            // A name that is not there is not a name a module has, and an import of nothing is
            // not an import: nothing is recorded, and nothing is resolved.
            if item.name().is_none_or(Name::is_missing) {
                continue;
            }

            let Some(EntityData::Use(data)) = tree.entity_data(item.clone()) else {
                continue;
            };

            let mut seen = Vec::new();
            let resolved = match self
                .walk
                .walk_plain(context.as_ref(), &data.path, &mut seen)
            {
                Ok(target) => target,
                Err(error) => {
                    if tree.syntax_loc(item.clone()).is_some() {
                        self.report(error, ResolvePlace::new(item.clone(), None));
                    }

                    Target::default()
                },
            };

            self.imports.insert(import, resolved);
        }
    }

    /// Walks the paths the surface of the module writes: the types of its entities.
    fn surface(&mut self) {
        let tree = self.tree;

        for (item, id) in tree.entities() {
            let types: Vec<(Option<TypePlace>, &'a TypeRef)> = match tree.entity(id).data() {
                EntityData::Function(data) => {
                    data.signature
                        .params
                        .iter()
                        .enumerate()
                        .filter_map(|(index, parameter)| {
                            Some((Some(TypePlace::Parameter(index)), parameter.ty.as_ref()?))
                        })
                        .chain(
                            data.signature
                                .ret
                                .iter()
                                .map(|ty| (Some(TypePlace::Result), ty)),
                        )
                        .collect()
                },
                EntityData::Const(data) => data.ty.iter().map(|ty| (None, ty)).collect(),
                EntityData::Impl(data) => {
                    data.class
                        .iter()
                        .chain(data.ty.iter())
                        .map(|ty| (None, ty))
                        .collect()
                },
                EntityData::Class(_) | EntityData::Value(_) | EntityData::Use(_) => Vec::new(),
            };

            for (place, ty) in types {
                self.check_type(&item, place, ty);
            }
        }
    }

    /// Checks that a type the surface writes denotes something, and so do the types it carries.
    fn check_type(&mut self, item: &ItemLoc, place: Option<TypePlace>, ty: &'a TypeRef) {
        match ty {
            TypeRef::Path(path) => self.check_path(item, place, path),
            TypeRef::Infer | TypeRef::Missing => {},
        }
    }

    /// Checks that a path a type writes denotes something.
    fn check_path(&mut self, item: &ItemLoc, place: Option<TypePlace>, path: &'a PathData) {
        // The arguments written at a path are types, wherever the path itself is written.
        for arg in &path.root_args {
            self.check_type(item, place, arg);
        }

        for segment in &path.segments {
            for arg in &segment.args {
                self.check_type(item, place, arg);
            }
        }

        match &path.anchor {
            // A name of the module, a binding of a body, or a type variable: the path denotes
            // something the module itself resolved. What follows it is a member of what it
            // denotes, and the language has no members yet: what a name after a class is is
            // what a type decides, and that naming is a record of its own.
            PathAnchor::Item(_)
            | PathAnchor::Local(_)
            | PathAnchor::TypeVar(_)
            | PathAnchor::Binding(_) => {},
            // The base is an entry of the import table: the path denotes what the import
            // resolved to, and the names after it are read inside that.
            PathAnchor::Use(import) => {
                let target = self.imports.get(import).cloned().unwrap_or_default();

                if target.is_empty() {
                    if let Some(name) = path.root.name() {
                        let error = ResolveError::UnresolvedName { name: name.clone() };
                        self.report(error, ResolvePlace::new(item.clone(), place));
                    }
                } else if !path.segments.is_empty() {
                    self.check_under(item, place, &target, &path.segments);
                }
            },
            // A project: the names after the root are read among the modules of it. The
            // lowering anchored the name at the root to the project it names, and the keyword
            // is the project the module is written in, which the graph says ([ADR-0016]).
            //
            // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
            PathAnchor::Project(project) => {
                let project = project
                    .clone()
                    .or_else(|| self.walk.project_of(self.module));

                let Some(project) = project else {
                    let error = ResolveError::NoProject;
                    self.report(error, ResolvePlace::new(item.clone(), place));
                    return;
                };

                self.check_project(&project, item, place, path);
            },
            // A name the lowering could not resolve is a name the module knows in another
            // namespace, which is a value where a type belongs. A name the module knows nothing
            // about is one it has already been told about where it is written ([ADR-0016]).
            //
            // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
            PathAnchor::Unresolved => {
                let Some(name) = path.root.name() else {
                    return;
                };

                if self
                    .tree
                    .scope()
                    .get(name)
                    .is_some_and(|entry| !entry.is_none())
                {
                    let error = ResolveError::NotAType { name: name.clone() };
                    self.report(error, ResolvePlace::new(item.clone(), place));
                }
            },
        }
    }

    /// Checks that a path rooted at a project denotes something: the names after the root are
    /// read among the modules of `project` ([ADR-0016]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    fn check_project(
        &mut self,
        project: &ProjectId,
        item: &ItemLoc,
        place: Option<TypePlace>,
        path: &'a PathData,
    ) {
        let names = segment_names(&path.segments);
        let mut seen = Vec::new();

        if let Err(error) = self.walk.walk_in(project, &names, &mut seen) {
            self.report(error, ResolvePlace::new(item.clone(), place));
        }
    }

    /// Checks the names a path writes after a base an import resolved to.
    fn check_under(
        &mut self,
        item: &ItemLoc,
        place: Option<TypePlace>,
        target: &Target,
        segments: &[PathSegmentData],
    ) {
        // Reaching an entity is the end of a walk: the names after it are the names of its
        // members, and the language has no members yet.
        if target.ty.is_some() || target.value.is_some() {
            return;
        }

        // What a path continues from is what a module or a prefix of module paths denotes, and
        // the names after it are read where it stands.
        let (Some((project, base)), Some(locator)) = (&target.place, &target.module) else {
            return;
        };

        let mut seen = Vec::new();
        let names = segment_names(segments);

        if let Err(error) = self
            .walk
            .walk_after(project, base, locator, &names, &mut seen)
        {
            self.report(error, ResolvePlace::new(item.clone(), place));
        }
    }

    /// The scope of the module: what each of its names denotes, once the modules it names are
    /// read.
    ///
    /// A name the module declares denotes its own entity, in the namespaces of its kind, with
    /// the visibility the declaration gave it. A name an import brings in denotes what the
    /// path resolved to, in a namespace the module declares nothing in, with the visibility of
    /// the import: what the name reaches is what the module that holds it wrote. A name
    /// nothing resolved to has no entry, and what a reader is told about is the diagnostic.
    fn scope(&self) -> ModuleScope {
        let mut scope = ModuleScope::default();

        for (name, entry) in self.tree.scope().iter() {
            let mut per_ns = PerNs::default();

            for &namespace in &[Namespace::Ty, Namespace::Value, Namespace::Module] {
                let Some(LocalTarget::Item(entity)) = entry.get(namespace) else {
                    continue;
                };

                let visibility = self.visibility(entity);

                match namespace {
                    Namespace::Ty => per_ns.ty = Some((entity.clone(), visibility)),
                    Namespace::Value => per_ns.value = Some((entity.clone(), visibility)),
                    Namespace::Module => {},
                }
            }

            if let Some(import) = &entry.import
                && let Some(target) = self.imports.get(import)
            {
                let visibility = self
                    .import_data(import)
                    .map_or(Visibility::Private, |data| data.visibility);

                fill(&mut per_ns, target, visibility);
            }

            if !per_ns.is_none() {
                scope.insert(name.clone(), per_ns);
            }
        }

        scope
    }

    /// How far an entity of the module reaches, as its declaration wrote it.
    fn visibility(&self, entity: &EntityLoc) -> Visibility {
        self.tree
            .entity_data(entity.item.clone())
            .and_then(EntityData::visibility)
            .unwrap_or(Visibility::Private)
    }

    /// What the module wrote about an import.
    fn import_data(&self, import: &UseLoc) -> Option<&'a UseData> {
        match self.tree.entity_data(ItemLoc::Use(import.clone()))? {
            EntityData::Use(data) => Some(data),
            _ => None,
        }
    }

    /// Records what the walk found wrong.
    fn report(&mut self, error: ResolveError, place: ResolvePlace) {
        self.diagnostics.push(ResolveDiag::new(error, place));
    }
}

/// Fills what an import brings in, in the namespaces the module declares nothing in.
fn fill(per_ns: &mut PerNs, target: &Target, visibility: Visibility) {
    if let Some(entity) = &target.ty
        && per_ns.ty.is_none()
    {
        per_ns.ty = Some((entity.clone(), visibility));
    }

    if let Some(entity) = &target.value
        && per_ns.value.is_none()
    {
        per_ns.value = Some((entity.clone(), visibility));
    }

    if let Some(module) = &target.module
        && per_ns.module.is_none()
    {
        per_ns.module = Some((module.clone(), visibility));
    }
}

/// The names of the segments of a path, in the order they are written.
fn segment_names(segments: &[PathSegmentData]) -> Vec<Name> {
    segments
        .iter()
        .map(|segment| segment.name.clone())
        .collect()
}
