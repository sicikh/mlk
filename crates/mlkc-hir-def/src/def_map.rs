//! What names denote: the names of one module, the resolved scope of one module,
//! and the index of the whole project.

use std::collections::BTreeMap;

use indexmap::IndexMap;

use crate::{
    id::{EntityLoc, ItemKind, ItemLocLike, ModuleId, UseLoc},
    item_data::Visibility,
    name::Name,
    path::PathAnchor,
    project_graph::ModuleLocator,
};

/// A namespace of a name: where a name is looked for, and what it denotes there.
///
/// The language has the same namespaces a resolved scope has ([`PerNs`]), so that a name
/// written in one of them means the same thing whether the module alone or the whole project
/// is what answers for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Namespace {
    /// Where a name written in the place of a type is looked for.
    Ty,
    /// Where a name written in the place of a value is looked for.
    Value,
    /// Where a name that starts a path to a module is looked for.
    Module,
}

impl ItemKind {
    /// The namespaces an entity of this kind is declared in.
    ///
    /// The rule is here, next to the namespaces it is about, and a caller that records a
    /// declaration asks it rather than deciding for itself.
    ///
    /// - a class is in the type namespace, and a function, a value, and a constant are in the
    ///   value namespace. A class that has a value of its own --- a constructor, or an
    ///   instance of it as a value --- is in both, which is what putting its name in both
    ///   here would say;
    /// - an `impl` has no name of its own, and a `use` is an entry of the import table:
    ///   the namespace an import lands in is what the import resolves to, which is not
    ///   something the module alone can say ([ADR-0004]).
    ///
    /// [ADR-0004]: ../../docs/adr/0004-module-system.md
    pub const fn namespaces(self) -> &'static [Namespace] {
        match self {
            Self::Class => &[Namespace::Ty],
            Self::Function | Self::Value | Self::Const => &[Namespace::Value],
            Self::Impl | Self::Use => &[],
        }
    }
}

/// What one name of a module denotes in one namespace.
///
/// Both variants are names by the rules of [ADR-0010]: an entity of this module, or an entry
/// of the import table of this module. Neither of them points outside the module.
///
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTarget {
    /// An entity the module declares.
    Item(EntityLoc),
    /// An entry of the module's import table.
    Use(UseLoc),
}

impl LocalTarget {
    /// What a path that names this target is anchored to.
    pub fn anchor(&self) -> PathAnchor {
        match self {
            Self::Item(item) => PathAnchor::Item(item.clone()),
            Self::Use(entry) => PathAnchor::Use(entry.clone()),
        }
    }
}

/// What one name of a module denotes, in the namespaces of that module.
///
/// A name may denote more than one thing at once: a class and a value may share a name, and a
/// name an import brings in may be a name an entity of the module has as well. What a name
/// denotes is therefore read per namespace, and the place a name is written in is what says
/// which namespace it is read in.
///
/// An import is held apart from the namespaces rather than written into them: which namespace
/// an import lands in is what the import resolves to, and the module alone cannot say
/// ([ADR-0004]). A name the module declares is what the name denotes in a namespace it is
/// declared in, and an import of the name is what it denotes in a namespace the module
/// declares nothing in.
///
/// [ADR-0004]: ../../docs/adr/0004-module-system.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalEntry {
    /// The type namespace: what the name denotes where a type belongs.
    pub ty: Option<LocalTarget>,
    /// The value namespace: what the name denotes where a value belongs.
    pub value: Option<LocalTarget>,
    /// The module namespace: what the name denotes at the start of a path to a module.
    pub module: Option<LocalTarget>,
    /// What an import of the name brings in, whichever namespace it lands in.
    pub import: Option<UseLoc>,
}

impl LocalEntry {
    /// What the name denotes in `namespace`.
    pub fn get(&self, namespace: Namespace) -> Option<&LocalTarget> {
        match namespace {
            Namespace::Ty => self.ty.as_ref(),
            Namespace::Value => self.value.as_ref(),
            Namespace::Module => self.module.as_ref(),
        }
    }

