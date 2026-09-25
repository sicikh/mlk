//! The macros that keep the identity vocabulary in step with the list of kinds.
//!
//! The kinds of module-level entity are declared once, in [`for_each_item_kind`],
//! and every family that has to agree with that list is generated from it:
//! the kinds, the names, the ids, the data, and the accessors of an item tree.
//!
//! The order of the list is part of the identity model:
//! it orders [`ItemKind`](crate::ItemKind), the erased name, and the entries of a scope,
//! so a kind is appended to the list, never inserted, renamed, or removed.

/// Declares the module-level kinds of entity the language has, once.
///
/// An entry is `Variant(Data, Loc, ModuleId)`: the kind, the data behind it,
/// the name of an entity of that kind, and its id inside an item tree.
macro_rules! for_each_item_kind {
    ($mac:ident) => {
        $mac! {
            Function(FunctionData, FunctionLoc, ModuleFunctionId),
            Class(ClassData, ClassLoc, ModuleClassId),
            Value(ValueData, ValueLoc, ModuleValueId),
            Const(ConstData, ConstLoc, ModuleConstId),
            Impl(ImplData, ImplLoc, ModuleImplId),
            Use(UseData, UseLoc, ModuleUseId),
        }
    };
}

pub(crate) use for_each_item_kind;

/// Expands the identity of the kinds: `ItemKind`, the names, and the ids.
///
/// Expands in [`crate::id`], where the hand-written parts of the family live.
macro_rules! define_identity_family {
    (
        $(
            $variant:ident($data:ident, $loc:ident, $module_id:ident),
        )*
    ) => {
        /// The kind of a module-level entity.
        ///
        /// The order of the variants is the order of the kinds in the identity model:
        /// it orders the erased name and the entries of a scope,
        /// so a variant is appended, never inserted, renamed, or removed.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ItemKind {
            $( $variant, )*
        }

        impl ItemKind {
            /// The name of the kind, as the language spells it.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => stringify!($variant), )*
                }
            }
        }

        impl std::fmt::Display for ItemKind {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        $(
            #[doc = concat!("The name of a `", stringify!($variant), "` inside its module.")]
            ///
            /// It is a value, not a position: it denotes the same entity
            /// in every revision in which the module declares an entity of that kind and name.
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $loc(pub(crate) ItemLocData);

            impl ItemLocLike for $loc {
                fn data(&self) -> &ItemLocData {
                    &self.0
                }

                fn kind(&self) -> ItemKind {
                    ItemKind::$variant
                }
            }

            impl From<$loc> for ItemLoc {
                fn from(loc: $loc) -> Self {
                    Self::$variant(loc)
                }
            }

            impl TryFrom<ItemLoc> for $loc {
                type Error = WrongKind;

                fn try_from(loc: ItemLoc) -> Result<Self, Self::Error> {
                    match loc {
                        ItemLoc::$variant(loc) => Ok(loc),
                        other => Err(WrongKind {
                            expected: ItemKind::$variant,
                            found: other.kind(),
                        }),
                    }
                }
            }

            #[doc = concat!("The id of a module-level `", stringify!($variant), "` in one item tree.")]
            ///
            /// It is a position in the tree's arena and a proof of existence inside it,
            /// and it means nothing outside the revision that minted it.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $module_id(pub(crate) Idx<Entity>);

            impl $module_id {
                /// The position of the entity in its item tree.
                pub(crate) fn ix(self) -> Idx<Entity> {
                    self.0
                }
            }

            impl ArenaIndex for $module_id {
                fn into_raw(self) -> RawIdx {
                    self.0.into_raw()
                }

                fn from_raw(index: RawIdx, token: ArenaToken) -> Self {
                    Self(<Idx<Entity> as ArenaIndex>::from_raw(index, token))
                }
            }

            impl From<$module_id> for ModuleDefId {
                fn from(id: $module_id) -> Self {
                    Self::$variant(id)
                }
            }

            impl TryFrom<ModuleDefId> for $module_id {
                type Error = WrongKind;

                fn try_from(id: ModuleDefId) -> Result<Self, Self::Error> {
                    match id {
                        ModuleDefId::$variant(id) => Ok(id),
                        other => Err(WrongKind {
                            expected: ItemKind::$variant,
                            found: other.kind(),
                        }),
                    }
                }
            }

            impl From<EntityLoc<$loc>> for EntityLoc {
                fn from(loc: EntityLoc<$loc>) -> Self {
                    Self {
                        module: loc.module,
                        item: loc.item.into(),
                    }
                }
            }
        )*

        /// The name of a module-level entity of any kind.
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ItemLoc {
            $( $variant($loc), )*
        }

        impl ItemLoc {
            /// Mints a name.
            ///
            /// Crate-internal on purpose: a name comes from an item tree's builder,
            /// which owns the counter that disambiguates duplicated names,
            /// never from a caller that assembled one out of parts.
            pub(crate) fn new(kind: ItemKind, name: Option<Name>, disambiguator: u32) -> Self {
                let data = ItemLocData {
                    name,
                    disambiguator,
                };
                match kind {
                    $( ItemKind::$variant => Self::$variant($loc(data)), )*
                }
            }
        }

        impl ItemLocLike for ItemLoc {
            fn data(&self) -> &ItemLocData {
                match self {
                    $( Self::$variant(loc) => &loc.0, )*
                }
            }

            fn kind(&self) -> ItemKind {
                match self {
                    $( Self::$variant(_) => ItemKind::$variant, )*
                }
            }
        }

        /// The id of a module-level entity of any kind, in one item tree.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ModuleDefId {
            $( $variant($module_id), )*
        }

        impl ModuleDefId {
            /// The kind of the entity.
            pub fn kind(&self) -> ItemKind {
                match self {
                    $( Self::$variant(_) => ItemKind::$variant, )*
                }
            }

            /// The position of the entity in its item tree.
            pub(crate) fn ix(self) -> Idx<Entity> {
                match self {
                    $( Self::$variant(id) => id.ix(), )*
                }
            }
        }
    };
}

