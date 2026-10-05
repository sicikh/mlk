//! Which module of a project a path names.

use std::collections::BTreeMap;

use crate::{
    id::ModuleId,
    name::Name,
    path::{PathRoot, PlainPath},
    project_graph::ProjectId,
};

/// The modules of one project, by the path each of them is called by.
///
/// An index, not a value: a module contributes its own entry, a change to the place of a module
/// replaces that entry, and a consumer is keyed by the entries it read ([ADR-0008]).
///
/// The paths are the ones the modules' interfaces record, read inside the project: a path rooted
/// at the keyword `project`, which is the only way a module of the project calls it ([ADR-0016]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleIndex {
    /// The project whose modules the index holds.
    project: ProjectId,
    /// The modules, in the order of their paths.
    paths: BTreeMap<Vec<Name>, ModuleId>,
}

impl ModuleIndex {
    /// An index of the modules of `project`.
    pub fn new(project: ProjectId) -> Self {
        Self {
            project,
            paths: BTreeMap::new(),
        }
    }

    /// The project whose modules the index holds.
    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    /// Records `module` under the path its interface records.
    ///
    /// A path rooted at a name records nothing and returns `None`: what a module calls its own
    /// project by is the keyword, and a path rooted at a name is a path of the project that name
    /// is ([ADR-0016]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn insert(&mut self, module: ModuleId, path: &PlainPath) -> Option<ModuleId> {
        let segments = self.segments(path)?;
        self.paths.insert(segments, module)
    }

    /// The module the path `segments` names, if the project holds one.
    pub fn get(&self, segments: &[Name]) -> Option<ModuleId> {
        self.paths.get(segments).copied()
    }

    /// Whether any module of the project stands under `segments`.
    ///
    /// A module path may be a prefix of another one --- `project::data` and
    /// `project::data::utils` are two modules --- and a name that is no module is a prefix of
    /// one when this answers `true`: the name a path may be continued from.
    pub fn has_prefix(&self, segments: &[Name]) -> bool {
        self.under(segments).next().is_some()
    }

    /// The modules that stand under `segments`, in the order of their paths.
    pub fn under<'a>(
        &'a self,
        segments: &'a [Name],
    ) -> impl Iterator<Item = (&'a [Name], ModuleId)> + 'a {
        self.paths
            .range(segments.to_vec()..)
            .take_while(move |(path, _)| path.starts_with(segments))
            .map(|(path, module)| (path.as_slice(), *module))
    }

    /// The modules of the project, in the order of their paths.
    pub fn iter(&self) -> impl Iterator<Item = (&[Name], ModuleId)> {
        self.paths
            .iter()
            .map(|(path, module)| (path.as_slice(), *module))
    }

    /// How many modules the index holds.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Whether the project holds no module at all.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// The segments of a path, if the path is a path of this project.
    ///
    /// A module calls the project it is in by the keyword and by nothing else: a path rooted at
    /// a name is a path of the project that name is, and of no other ([ADR-0016]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    fn segments(&self, path: &PlainPath) -> Option<Vec<Name>> {
        match path.root() {
            PathRoot::Project => Some(path.segments().to_vec()),
            PathRoot::Named(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::{ModuleId, ModuleIndex, Name, PathRoot, PlainPath, ProjectId};
    use crate::path::PlainPathId;

    fn module(index: u32) -> ModuleId {
        ModuleId(FileId::from_raw(index))
    }

    fn project() -> ProjectId {
        ProjectId::new("the-project")
    }

    fn path(segments: &[&str]) -> PlainPath {
        PlainPath::from_root(
            PathRoot::Project,
            segments.iter().map(|name| Name::new(name)),
        )
    }

    fn index() -> ModuleIndex {
        let mut index = ModuleIndex::new(project());

        index.insert(module(0), &path(&["data"]));
        index.insert(module(1), &path(&["data", "utils"]));
        index.insert(module(2), &path(&["main"]));

        index
    }

    #[test]
    fn a_module_is_found_by_the_path_it_is_called_by() {
        let index = index();

        assert_eq!(index.get(&[Name::new("main")]), Some(module(2)));
        assert_eq!(
            index.get(&[Name::new("data"), Name::new("utils")]),
            Some(module(1))
        );
        assert_eq!(index.get(&[Name::new("nope")]), None);
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn a_prefix_of_a_module_path_is_what_the_modules_under_it_stand_on() {
        let index = index();

        assert!(index.has_prefix(&[Name::new("data")]));
        assert!(index.has_prefix(&[Name::new("data"), Name::new("utils")]));
        assert!(!index.has_prefix(&[Name::new("data"), Name::new("missing")]));
        assert!(!index.has_prefix(&[Name::new("nope")]));

        let under: Vec<String> = index
            .under(&[Name::new("data")])
            .map(|(path, _)| {
                path.iter()
                    .map(|name| name.as_str().to_owned())
                    .collect::<Vec<_>>()
                    .join("::")
            })
            .collect();

        assert_eq!(under, ["data", "data::utils"]);
    }

    #[test]
    fn a_project_is_not_named_by_its_own_name() {
        let mut index = ModuleIndex::new(project());

        // A module calls the project it is in by the keyword, and not by the name the project is
        // declared under: a path rooted at a name is a path of the project that name is.
        let inserted = index.insert(
            module(0),
            &PlainPath::from_root(PathRoot::Named(Name::new("the-project")), [Name::new(
                "main",
            )]),
        );

        assert_eq!(inserted, None);
        assert!(index.is_empty());
    }

    #[test]
    fn a_path_of_another_project_is_not_a_path_of_this_one() {
        let mut index = ModuleIndex::new(project());

        let inserted = index.insert(
            module(0),
            &PlainPath::from_root(PathRoot::Named(Name::new("other")), [Name::new("main")]),
        );

        assert_eq!(inserted, None);
        assert!(index.is_empty());
    }

    #[test]
    fn the_index_reads_in_the_order_of_the_paths() {
        let index = index();

        let paths: Vec<String> = index
            .iter()
            .map(|(path, _)| path.last().expect("a name").as_str().to_owned())
            .collect();

        assert_eq!(paths, ["data", "utils", "main"]);
    }

    #[test]
    fn a_path_interned_from_a_plain_path_is_the_same_path() {
        // The index is keyed by the text of a path, so a path interned twice is one entry.
        let first = PlainPathId::new(path(&["main"]));
        let second = PlainPathId::new(path(&["main"]));

        assert_eq!(first, second);
    }
}
