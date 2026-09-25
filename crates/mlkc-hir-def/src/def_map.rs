//! What names denote: the names of one module, the resolved scope of one module,
//! and the index of the whole project.

use std::collections::BTreeMap;

use indexmap::IndexMap;

use crate::{
    id::{EntityLoc, ModuleId, UseLoc},
    item_data::Visibility,
    name::Name,
    path::PathAnchor,
    project_graph::ModuleLocator,
};

/// What one name of a module denotes once only that module has been read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalEntry {
    /// An entity the module declares.
    Item(EntityLoc),
    /// An entry of the module's import table.
    Use(UseLoc),
}

/// The names a module declares, and what each of them denotes.
///
/// It is what the module's own text alone can say: a name resolves to an entity of the module
/// or to an entry of its import table, and no other module is read ([ADR-0004]).
/// The targets of imports appear when a [`ModuleScope`] resolves them.
///
/// [ADR-0004]: ../../docs/adr/0004-module-system.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalScope {
    entries: IndexMap<Name, LocalEntry>,
}

impl LocalScope {
    /// Records a declaration of a name.
    ///
    /// The first declaration of a name wins: a duplicate is an error that a diagnostic
    /// reports, and a scope does not decide which of the two the module meant.
    /// Returns whether the name was new.
    pub fn declare(&mut self, name: Name, entry: LocalEntry) -> bool {
        if self.entries.contains_key(&name) {
            return false;
        }

        self.entries.insert(name, entry);
        true
    }

    /// What a name denotes, if the module declares one.
    pub fn get(&self, name: &Name) -> Option<&LocalEntry> {
        self.entries.get(name)
    }

    /// The anchor a name resolves to against the names of this module alone.
    ///
    /// The rule lives here, once, so that the item tree's builder
    /// and the lowering of a body resolve a module-level name the same way.
    /// A caller that also knows the bindings and the type variables of the body it lowers
    /// checks those first, and falls back to this.
    pub fn anchor(&self, name: &Name) -> PathAnchor {
        match self.get(name) {
            Some(LocalEntry::Item(item)) => PathAnchor::Item(item.clone()),
            Some(LocalEntry::Use(entry)) => PathAnchor::Use(entry.clone()),
            None => PathAnchor::Unresolved,
        }
    }

    /// The names of the module, in the order it declares them.
    pub fn iter(&self) -> impl Iterator<Item = (&Name, &LocalEntry)> {
        self.entries.iter()
    }

    /// How many names the module declares.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the module declares no name at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// What one name denotes in the namespaces of a resolved scope.
///
/// An empty namespace is `None`; a name may be in more than one at once,
/// because a class and a value may share a name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PerNs {
    /// The type namespace.
    pub ty: Option<(EntityLoc, Visibility)>,
    /// The value namespace.
    pub value: Option<(EntityLoc, Visibility)>,
    /// The module namespace.
    pub module: Option<(ModuleLocator, Visibility)>,
}

impl PerNs {
    /// Whether the name denotes nothing at all.
    pub fn is_none(&self) -> bool {
        self.ty.is_none() && self.value.is_none() && self.module.is_none()
    }
}

/// What the names of one module denote once every module has been read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleScope {
    entries: IndexMap<Name, PerNs>,
}

impl ModuleScope {
    /// Records what a name denotes, replacing what it denoted before.
    pub fn insert(&mut self, name: Name, entry: PerNs) -> Option<PerNs> {
        self.entries.insert(name, entry)
    }

    /// What a name denotes, if the scope holds one.
    pub fn get(&self, name: &Name) -> Option<&PerNs> {
        self.entries.get(name)
    }

    /// The names of the module, in the order they were recorded.
    pub fn iter(&self) -> impl Iterator<Item = (&Name, &PerNs)> {
        self.entries.iter()
    }

