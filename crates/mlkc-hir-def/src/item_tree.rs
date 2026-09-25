//! The surface of one module: its entities, their names, and their data.

use std::fmt;

use mlkc_la_arena::{Arena, Idx};
use rustc_hash::FxHashMap;

use crate::{
    def_map::{LocalScope, LocalTarget},
    id::{
        BodyLoc, ClassLoc, ConstLoc, EntityLoc, FunctionLoc, ImplLoc, ItemKind, ItemLoc,
        ItemLocLike, ModuleClassId, ModuleConstId, ModuleDefId, ModuleDefWithBodyId,
        ModuleFunctionId, ModuleId, ModuleImplId, ModuleUseId, ModuleValueId, UseLoc, ValueLoc,
    },
    item_data::{ClassData, ConstData, EntityData, FunctionData, ImplData, UseData, ValueData},
    macros::{define_entity_accessors, for_each_item_kind},
    name::Name,
    path::PlainPathId,
};

/// Where an entity is in the module's syntax.
///
/// A path of child slots from the module root: the first index selects a child of the root,
/// the next selects a child of that node, and so on.
/// It is deliberately not a text range and not a count of the nodes of the file:
/// an edit inside one body must not move another entity,
/// or every revision of a comment would change what every entity is called.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemSyntaxLoc {
    slots: Vec<u32>,
}

impl ItemSyntaxLoc {
    /// The module itself.
    pub const fn root() -> Self {
        Self { slots: Vec::new() }
    }

    /// The child in `slot` of this node.
    pub fn child(&self, slot: u32) -> Self {
        let mut slots = self.slots.clone();
        slots.push(slot);
        Self { slots }
    }

    /// Appends a slot in place.
    pub fn push(&mut self, slot: u32) {
        self.slots.push(slot);
    }

    /// The slots of the path, from the module root.
    pub fn slots(&self) -> &[u32] {
        &self.slots
    }

    /// Whether the position is the module itself.
    pub fn is_root(&self) -> bool {
        self.slots.is_empty()
    }

    /// How many slots the path descends.
    pub fn depth(&self) -> usize {
        self.slots.len()
    }
}

impl fmt::Display for ItemSyntaxLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for slot in &self.slots {
            if !first {
                f.write_str(".")?;
            }
            first = false;
            write!(f, "{slot}")?;
        }
        Ok(())
    }
}

/// One entity of a module: its name, where it is in the syntax, and its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    loc: ItemLoc,
    syntax: ItemSyntaxLoc,
    data: EntityData,
}

impl Entity {
    /// The name of the entity.
    pub fn loc(&self) -> &ItemLoc {
        &self.loc
    }

    /// Where the entity is in the module's syntax.
    pub fn syntax(&self) -> &ItemSyntaxLoc {
        &self.syntax
    }

    /// The data of the entity.
    pub fn data(&self) -> &EntityData {
        &self.data
    }

    /// The kind of the entity.
    pub fn kind(&self) -> ItemKind {
        self.data.kind()
    }

    /// Resolves the anchors the paths of this entity's data left unresolved.
    pub(crate) fn resolve(&mut self, scope: &LocalScope) {
        self.data.resolve(scope);
    }
}

/// The surface of one module: what the text of that module alone determines.
///
/// The tree holds the entities of the module with the data behind them,
/// the name of every entity, and where each entity is in the syntax.
/// It holds no body, and nothing that an edit inside a body can shift,
/// so an edit inside a body leaves the tree equal
/// and no reader of an existing entity of the module is invalidated.
///
/// Its value includes where the entities are in the syntax, because a position
/// is only meaningful together with the tree it was computed from;
/// the names and the scope are functions of the entities and are compared with them.
#[derive(Debug, PartialEq, Eq)]
pub struct ItemTree {
    module: ModuleId,
    /// The path the module declares itself as, if its preamble writes one.
    path: Option<PlainPathId>,
    /// The entities, in the order the module declares them.
    entities: Arena<Entity>,
    /// What each name denotes in this revision: a function of `entities`.
    names: FxHashMap<ItemLoc, ModuleDefId>,
    /// What each name of the module denotes before another module is read.
    scope: LocalScope,
}

