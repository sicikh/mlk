//! The projects the compiler knows, what each of them depends on, and which project a module
//! belongs to.

use std::{cmp::Ordering, collections::BTreeMap, fmt};

use indexmap::IndexMap;
use mlkc_intern::Interned;

use crate::{id::ModuleId, name::Name, path::PlainPathId, prelude::Prelude};

/// The name of a project: the path of its root, or the name it is declared under.
///
/// A project is named by its path and not by its index in the graph.
/// An entry of a scope may point at a virtual module of a dependency,
/// and a dependency added to the project must not move what that entry names ([ADR-0010]).
///
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(Interned<str>);

impl ProjectId {
    /// The name of a project.
    pub fn new(name: &str) -> Self {
        Self(Interned::new_str(name))
    }

    /// The name as it is written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Ord for ProjectId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for ProjectId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a name that denotes a module, or a prefix of a module path, points.
///
/// Two cases are worth telling apart.
///
/// A `use` that names a module of a project binds that module:
///
/// ```text
/// // module.mlk:  module project.main-module
/// // main.mlk:    use project.main-module
/// ```
///
/// and the entry of the scope is [`ModuleLocator::Module`],
/// because the path names a file the compiler has.
///
/// A `use` that names a *prefix* of a module path binds the prefix,
/// which is not a module and has no file of its own:
///
/// ```text
/// // module.mlk:  module project.data.main-module
/// // main.mlk:    use project.data
/// //
/// //              fun main() =
/// //                  data.main-module.start-app()
/// ```
///
/// and the entry is [`ModuleLocator::Prefix`].
/// The segments written after such a prefix are resolved by the stage that holds
/// the module paths a project declares: the path of the `use` and the segments of the
/// expression are walked against those declared paths until they reach a module,
/// and then an entity of that module.
///
/// The root of a path says which project the names after it are read in. The keyword `project`
/// is the project the module is written in --- what a module knows of itself without a manifest,
/// which is what makes it steadier than the name the project is declared under --- and the name
/// of another project is one the module depends on, as in `std.core`, the prefix `core` of the
/// project `std`. A module may name its own project the way the manifest does all the same, and
/// the two spellings of one prefix are for the stage that builds the entries to merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleLocator {
    /// A module that is a file of a project.
    Module(ModuleId),
    /// A prefix of a module path: a hierarchical name that is not a module itself.
    Prefix {
        /// The project whose module paths the prefix belongs to.
        project: ProjectId,
        /// The prefix as it is written, rooted at the keyword `project` or at the name of
        /// the project, so that it can be interned and compared like any other path.
        path: PlainPathId,
    },
}

/// What a project is: the module it starts from, what it depends on, and the imports every
/// module of it is given without writing them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectData {
    /// The module the project starts from.
    pub root_module: ModuleId,
    /// The dependencies, by the name each of them is declared under.
    pub dependencies: IndexMap<Name, ProjectId>,
    /// The prelude of the project: what every module of it is given without writing it
    /// ([ADR-0011]).
    ///
    /// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    pub prelude: Prelude,
}

impl ProjectData {
    /// A project of one module, which depends on nothing and gives its modules the prelude of
    /// the language.
    ///
    /// A caller that has more to say replaces the fields it has something to say about:
    ///
    /// ```
    /// use mlkc_hir_def::{ModuleId, Prelude, ProjectData};
    ///
    /// let project = ProjectData {
    ///     prelude: Prelude::none(),
    ///     ..ProjectData::new(ModuleId(mlkc_vfs::FileId::from_raw(0)))
    /// };
    /// ```
    pub fn new(root_module: ModuleId) -> Self {
        Self {
            root_module,
            dependencies: IndexMap::new(),
            prelude: Prelude::default(),
        }
    }
}

/// The projects the compiler knows, and the project each module belongs to.
///
/// The mapping of a module to its project is an index the loader keeps,
/// and it is cleared when a project goes away.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectGraph {
    projects: BTreeMap<ProjectId, ProjectData>,
    module_project: BTreeMap<ModuleId, ProjectId>,
}

impl ProjectGraph {
    /// What a project is, if the graph holds it.
    pub fn project(&self, id: &ProjectId) -> Option<&ProjectData> {
        self.projects.get(id)
    }

