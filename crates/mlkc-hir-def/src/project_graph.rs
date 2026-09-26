//! The projects the compiler knows, what each of them depends on,
//! and which project a module belongs to.

use std::{cmp::Ordering, collections::BTreeMap, fmt};

use indexmap::IndexMap;
use mlkc_intern::Interned;

use crate::{id::ModuleId, name::Name, path::PlainPathId};

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

/// What a project is: the module it starts from, and what it depends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectData {
    /// The module the project starts from.
    pub root_module: ModuleId,
    /// The dependencies, by the name each of them is declared under.
    pub dependencies: IndexMap<Name, ProjectId>,
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
    pub fn insert(&mut self, id: ProjectId, data: ProjectData) -> Option<ProjectData> {
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

    /// The projects of the graph, in the order of their names.
    pub fn projects(&self) -> impl Iterator<Item = (&ProjectId, &ProjectData)> {
        self.projects.iter()
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;

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
        graph.insert(project(), ProjectData {
            root_module: module(0),
            dependencies: IndexMap::new(),
        });
        graph.set_module_project(module(1), project());

        assert_eq!(
            graph.project(&project()).map(|data| data.root_module),
            Some(module(0))
        );
        assert_eq!(graph.project_of(module(1)), Some(&project()));
        assert_eq!(graph.project_of(module(2)), None);
    }

    #[test]
    fn dropping_a_project_drops_the_mapping_of_its_modules() {
        let mut graph = ProjectGraph::default();
        graph.insert(project(), ProjectData {
            root_module: module(0),
            dependencies: IndexMap::new(),
        });
        graph.set_module_project(module(1), project());

        assert!(graph.remove(&project()).is_some());
        assert_eq!(graph.project_of(module(1)), None);
    }
}