impl ItemTree {
    /// The module this tree describes.
    pub fn module(&self) -> ModuleId {
        self.module
    }

    /// The path the module declares itself as, if the module has a preamble.
    ///
    /// It is a claim about the file rather than an item of it: the module is named by its file
    /// ([`ModuleId`]), the preamble is the path the project knows it by ---
    /// `my-proj.main-module` --- and what checks that the two agree is the project.
    pub fn path(&self) -> Option<PlainPathId> {
        self.path.clone()
    }

    /// What each name of the module denotes before another module is read.
    pub fn scope(&self) -> &LocalScope {
        &self.scope
    }

    /// The entities of the module, in the order it declares them.
    pub fn entities(&self) -> impl Iterator<Item = (ItemLoc, ModuleDefId)> + '_ {
        self.entities
            .iter()
            .map(|(ix, entity)| (entity.loc.clone(), def_id(entity.data(), ix)))
    }

    /// The entity an id points at.
    ///
    /// # Panics
    ///
    /// Panics if the id was minted by another item tree.
    /// An id is a proof only inside the revision that holds it;
    /// a name is what crosses a revision.
    pub fn entity(&self, id: ModuleDefId) -> &Entity {
        &self.entities[id.ix()]
    }

    /// The data of the entity this name denotes, if this revision declares one.
    pub fn entity_data(&self, item: ItemLoc) -> Option<&EntityData> {
        self.id_of(item).map(|id| self.entity(id).data())
    }

    /// The id of the entity this name denotes, if this revision declares one.
    pub fn id_of(&self, item: ItemLoc) -> Option<ModuleDefId> {
        self.names.get(&item).copied()
    }

    /// The name of an entity, by its id.
    ///
    /// # Panics
    ///
    /// Panics if the id was minted by another item tree.
    pub fn loc_of(&self, id: ModuleDefId) -> ItemLoc {
        self.entity(id).loc().clone()
    }

    /// Where the entity this name denotes is in the module's syntax.
    pub fn syntax_loc(&self, item: ItemLoc) -> Option<&ItemSyntaxLoc> {
        self.id_of(item).map(|id| self.entity(id).syntax())
    }

    /// The id of the entity that carries this name, narrowed to the kind of the name.
    pub fn id_of_kind<E: ModuleEntity>(&self, item: E::Loc) -> Option<E> {
        E::try_from(self.id_of(item.into())?).ok()
    }

    /// The id of the entity that owns a body, if this revision declares one under that name.
    pub fn body_id(&self, item: BodyLoc) -> Option<ModuleDefWithBodyId> {
        match item {
            BodyLoc::Function(loc) => {
                self.id_of_kind::<ModuleFunctionId>(loc)
                    .map(ModuleDefWithBodyId::Function)
            },
            BodyLoc::Const(loc) => {
                self.id_of_kind::<ModuleConstId>(loc)
                    .map(ModuleDefWithBodyId::Const)
            },
        }
    }

    /// The name of an entity that owns a body, by its id.
    pub fn body_loc(&self, id: ModuleDefWithBodyId) -> BodyLoc {
        match id {
            ModuleDefWithBodyId::Function(id) => BodyLoc::Function(id.loc(self)),
            ModuleDefWithBodyId::Const(id) => BodyLoc::Const(id.loc(self)),
        }
    }
}

/// The id of a module-level entity of one kind.
///
/// An id is a proof of existence inside the item tree that minted it, and nothing outside it;
/// the kind rides in the type, so a pass that knows the kind it works with
/// gets the data of that kind without a match.
pub trait ModuleEntity: Copy + Into<ModuleDefId> + TryFrom<ModuleDefId> {
    /// The data the entity carries.
    type Data;