    /// How many names the module has.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the module has no names at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The scopes of every module of a project, as an index of per-module contributions.
///
/// An index and not a value: a change to one module's surface replaces one entry,
/// and a consumer of the index is keyed by the entries it read, not by the index ([ADR-0008]).
/// The module ids order the entries, so the index reads the same way in every process.
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectDefMap {
    modules: BTreeMap<ModuleId, ModuleScope>,
}

impl ProjectDefMap {
    /// The scope of a module, if the index holds one.
    pub fn get(&self, module: ModuleId) -> Option<&ModuleScope> {
        self.modules.get(&module)
    }

    /// Replaces the scope of a module, returning the scope it had.
    pub fn set(&mut self, module: ModuleId, scope: ModuleScope) -> Option<ModuleScope> {
        self.modules.insert(module, scope)
    }

    /// Drops the scope of a module, returning it.
    pub fn remove(&mut self, module: ModuleId) -> Option<ModuleScope> {
        self.modules.remove(&module)
    }

    /// The modules of the index, in the order of their ids.
    pub fn iter(&self) -> impl Iterator<Item = (ModuleId, &ModuleScope)> {
        self.modules.iter().map(|(module, scope)| (*module, scope))
    }

    /// How many modules the index holds.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Whether the index holds no module at all.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;
    use crate::{
        id::{FunctionLoc, ItemKind, ItemLoc, ItemLocData, UseLoc},
        path::{PlainPath, PlainPathId},
    };

    fn module(index: u32) -> ModuleId {
        ModuleId(FileId::from_raw(index))
    }

    fn entity(name: &str) -> EntityLoc {
        EntityLoc {
            module: module(0),
            item: ItemLoc::Function(FunctionLoc(ItemLocData {
                name: Some(Name::new(name)),
                disambiguator: 0,
            })),
        }
    }

    #[test]
    fn the_first_declaration_of_a_name_wins() {
        let mut scope = LocalScope::default();

        assert!(scope.declare(Name::new("foo"), LocalEntry::Item(entity("first"))));
        assert!(!scope.declare(Name::new("foo"), LocalEntry::Item(entity("second"))));
        assert_eq!(scope.len(), 1);
        assert_eq!(
            scope.get(&Name::new("foo")),
            Some(&LocalEntry::Item(entity("first")))
        );
    }

    #[test]
    fn a_name_that_only_denotes_a_module_is_not_empty() {
        let mut per_ns = PerNs::default();
        assert!(per_ns.is_none());

        per_ns.module = Some((
            ModuleLocator::Prefix {
                project: crate::project_graph::ProjectId::new("project"),
                path: PlainPathId::new(PlainPath::from_segments([Name::new("project")])),
            },
            Visibility::Public,
        ));

        assert!(!per_ns.is_none());
    }

    #[test]
    fn a_name_anchors_to_what_the_module_declares() {
        let mut scope = LocalScope::default();
        scope.declare(Name::new("foo"), LocalEntry::Item(entity("foo")));
        let import = UseLoc::try_from(ItemLoc::new(ItemKind::Use, Some(Name::new("bar")), 0))
            .expect("a use");
        scope.declare(Name::new("bar"), LocalEntry::Use(import.clone()));

        assert_eq!(
            scope.anchor(&Name::new("foo")),
            PathAnchor::Item(entity("foo"))
        );
        assert_eq!(scope.anchor(&Name::new("bar")), PathAnchor::Use(import));
        assert_eq!(scope.anchor(&Name::new("baz")), PathAnchor::Unresolved);
    }

    #[test]
    fn a_project_def_map_replaces_and_drops_scopes() {
        let mut map = ProjectDefMap::default();
        let first = ModuleScope::default();
        let second = ModuleScope::default();

        assert_eq!(map.set(module(0), first), None);
        assert_eq!(map.set(module(0), second), Some(ModuleScope::default()));
        assert_eq!(map.len(), 1);
        assert!(map.remove(module(0)).is_some());
        assert!(map.is_empty());
    }

    #[test]
    fn the_index_reads_in_the_order_of_the_modules() {
        let mut map = ProjectDefMap::default();
        map.set(module(1), ModuleScope::default());
        map.set(module(0), ModuleScope::default());

        let order: Vec<_> = map.iter().map(|(module, _)| module).collect();
        assert_eq!(order, [module(0), module(1)]);
    }
}
