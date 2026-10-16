//! The import graph: the modules a module's names reach, and where each of them is named.
//!
//! A name is read inside the module it lands in, and that module is what a program has to
//! instantiate before the one that named it: the modules of a program, with an edge from a
//! module to every module its names reach, form a graph whose cycles are a mistake of the
//! source ([ADR-0016]).
//!
//! What a name reaches is the module the name *belongs to*, and not the module a path was
//! written through: `use std::prelude::Int` reaches `std::core`, which is where `Int` is
//! declared, and `use project::types::utils::Point` reaches `project::types::utils`, which is
//! where `Point` stands. A re-export is transparent this way, which is what the back end sees
//! as well: the module a value is imported from is the module that provides it
//! ([ADR-0021]). What a module names of *itself* is not an edge: a body may read anything the
//! module declares, and nothing has to be read before it.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//! [ADR-0021]: ../../docs/adr/0021-translation-units.md

use std::collections::BTreeMap;

use mlkc_hir_def::{
    Body, BodyEntityLoc, EntityData, ItemLoc, ItemTree, ModuleId, ModuleLocator, Name, PathAnchor,
    PathData, PathId, Resolution, Target, TypeRef, UseLoc, Walk, dump::TypePlace,
    path::PathSegmentData,
};

use crate::{ResolveDeps, resolve::segment_names};

/// The modules one module's names reach, and where each of them is named.
///
/// The names are every path the module writes: the imports of its item tree, the types its
/// declarations write, and the paths of its bodies. The place kept for a module is where it is
/// named *first*, in the order the module is read in: what a diagnostic about a cycle points
/// at is a name the module wrote, and one place of it is enough.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamedModules {
    named: BTreeMap<ModuleId, NamedPlace>,
}

/// Where a module is named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedPlace {
    /// An import of the module: `use project::other::g`.
    ///
    /// An import the module did not write --- one the prelude brought in --- is named this way
    /// as well, and the driver renders it without a place: there is no syntax to point at
    /// ([ADR-0011]).
    ///
    /// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    Import(UseLoc),
    /// A type a declaration writes, at the place the declaration writes it.
    Declared {
        /// The declaration that writes the type.
        item: ItemLoc,
        /// Which type of the declaration it is, when the declaration writes more than one.
        place: Option<TypePlace>,
    },
    /// A path of one body, by the body that writes it and the path inside it.
    Body {
        /// The entity whose body names the module.
        owner: BodyEntityLoc,
        /// The path inside that body.
        path: PathId,
    },
}

impl NamedModules {
    /// The modules the names reach, each with where it is named first.
    pub fn iter(&self) -> impl Iterator<Item = (ModuleId, &NamedPlace)> {
        self.named.iter().map(|(module, place)| (*module, place))
    }

    /// Where the name that reaches `module` is written, if a name reaches it.
    pub fn get(&self, module: ModuleId) -> Option<&NamedPlace> {
        self.named.get(&module)
    }

    /// How many modules the names reach.
    pub fn len(&self) -> usize {
        self.named.len()
    }

    /// Whether the names reach no other module at all.
    pub fn is_empty(&self) -> bool {
        self.named.is_empty()
    }
}

/// The modules the names of one module reach, gathered by walking its paths.
///
/// The walks read what a resolution reads, and what a check reads: the closure handed over is
/// the wider one, which holds the interfaces and the index entries of the paths of the bodies
/// too ([`crate::closure::of_check`]). Nothing is reported here: what a name could not reach is
/// what the resolution and the check report, and this reads the same names to find where they
/// land.
pub fn named_modules<'a>(
    module: ModuleId,
    tree: &ItemTree,
    bodies: impl IntoIterator<Item = (BodyEntityLoc, &'a Body)>,
    resolution: &Resolution,
    deps: &ResolveDeps,
) -> NamedModules {
    let mut walk = Walk::of(&deps.graph, &deps.closure);
    let mut names = Names::new(module);

    // An import names what it resolved to: the module of the entity, or the module itself.
    for (import, target) in resolution.imports() {
        names.reach(target, NamedPlace::Import(import.clone()));
    }

    // A type a declaration writes names the modules it reaches.
    for (item, id) in tree.entities() {
        for (place, ty) in declared_types(tree, id) {
            names.ty(&mut walk, resolution, ty, &|| {
                NamedPlace::Declared {
                    item: item.clone(),
                    place,
                }
            });
        }
    }

    // And the paths of the bodies do the same.
    for (owner, body) in bodies {
        for (path, data) in body.paths().iter() {
            names.path(&mut walk, resolution, data, &|| {
                NamedPlace::Body {
                    owner: owner.clone(),
                    path,
                }
            });
        }
    }

    names.finish()
}