    /// The name of an entity of this kind inside its module.
    type Loc: ItemLocLike + Into<ItemLoc>;

    /// The kind this id belongs to.
    const KIND: ItemKind;

    /// The name the entity has in `tree`.
    ///
    /// # Panics
    ///
    /// Panics if the id was minted by another item tree.
    fn loc(self, tree: &ItemTree) -> Self::Loc;

    /// The data of the entity in `tree`.
    ///
    /// # Panics
    ///
    /// Panics if the id was minted by another item tree.
    fn data(self, tree: &ItemTree) -> &Self::Data;
}

/// What declaring an entity of a module produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The name of the entity, which is what it is addressed by.
    pub loc: EntityLoc,
    /// Whether the module declared the name in a namespace this entity is declared in.
    ///
    /// The scope keeps the first declaration and the item tree keeps both entities, so that a
    /// name the module wrote twice is still nameable; what the duplicate means is a diagnostic
    /// the caller reports. An entity whose name is not there is no duplicate of anything, and
    /// neither is an entity with no name of its own.
    pub duplicate: bool,
}

/// Builds the item tree of one module.
///
/// The builder is the only thing that mints names: it owns the counter that disambiguates
/// entities declared under one name, and it is what resolves the anchors a caller left
/// unresolved, against the names the module declared.
///
/// The order of the declarations is the order of the module's items:
/// it is the order the entities are numbered in, iterated in,
/// and the order an anonymous entity is named by.
#[derive(Debug)]
pub struct ItemTreeBuilder {
    module: ModuleId,
    path: Option<PlainPathId>,
    entities: Arena<Entity>,
    names: FxHashMap<ItemLoc, ModuleDefId>,
    scope: LocalScope,
    /// How many entities of each kind and name have been declared.
    declared: FxHashMap<(ItemKind, Option<Name>), u32>,
}

impl ItemTreeBuilder {
    /// A builder for the item tree of `module`.
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            path: None,
            entities: Arena::new(),
            names: FxHashMap::default(),
            scope: LocalScope::default(),
            declared: FxHashMap::default(),
        }
    }

    /// Records the path the module declares itself as, which is what its preamble writes.
    ///
    /// A module declares one path: a caller that records a second one replaces the first.
    pub fn set_path(&mut self, path: PlainPathId) {
        self.path = Some(path);
    }

    /// Declares one entity of the module, and returns what declaring it produced.
    ///
    /// `name` is `None` for an entity that has no name of its own, such as an `impl`;
    /// such an entity takes part in the order of the module and in nothing else.
    pub fn declare(
        &mut self,
        name: Option<Name>,
        data: EntityData,
        syntax: ItemSyntaxLoc,
    ) -> Declared {
        let kind = data.kind();
        let disambiguator = {
            let counter = self.declared.entry((kind, name.clone())).or_default();
            let current = *counter;
            *counter += 1;
            current
        };
        let item = ItemLoc::new(kind, name.clone(), disambiguator);
        let ix = self.entities.alloc(Entity {
            loc: item.clone(),
            syntax,
            data,
        });
        let id = def_id(self.entities[ix].data(), ix);
        self.names.insert(item.clone(), id);

        // What a name denotes in this module is recorded as the module declares it: a target
        // of this module, or an entry of its import table, since a `use` is not a definition
        // of it.
        let duplicate = match name {
            Some(name) => {
                let target = match &item {
                    ItemLoc::Use(import) => LocalTarget::Use(import.clone()),
                    _ => {
                        LocalTarget::Item(EntityLoc {
                            module: self.module,
                            item: item.clone(),
                        })
                    },
                };

                // The scope keeps the first declaration of a name in a namespace, and the one
                // that was not recorded is what a diagnostic is about.
                !self.scope.declare(name, target)
            },
            // An entity with no name of its own is in no scope, and is no duplicate of one.
            None => false,
        };

        Declared {
            loc: EntityLoc {
                module: self.module,
                item,
            },
            duplicate,
        }
    }

    /// Finishes the tree, resolving the anchors that no caller resolved.
    pub fn finish(mut self) -> ItemTree {
        for entity in self.entities.values_mut() {
            entity.resolve(&self.scope);
        }

        ItemTree {
            module: self.module,
            path: self.path,
            entities: self.entities,
            names: self.names,
            scope: self.scope,
        }
    }
}