    /// Records a project, returning the one it replaces.
    ///
    /// The root module of a project is a module of it, so recording the project records the
    /// mapping of the module it starts from.
    pub fn insert(&mut self, id: ProjectId, data: ProjectData) -> Option<ProjectData> {
        self.module_project.insert(data.root_module, id.clone());
        self.projects.insert(id, data)
    }

    /// Drops a project and the mapping of its modules, returning what it was.
    pub fn remove(&mut self, id: &ProjectId) -> Option<ProjectData> {
        let removed = self.projects.remove(id);
        if removed.is_some() {
            self.module_project.retain(|_, project| project != id);
        }
        removed
    }

    /// Records which project a module belongs to.
    pub fn set_module_project(
        &mut self,
        module: ModuleId,
        project: ProjectId,
    ) -> Option<ProjectId> {
        self.module_project.insert(module, project)
    }

    /// The project a module belongs to, if the loader recorded one.
    pub fn project_of(&self, module: ModuleId) -> Option<&ProjectId> {
        self.module_project.get(&module)
    }

    /// The prelude of the project a module belongs to.
    ///
    /// A module that belongs to no project --- a file a host pushed on its own, which no
    /// manifest claimed --- is compiled with the prelude of the language ([`Prelude::standard`]).
    pub fn prelude_of(&self, module: ModuleId) -> &Prelude {
        self.project_of(module)
            .and_then(|project| self.projects.get(project))
            .map_or(Prelude::standard(), |data| &data.prelude)
    }

    /// The projects of the graph, in the order of their names.
    pub fn projects(&self) -> impl Iterator<Item = (&ProjectId, &ProjectData)> {
        self.projects.iter()
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;
    use crate::path::PlainPath;

    fn module(index: u32) -> ModuleId {
        ModuleId(FileId::from_raw(index))
    }

    fn project() -> ProjectId {
        ProjectId::new("the-project")
    }

    #[test]
    fn a_project_is_named_by_its_path() {
        assert_eq!(ProjectId::new("a").to_string(), "a");
        assert_eq!(ProjectId::new("a"), ProjectId::new("a"));
        assert!(ProjectId::new("a") < ProjectId::new("b"));
    }

    #[test]
    fn a_project_is_found_by_its_name_and_by_its_modules() {
        let mut graph = ProjectGraph::default();
        graph.insert(project(), ProjectData::new(module(0)));
        graph.set_module_project(module(1), project());

        assert_eq!(
            graph.project(&project()).map(|data| data.root_module),
            Some(module(0))
        );
        // The module a project starts from is a module of it.
        assert_eq!(graph.project_of(module(0)), Some(&project()));
        assert_eq!(graph.project_of(module(1)), Some(&project()));
        assert_eq!(graph.project_of(module(2)), None);
    }

    #[test]
    fn dropping_a_project_drops_the_mapping_of_its_modules() {
        let mut graph = ProjectGraph::default();
        graph.insert(project(), ProjectData::new(module(0)));
        graph.set_module_project(module(1), project());

        assert!(graph.remove(&project()).is_some());
        assert_eq!(graph.project_of(module(0)), None);
        assert_eq!(graph.project_of(module(1)), None);
    }

    #[test]
    fn a_module_takes_the_prelude_of_its_project() {
        let mut graph = ProjectGraph::default();
        let prelude = Prelude::from_paths([PlainPath::from_segments([
            Name::new("project"),
            Name::new("core"),
            Name::new("Int"),
        ])]);

        graph.insert(project(), ProjectData {
            prelude: prelude.clone(),
            ..ProjectData::new(module(0))
        });

        assert_eq!(graph.prelude_of(module(0)), &prelude);
        // A module no project claims is compiled with the prelude of the language.
        assert_eq!(graph.prelude_of(module(1)), Prelude::standard());
    }

    #[test]
    fn a_module_of_a_project_the_graph_does_not_hold_takes_the_prelude_of_the_language() {
        let mut graph = ProjectGraph::default();
        graph.set_module_project(module(0), project());

        assert_eq!(graph.prelude_of(module(0)), Prelude::standard());
    }
}