    /// The namespaces the name denotes something in, in the order of [`Namespace`].
    pub fn iter(&self) -> impl Iterator<Item = (Namespace, &LocalTarget)> {
        [Namespace::Ty, Namespace::Value, Namespace::Module]
            .into_iter()
            .filter_map(|namespace| Some((namespace, self.get(namespace)?)))
    }

    /// Whether the name denotes nothing at all.
    ///
    /// A name that only an import brings in is not empty: what it denotes is that import.
    pub fn is_none(&self) -> bool {
        self.ty.is_none() && self.value.is_none() && self.module.is_none() && self.import.is_none()
    }

    /// The slot of one namespace of the name.
    fn slot_mut(&mut self, namespace: Namespace) -> &mut Option<LocalTarget> {
        match namespace {
            Namespace::Ty => &mut self.ty,
            Namespace::Value => &mut self.value,
            Namespace::Module => &mut self.module,
        }
    }
}

/// The names a module declares, and what each of them denotes.
///
/// It is what the module's own text alone can say: a name resolves to an entity of the module
/// or to an entry of its import table, and no other module is read ([ADR-0004]).
/// The targets of imports appear when a [`ModuleScope`] resolves them.
///
/// The namespaces of a name are separate ([`Namespace`]), which is what keeps a name of one of
/// them out of the way of a name of another: a module that is edited to declare a function
/// named like a class of its own changes what the name means where a value belongs, and leaves
/// what it means where a type belongs alone.
///
/// [ADR-0004]: ../../docs/adr/0004-module-system.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalScope {
    entries: IndexMap<Name, LocalEntry>,
}

impl LocalScope {
    /// Records what the module declares under a name, and returns whether the name was free.
    ///
    /// The namespaces a declaration takes are the namespaces of its kind, and an import takes
    /// the import table. The first declaration of a name in a namespace wins: a declaration
    /// that finds the name taken there is not recorded in that namespace, and what is left for
    /// a diagnostic to report is that the module wrote the name twice. A name that is not
    /// there is not a name the module declared, so nothing is recorded and no name is taken.
    ///
    /// A name is inserted where the module first declares it, in any namespace; a declaration
    /// of the same name in another namespace joins the entry that is already there. The order
    /// of the entries is therefore the order of the module's names, not of its declarations.
    pub fn declare(&mut self, name: Name, target: LocalTarget) -> bool {
        if records_nothing(&name, &target) {
            return true;
        }

        let entry = self.entries.entry(name).or_default();

        match target {
            LocalTarget::Item(entity) => {
                let mut free = true;

                for &namespace in entity.item.kind().namespaces() {
                    let slot = entry.slot_mut(namespace);

                    if slot.is_some() {
                        free = false;
                    } else {
                        *slot = Some(LocalTarget::Item(entity.clone()));
                    }
                }

                free
            },
            // An import is an entry of the import table, and a name of the module is not an
            // import of it: the two are separate, and a name the module declares is what it
            // denotes in a namespace it is declared in.
            LocalTarget::Use(import) => {
                if entry.import.is_some() {
                    return false;
                }

                entry.import = Some(import);
                true
            },
        }
    }

    /// What the module declares under a name, in every namespace it declares it in.
    pub fn get(&self, name: &Name) -> Option<&LocalEntry> {
        self.entries.get(name)
    }