for_each_item_kind!(define_entity_accessors);

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;
    use crate::{
        body::Pat,
        def_map::{LocalEntry, LocalTarget, Namespace},
        id::{FunctionLoc, UseLoc, WrongKind},
        item_data::{Attributes, ParamData, Signature, Visibility},
        path::{PathAnchor, PathData, PlainPath, PlainPathId},
        type_ref::TypeRef,
    };

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn syntax(slot: u32) -> ItemSyntaxLoc {
        ItemSyntaxLoc::root().child(slot)
    }

    fn item(name: &str, kind: ItemKind) -> ItemLoc {
        ItemLoc::new(kind, Some(Name::new(name)), 0)
    }

    /// The data of a function.
    ///
    /// The name is an argument for the reader: a name lives in the entity's loc,
    /// not in its data, and the builder mints it from what `declare` is given.
    fn function(_name: &str) -> EntityData {
        EntityData::Function(FunctionData {
            attributes: Attributes::default(),
            visibility: Visibility::Private,
            signature: Signature::default(),
        })
    }

    /// The data of a class, named for the reader as [`function`] is.
    fn class(_name: &str) -> EntityData {
        EntityData::Class(ClassData {
            attributes: Attributes::default(),
            visibility: Visibility::Private,
        })
    }

    fn import(path: &str) -> EntityData {
        EntityData::Use(UseData {
            path: PlainPathId::new(PlainPath::from_segments([Name::new(path)])),
            alias: None,
            visibility: Visibility::Private,
        })
    }

    fn anonymous_impl() -> EntityData {
        EntityData::Impl(ImplData {
            class: None,
            ty: None,
        })
    }

    fn tree(items: &[(&str, EntityData)]) -> ItemTree {
        let mut builder = ItemTreeBuilder::new(module());
        for (slot, (name, data)) in items.iter().enumerate() {
            builder.declare(Some(Name::new(name)), data.clone(), syntax(slot as u32));
        }
        builder.finish()
    }

    #[test]
    fn an_inserted_entity_leaves_the_name_of_another_alone() {
        let before = tree(&[("f", function("f")), ("g", function("g"))]);
        let after = tree(&[
            ("f", function("f")),
            ("h", function("h")),
            ("g", function("g")),
        ]);

        let name = item("g", ItemKind::Function);
        let data = before.entity_data(name.clone()).expect("g to be there");
        let before_id = before.id_of(name.clone()).expect("g to be there");
        let after_id = after.id_of(name).expect("g to be there");

        // The position moved, so a cache keyed by one would have lost `g`.
        assert_ne!(before.id_of(item("f", ItemKind::Function)), Some(before_id));
        assert_ne!(before_id, after_id);
        // The data did not, so a cache keyed by the name still holds what it read.
        assert_eq!(after.entity_data(item("g", ItemKind::Function)), Some(data));
    }

    #[test]
    fn a_tree_of_one_module_is_equal_to_itself() {
        let items = [
            ("f", function("f")),
            ("T", class("T")),
            ("g", function("g")),
        ];

        // Two revisions that declare the same things have equal trees:
        // this is what lets the driver keep the value it already has.
        assert_eq!(tree(&items), tree(&items));
    }

    #[test]
    fn the_names_and_the_ids_are_a_bijection() {
        let tree = tree(&[
            ("f", function("f")),
            ("T", class("T")),
            ("g", function("g")),
        ]);

        for (loc, id) in tree.entities() {
            assert_eq!(tree.id_of(loc.clone()), Some(id));
            assert_eq!(tree.loc_of(id), loc);
        }
    }

    #[test]
    fn a_badly_named_entity_has_no_id() {
        let tree = tree(&[("f", function("f"))]);

        assert_eq!(tree.id_of(item("g", ItemKind::Function)), None);
        // The kind is part of the name: a class named `f` is a different entity.
        assert_eq!(tree.id_of(item("f", ItemKind::Class)), None);
    }

    #[test]
    fn a_duplicated_name_is_disambiguated() {
        let tree = tree(&[("f", function("f")), ("f", function("f"))]);

        let first = ItemLoc::new(ItemKind::Function, Some(Name::new("f")), 0);
        let second = ItemLoc::new(ItemKind::Function, Some(Name::new("f")), 1);
        assert!(tree.id_of(first.clone()).is_some());
        assert!(tree.id_of(second.clone()).is_some());
        assert_ne!(tree.id_of(first.clone()), tree.id_of(second));
        // The scope keeps the first declaration of a name.
        assert_eq!(
            tree.scope().get(&Name::new("f")),
            Some(&LocalEntry {
                value: Some(LocalTarget::Item(EntityLoc {
                    module: module(),
                    item: first,
                })),
                ..LocalEntry::default()
            }),
        );
    }

    #[test]
    fn an_entity_with_no_name_of_its_own_is_in_no_scope() {
        let mut builder = ItemTreeBuilder::new(module());
        let declared = builder.declare(None, anonymous_impl(), syntax(0));
        let tree = builder.finish();

        assert!(declared.loc.item.name().is_none());
        assert_eq!(declared.loc.item.kind(), ItemKind::Impl);
        assert!(!declared.duplicate);
        assert_eq!(tree.scope().len(), 0);
        assert_eq!(
            tree.entity_data(declared.loc.item).map(EntityData::kind),
            Some(ItemKind::Impl)
        );
    }

    #[test]
    fn an_entity_whose_name_is_not_there_is_in_no_scope() {
        let mut builder = ItemTreeBuilder::new(module());
        let first = builder.declare(Some(Name::missing()), function("f"), syntax(0));
        let second = builder.declare(Some(Name::missing()), function("f"), syntax(1));
        let tree = builder.finish();

        // A name that is not there is not a name the module declares: nothing is recorded,
        // and two entities whose names are lost are not duplicates of one another.
        assert!(tree.scope().is_empty());
        assert!(!first.duplicate);
        assert!(!second.duplicate);
        // Both entities are still entities of the module, told apart by their names.
        assert_ne!(first.loc, second.loc);
        assert_eq!(first.loc.item.disambiguator(), 0);
        assert_eq!(second.loc.item.disambiguator(), 1);
    }

    #[test]
    fn a_class_and_a_function_may_share_a_name() {
        let mut builder = ItemTreeBuilder::new(module());
        let class = builder.declare(Some(Name::new("Box")), class("Box"), syntax(0));
        let function = builder.declare(Some(Name::new("Box")), function("Box"), syntax(1));
        let tree = builder.finish();

        // The two are names of the module in different namespaces, and neither is in the
        // way of the other.
        assert!(!class.duplicate);
        assert!(!function.duplicate);
        assert_eq!(
            tree.scope().get(&Name::new("Box")).map(LocalEntry::is_none),
            Some(false)
        );
        assert_eq!(
            tree.scope().anchor(&Name::new("Box"), Namespace::Ty),
            PathAnchor::Item(class.loc),
        );
        assert_eq!(
            tree.scope().anchor(&Name::new("Box"), Namespace::Value),
            PathAnchor::Item(function.loc),
        );
    }

    #[test]
    fn a_use_is_an_entry_of_the_scope() {
        let tree = tree(&[("project", import("project"))]);

        assert_eq!(
            tree.scope().get(&Name::new("project")),
            Some(&LocalEntry {
                import: Some(UseLoc::try_from(item("project", ItemKind::Use)).expect("a use")),
                ..LocalEntry::default()
            }),
        );
    }

    #[test]
    fn a_signature_resolves_a_local_anchor() {
        let mut builder = ItemTreeBuilder::new(module());
        builder.declare(Some(Name::new("T")), class("T"), syntax(0));
        builder.declare(
            Some(Name::new("f")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
                signature: Signature {
                    params: vec![ParamData {
                        pat: Pat::Bind(Name::new("x")),
                        ty: Some(TypeRef::Path(PathData::ident(
                            Name::new("T"),
                            PathAnchor::Unresolved,
                        ))),
                    }],
                    ret: None,
                },
            }),
            syntax(1),
        );
        let tree = builder.finish();

        let Some(EntityData::Function(data)) = tree.entity_data(item("f", ItemKind::Function))
        else {
            panic!("f is a function");
        };
        assert_eq!(
            data.signature.params[0].ty,
            Some(TypeRef::Path(PathData::ident(
                Name::new("T"),
                PathAnchor::Item(EntityLoc {
                    module: module(),
                    item: item("T", ItemKind::Class),
                }),
            ))),
        );
    }

    #[test]
    fn a_name_of_another_module_stays_unresolved() {
        let mut builder = ItemTreeBuilder::new(module());
        builder.declare(
            Some(Name::new("f")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Private,
                signature: Signature {
                    params: Vec::new(),
                    ret: Some(TypeRef::Path(PathData::ident(
                        Name::new("Other"),
                        PathAnchor::Unresolved,
                    ))),
                },
            }),
            syntax(0),
        );
        let tree = builder.finish();

        let Some(EntityData::Function(data)) = tree.entity_data(item("f", ItemKind::Function))
        else {
            panic!("f is a function");
        };
        assert_eq!(
            data.signature.ret,
            Some(TypeRef::Path(PathData::ident(
                Name::new("Other"),
                PathAnchor::Unresolved,
            ))),
        );
    }

    #[test]
    fn an_entity_remembers_where_it_is_in_the_syntax() {
        let tree = tree(&[("f", function("f"))]);

        assert_eq!(
            tree.syntax_loc(item("f", ItemKind::Function)),
            Some(&syntax(0)),
        );
    }

    #[test]
    fn a_body_has_a_name_only_if_its_kind_owns_one() {
        let tree = tree(&[("f", function("f")), ("T", class("T"))]);
        let body =
            BodyLoc::try_from(item("f", ItemKind::Function)).expect("a function owns a body");

        let id = tree.body_id(body.clone()).expect("f to be there");
        assert_eq!(tree.body_loc(id), body);

        let error = BodyLoc::try_from(item("T", ItemKind::Class)).unwrap_err();
        assert_eq!(error, WrongKind {
            expected: ItemKind::Function,
            found: ItemKind::Class,
        },);
    }

    #[test]
    fn the_typed_accessors_agree_with_the_erased_ones() {
        let tree = tree(&[("f", function("f"))]);
        let loc = FunctionLoc::try_from(item("f", ItemKind::Function)).expect("a function");

        let id = tree
            .id_of_kind::<ModuleFunctionId>(loc.clone())
            .expect("f to be there");
        assert_eq!(id, loc.id_in(&tree).expect("f to be there"));
        assert_eq!(id.loc(&tree), loc);
        assert_eq!(
            loc.data_in(&tree),
            match tree.entity_data(item("f", ItemKind::Function)) {
                Some(EntityData::Function(data)) => Some(data),
                _ => panic!("f is a function"),
            }
        );
    }

    #[test]
    fn the_syntax_position_is_shown_as_a_path() {
        assert_eq!(syntax(3).to_string(), "3");
        assert_eq!(syntax(3).child(1).to_string(), "3.1");
        assert!(ItemSyntaxLoc::root().is_root());
        assert_eq!(syntax(3).child(1).depth(), 2);
    }
}