/// The types one declaration writes, each with the place among them it is written at.
///
/// The places are the ones the resolution reads the declarations by, so a module that names
/// another through a type is pointed at exactly where the resolution would point ([`surface`]).
///
/// [`surface`]: crate::resolve
fn declared_types(
    tree: &ItemTree,
    id: mlkc_hir_def::ModuleDefId,
) -> Vec<(Option<TypePlace>, &TypeRef)> {
    match tree.entity(id).data() {
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
            data.ty_class
                .iter()
                .chain(data.ty.iter())
                .map(|ty| (None, ty))
                .collect()
        },
        EntityData::Adt(_) | EntityData::Use(_) => Vec::new(),
    }
}

/// The names reached, gathered while the paths of one module are walked.
struct Names {
    /// The module the names are read in, which names of itself are no edge.
    module: ModuleId,
    /// The modules reached, with where each is named first.
    named: BTreeMap<ModuleId, NamedPlace>,
}

impl Names {
    fn new(module: ModuleId) -> Self {
        Self {
            module,
            named: BTreeMap::new(),
        }
    }

    fn finish(self) -> NamedModules {
        NamedModules { named: self.named }
    }

    /// Records the module a target belongs to, at the place it is named.
    ///
    /// A target that denotes a prefix of module paths names no module: what the names under a
    /// prefix stand on are the modules themselves.
    fn reach(&mut self, target: &Target, place: NamedPlace) {
        let Some(module) = module_of(target) else {
            return;
        };

        // A module that names itself is not a cycle: what a module declares is read by its own
        // bodies, and nothing has to stand before it.
        if module == self.module {
            return;
        }

        self.named.entry(module).or_insert(place);
    }

    /// Records the modules a type reaches.
    fn ty(
        &mut self,
        walk: &mut Walk<'_>,
        resolution: &Resolution,
        ty: &TypeRef,
        place: &dyn Fn() -> NamedPlace,
    ) {
        match ty {
            TypeRef::Path(path) => self.path(walk, resolution, path, place),
            TypeRef::Infer | TypeRef::Missing => {},
        }
    }

    /// Records the modules a path reaches.
    fn path(
        &mut self,
        walk: &mut Walk<'_>,
        resolution: &Resolution,
        path: &PathData,
        place: &dyn Fn() -> NamedPlace,
    ) {
        // The arguments written at a path are types, wherever the path itself is written.
        for arg in &path.root_args {
            self.ty(walk, resolution, arg, place);
        }

        for segment in &path.segments {
            for arg in &segment.args {
                self.ty(walk, resolution, arg, place);
            }
        }

        match &path.anchor {
            // A name of the module, a binding of a body, or a type variable: nothing of another
            // module is named.
            PathAnchor::Item(_)
            | PathAnchor::Local(_)
            | PathAnchor::TypeVar(_)
            | PathAnchor::Binding(_)
            | PathAnchor::Unresolved => {},
            // The base is an entry of the import table: the import itself reaches what it
            // resolved to, and what is written after the base is read inside that.
            PathAnchor::Use(import) => {
                let Some(target) = resolution.imports().get(import).cloned() else {
                    return;
                };

                self.reach(&target, NamedPlace::Import(import.clone()));

                if !path.segments.is_empty() {
                    self.under(walk, &target, &path.segments, place);
                }
            },
            // The names after the root are read among the modules of the project.
            PathAnchor::Project(project) => {
                let project = project.clone().or_else(|| walk.project_of(self.module));

                let Some(project) = project else {
                    return;
                };

                let names = segment_names(&path.segments);
                let reached = walked(walk, &project, &names);

                self.reach(&reached, place());
            },
        }
    }

    /// Records the modules the names after a base reach.
    fn under(
        &mut self,
        walk: &mut Walk<'_>,
        target: &Target,
        segments: &[PathSegmentData],
        place: &dyn Fn() -> NamedPlace,
    ) {
        // Reaching an entity is the end of a walk: the names after it are the names of its
        // members, and the language has no members yet.
        if target.ty().is_some() || target.value().is_some() {
            return;
        }

        let (Some((project, base)), Some(locator)) = (target.place(), target.module()) else {
            return;
        };

        let names = segment_names(segments);
        let mut seen = Vec::new();
        let reached = walk
            .walk_after(project, base, locator, &names, &mut seen)
            .unwrap_or_default();

        self.reach(&reached, place());
    }
}

/// Walks the names of a path rooted at a project, and answers with what they reached.
fn walked(walk: &mut Walk<'_>, project: &mlkc_hir_def::ProjectId, names: &[Name]) -> Target {
    let mut seen = Vec::new();

    walk.walk_in(project, names, &mut seen).unwrap_or_default()
}

/// The module a target belongs to: the module of the entity, or the module a locator names.
///
/// A prefix of module paths is not a module: what the names under a prefix stand on are the
/// modules themselves, and a target that denotes one names no module of its own.
fn module_of(target: &Target) -> Option<ModuleId> {
    if let Some(entity) = target.ty().or_else(|| target.value()) {
        return Some(entity.module);
    }

    match target.module()? {
        ModuleLocator::Module(module) => Some(*module),
        ModuleLocator::Prefix { .. } => None,
    }
}