    /// The anchor a name resolves to against the names of this module alone.
    ///
    /// The rule lives here, once, so that the item tree's builder and the lowering of a body
    /// resolve a name the same way. A name the module declares is what it denotes in a
    /// namespace it is declared in; in a namespace the module declares nothing in, an import
    /// of the name is what it denotes, since which namespace an import lands in is what the
    /// import resolves to. A name the module says nothing about is unresolved, and so is a
    /// name it declares in another namespace, which only the scope of the whole project can
    /// tell apart from a name of another module.
    ///
    /// A caller that also knows the bindings and the type variables of the body it lowers
    /// checks those first, and falls back to this.
    pub fn anchor(&self, name: &Name, namespace: Namespace) -> PathAnchor {
        let Some(entry) = self.entries.get(name) else {
            return PathAnchor::Unresolved;
        };

        if let Some(target) = entry.get(namespace) {
            return target.anchor();
        }

        match &entry.import {
            Some(import) => PathAnchor::Use(import.clone()),
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

/// Whether a declaration has nothing to record, and so takes no name.
///
/// A name that is not there is not a name the module declared, and so is not a duplicate of
/// one; and an entity of a kind whose namespaces are none --- an `impl`, which has no name of
/// its own either --- has no name to lodge anywhere.
fn records_nothing(name: &Name, target: &LocalTarget) -> bool {
    if name.is_missing() {
        return true;
    }

    match target {
        LocalTarget::Item(entity) => entity.item.kind().namespaces().is_empty(),
        LocalTarget::Use(_) => false,
    }
}

/// What one name denotes in the namespaces of a resolved scope.
///
/// The same three namespaces a [`LocalScope`] holds, filled in with what the import tables of
/// the project resolved to rather than with what one module says on its own.
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
        id::ItemLoc,
        path::{PlainPath, PlainPathId},
    };

    fn module(index: u32) -> ModuleId {
        ModuleId(FileId::from_raw(index))
    }

    fn entity(name: &str, kind: ItemKind) -> EntityLoc {
        EntityLoc {
            module: module(0),
            item: ItemLoc::new(kind, Some(Name::new(name)), 0),
        }
    }

    fn function(name: &str) -> LocalTarget {
        LocalTarget::Item(entity(name, ItemKind::Function))
    }

    fn class(name: &str) -> LocalTarget {
        LocalTarget::Item(entity(name, ItemKind::Class))
    }

    fn import(name: &str) -> LocalTarget {
        LocalTarget::Use(
            UseLoc::try_from(ItemLoc::new(ItemKind::Use, Some(Name::new(name)), 0)).expect("a use"),
        )
    }

    #[test]
    fn the_first_declaration_of_a_name_in_a_namespace_wins() {
        let mut scope = LocalScope::default();

        assert!(scope.declare(Name::new("foo"), function("first")));
        assert!(!scope.declare(Name::new("foo"), function("second")));
        assert_eq!(scope.len(), 1);

        let entry = scope.get(&Name::new("foo")).expect("the name to be there");
        assert_eq!(entry.value, Some(function("first")));
    }

    #[test]
    fn a_class_and_a_function_may_share_a_name() {
        let mut scope = LocalScope::default();

        assert!(scope.declare(Name::new("Box"), class("Box")));
        assert!(scope.declare(Name::new("Box"), function("Box")));
        assert_eq!(scope.len(), 1);

        // The two are separate, and a name is read in the namespace the place it is written
        // in asks for.
        let entry = scope.get(&Name::new("Box")).expect("the name to be there");
        assert_eq!(entry.ty, Some(class("Box")));
        assert_eq!(entry.value, Some(function("Box")));
        assert_eq!(
            scope.anchor(&Name::new("Box"), Namespace::Ty),
            class("Box").anchor(),
        );
        assert_eq!(
            scope.anchor(&Name::new("Box"), Namespace::Value),
            function("Box").anchor(),
        );
    }

    #[test]
    fn a_name_declared_in_one_namespace_is_not_in_another() {
        let mut scope = LocalScope::default();
        scope.declare(Name::new("helper"), function("helper"));

        // A function is not a type: the module alone has nothing to say about `helper` where a
        // type belongs, and a name that is not there is a name the scope does not answer for.
        assert_eq!(
            scope.anchor(&Name::new("helper"), Namespace::Ty),
            PathAnchor::Unresolved,
        );
        assert_eq!(
            scope.anchor(&Name::new("helper"), Namespace::Value),
            function("helper").anchor(),
        );
    }

    #[test]
    fn a_name_an_import_brings_in_is_what_it_denotes_in_every_namespace() {
        let mut scope = LocalScope::default();
        assert!(scope.declare(Name::new("bar"), import("bar")));

        // Which namespace an import lands in is what the import resolves to, so a name the
        // module only imports is what the name denotes wherever it is written.
        let anchor = scope.anchor(&Name::new("bar"), Namespace::Ty);
        assert_eq!(anchor, scope.anchor(&Name::new("bar"), Namespace::Value));
        assert_eq!(anchor, scope.anchor(&Name::new("bar"), Namespace::Module));
        assert_eq!(anchor, import("bar").anchor());
    }

    #[test]
    fn a_name_the_module_declares_shadows_an_import_of_it() {
        let mut scope = LocalScope::default();
        scope.declare(Name::new("foo"), import("foo"));
        assert!(scope.declare(Name::new("foo"), class("foo")));

        // The declaration is what the name denotes where a type belongs, and the import is
        // what it denotes where the module declares nothing.
        assert_eq!(
            scope.anchor(&Name::new("foo"), Namespace::Ty),
            class("foo").anchor(),
        );
        assert_eq!(
            scope.anchor(&Name::new("foo"), Namespace::Value),
            import("foo").anchor(),
        );
    }

    #[test]
    fn an_entity_of_a_kind_with_no_namespace_of_its_own_is_in_no_scope() {
        let mut scope = LocalScope::default();

        // An `impl` has no name of its own to declare, and a name that would have lodged
        // nothing anywhere is not a name of the module.
        assert!(scope.declare(
            Name::new("impl"),
            LocalTarget::Item(entity("impl", ItemKind::Impl))
        ));
        assert!(scope.is_empty());
    }

    #[test]
    fn a_name_that_is_not_there_is_not_a_name_the_module_declares() {
        let mut scope = LocalScope::default();

        // Nothing is recorded, and nothing is taken: a name that is not there is no duplicate
        // of a name that is, and no path can name it.
        assert!(scope.declare(Name::missing(), function("foo")));
        assert!(scope.is_empty());
        assert_eq!(
            scope.anchor(&Name::missing(), Namespace::Value),
            PathAnchor::Unresolved,
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
    fn the_entry_of_a_name_reads_in_every_namespace_it_denotes_in() {
        let mut scope = LocalScope::default();
        scope.declare(Name::new("Box"), class("Box"));
        scope.declare(Name::new("Box"), function("Box"));

        let entry = scope.get(&Name::new("Box")).expect("the name to be there");
        let namespaces: Vec<_> = entry.iter().map(|(namespace, _)| namespace).collect();

        assert_eq!(namespaces, [Namespace::Ty, Namespace::Value]);
        assert!(!entry.is_none());
    }

    #[test]
    fn the_names_of_a_module_read_in_the_order_it_declares_them() {
        let mut scope = LocalScope::default();
        scope.declare(Name::new("second"), function("second"));
        scope.declare(Name::new("first"), function("first"));
        // A name declared in another namespace is not a second name of the module.
        scope.declare(Name::new("second"), class("second"));

        let names: Vec<_> = scope.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["second", "first"]);
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

    #[test]
    fn the_kinds_of_a_namespace_are_the_ones_the_language_declares() {
        assert_eq!(ItemKind::Class.namespaces(), [Namespace::Ty]);
        assert_eq!(ItemKind::Function.namespaces(), [Namespace::Value]);
        assert_eq!(ItemKind::Value.namespaces(), [Namespace::Value]);
        assert_eq!(ItemKind::Const.namespaces(), [Namespace::Value]);
        assert!(ItemKind::Impl.namespaces().is_empty());
        assert!(ItemKind::Use.namespaces().is_empty());
    }
}