pub(crate) use define_identity_family;

/// Expands the data of the kinds: the erased value an item tree's arena holds.
///
/// Expands in [`crate::item_data`], where the data of each kind is written down.
macro_rules! define_entity_data {
    (
        $(
            $variant:ident($data:ident, $loc:ident, $module_id:ident),
        )*
    ) => {
        /// The observable data of a module-level entity, whatever its kind.
        ///
        /// This is the value a dependent records and compares:
        /// it is self-contained, so it can be held without the item tree it came from.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum EntityData {
            $( $variant($data), )*
        }

        impl EntityData {
            /// The kind of the entity this data belongs to.
            pub fn kind(&self) -> ItemKind {
                match self {
                    $( Self::$variant(_) => ItemKind::$variant, )*
                }
            }
        }
    };
}

pub(crate) use define_entity_data;

/// Expands the accessors of an item tree for the kinds of entity.
///
/// Expands in [`crate::item_tree`], where the tree and its builder live.
macro_rules! define_entity_accessors {
    (
        $(
            $variant:ident($data:ident, $loc:ident, $module_id:ident),
        )*
    ) => {
        /// The id of the entity whose data is `data`, at `ix` in the arena of its tree.
        pub(crate) fn def_id(data: &EntityData, ix: Idx<Entity>) -> ModuleDefId {
            match data {
                $( EntityData::$variant(_) => ModuleDefId::$variant($module_id(ix)), )*
            }
        }

        $(
            impl $loc {
                /// The id of the entity this name denotes in `tree`,
                /// if the revision the tree belongs to declares one.
                pub fn id_in(&self, tree: &ItemTree) -> Option<$module_id> {
                    tree.id_of_kind::<$module_id>(self.clone())
                }

                /// The data of the entity this name denotes in `tree`,
                /// if the revision the tree belongs to declares one.
                pub fn data_in<'t>(&self, tree: &'t ItemTree) -> Option<&'t $data> {
                    self.id_in(tree).map(|id| id.data(tree))
                }
            }

            impl ModuleEntity for $module_id {
                type Data = $data;
                type Loc = $loc;

                const KIND: ItemKind = ItemKind::$variant;

                fn loc(self, tree: &ItemTree) -> Self::Loc {
                    match tree.loc_of(self.into()) {
                        ItemLoc::$variant(loc) => loc,
                        other => unreachable!(
                            "an id of a {} is never minted for a {}",
                            ItemKind::$variant,
                            other.kind(),
                        ),
                    }
                }

                fn data(self, tree: &ItemTree) -> &Self::Data {
                    match tree.entity(self.into()).data() {
                        EntityData::$variant(data) => data,
                        other => unreachable!(
                            "an id of a {} is never minted for a {}",
                            ItemKind::$variant,
                            other.kind(),
                        ),
                    }
                }
            }
        )*
    };
}

pub(crate) use define_entity_accessors;
