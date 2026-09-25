//! The identity of a module-level entity: its kind, its name, and its id.
//!
//! The kinds are declared once, in [`crate::macros`], and the families that have
//! to agree with them are generated from that list; what is written here is the part
//! that is not a function of the list: module ids, the shared data of a name,
//! the subsets the language defines, and the ids of the entities inside a body.

use std::fmt;

use mlkc_la_arena::{ArenaIndex, ArenaToken, Idx, RawIdx};
use mlkc_vfs::FileId;

use crate::{
    body::{LocalConstData, LocalFunctionData},
    item_tree::Entity,
    macros::{define_identity_family, for_each_item_kind},
    name::Name,
};

/// A module, named by its file.
///
/// The VFS interns a path into a [`FileId`] once and never reuses it,
/// and the path itself is the canonical form of the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub FileId);

/// The part of a name that every kind of entity shares.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemLocData {
    pub(crate) name: Option<Name>,
    pub(crate) disambiguator: u32,
}

impl ItemLocData {
    /// The name the entity was declared under,
    /// or `None` for an entity with no name of its own, such as an `impl`.
    pub fn name(&self) -> Option<&Name> {
        self.name.as_ref()
    }

    /// Which entity of this kind and name this is, counted from the start of the module.
    ///
    /// Zero unless the name is duplicated, which is an error that still has to be nameable.
    pub fn disambiguator(&self) -> u32 {
        self.disambiguator
    }
}

/// A name of an entity inside its owner.
///
/// Implemented by the name of every kind and by the erased [`ItemLoc`],
/// so that code that does not care about the kind never matches on one.
pub trait ItemLocLike {
    /// The part of the name every kind shares.
    fn data(&self) -> &ItemLocData;

    /// The kind of the entity the name denotes.
    fn kind(&self) -> ItemKind;

    /// The name the entity was declared under.
    fn name(&self) -> Option<&Name> {
        self.data().name()
    }

    /// Which entity of this kind and name this is.
    fn disambiguator(&self) -> u32 {
        self.data().disambiguator()
    }
}

for_each_item_kind!(define_identity_family);

/// A name or an id of one kind, used where another kind is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WrongKind {
    /// The kind the position wanted.
    pub expected: ItemKind,
    /// The kind the value has.
    pub found: ItemKind,
}

impl fmt::Display for WrongKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected a {}, found a {}", self.expected, self.found)
    }
}

impl std::error::Error for WrongKind {}

/// The name of an entity in the project: the module, and the name inside it.
///
/// The order is `(kind, text of the name, disambiguator)` inside a module
/// and the module is the outermost key, so a structure keyed by an entity name
/// iterates the same way in every process.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityLoc<I = ItemLoc> {
    /// The module that declares the entity.
    pub module: ModuleId,
    /// The name of the entity inside that module.
    pub item: I,
}

impl<I> EntityLoc<I> {
    /// The module that declares the entity.
    pub fn module(&self) -> ModuleId {
        self.module
    }

    /// The name of the entity inside its module.
    pub fn item(&self) -> &I {
        &self.item
    }
}

/// The number of a node in the arena it lives in, counted from zero.
///
/// This is the label a dump gives a node; it is not an identity of anything,
/// and it changes when a node is allocated before it.
pub(crate) fn arena_index<T>(id: Idx<T>) -> u32 {
    id.into_raw().into_u32() - 1
}

/// The name of an entity that owns a body, in the project.
pub type BodyEntityLoc = EntityLoc<BodyLoc>;

impl From<BodyEntityLoc> for EntityLoc {
    fn from(loc: BodyEntityLoc) -> Self {
        Self {
            module: loc.module,
            item: loc.item.into(),
        }
    }
}

impl TryFrom<EntityLoc> for BodyEntityLoc {
    type Error = WrongKind;

    fn try_from(loc: EntityLoc) -> Result<Self, Self::Error> {
        Ok(Self {
            module: loc.module,
            item: loc.item.try_into()?,
        })
    }
}

/// The name of an entity that owns a body.
///
/// The language decides which kinds those are, and the variant order follows
/// the order of [`ItemKind`], so that the two orders agree.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyLoc {
    /// A function.
    Function(FunctionLoc),
    /// A constant.
    Const(ConstLoc),
}

impl ItemLocLike for BodyLoc {
    fn data(&self) -> &ItemLocData {
        match self {
            Self::Function(loc) => loc.data(),
            Self::Const(loc) => loc.data(),
        }
    }

    fn kind(&self) -> ItemKind {
        match self {
            Self::Function(_) => ItemKind::Function,
            Self::Const(_) => ItemKind::Const,
        }
    }
}

impl std::fmt::Debug for BodyLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function(loc) => std::fmt::Debug::fmt(loc, f),
            Self::Const(loc) => std::fmt::Debug::fmt(loc, f),
        }
    }
}

impl From<FunctionLoc> for BodyLoc {
    fn from(loc: FunctionLoc) -> Self {
        Self::Function(loc)
    }
}

impl From<ConstLoc> for BodyLoc {
    fn from(loc: ConstLoc) -> Self {
        Self::Const(loc)
    }
}

impl From<BodyLoc> for ItemLoc {
    fn from(loc: BodyLoc) -> Self {
        match loc {
            BodyLoc::Function(loc) => Self::Function(loc),
            BodyLoc::Const(loc) => Self::Const(loc),
        }
    }
}

impl TryFrom<ItemLoc> for BodyLoc {
    type Error = WrongKind;

    fn try_from(loc: ItemLoc) -> Result<Self, Self::Error> {
        match loc {
            ItemLoc::Function(loc) => Ok(Self::Function(loc)),
            ItemLoc::Const(loc) => Ok(Self::Const(loc)),
            other => {
                Err(WrongKind {
                    expected: ItemKind::Function,
                    found: other.kind(),
                })
            },
        }
    }
}

impl TryFrom<BodyLoc> for FunctionLoc {
    type Error = WrongKind;

    fn try_from(loc: BodyLoc) -> Result<Self, Self::Error> {
        match loc {
            BodyLoc::Function(loc) => Ok(loc),
            other => {
                Err(WrongKind {
                    expected: ItemKind::Function,
                    found: other.kind(),
                })
            },
        }
    }
}

impl TryFrom<BodyLoc> for ConstLoc {
    type Error = WrongKind;

    fn try_from(loc: BodyLoc) -> Result<Self, Self::Error> {
        match loc {
            BodyLoc::Const(loc) => Ok(loc),
            other => {
                Err(WrongKind {
                    expected: ItemKind::Const,
                    found: other.kind(),
                })
            },
        }
    }
}

/// The id of a module-level entity that owns a body.
///
/// The subset the language defines, as an id inside one item tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleDefWithBodyId {
    /// A function.
    Function(ModuleFunctionId),
    /// A constant.
    Const(ModuleConstId),
}

impl ModuleDefWithBodyId {
    /// The kind of the entity.
    pub fn kind(&self) -> ItemKind {
        match self {
            Self::Function(_) => ItemKind::Function,
            Self::Const(_) => ItemKind::Const,
        }
    }
}

impl From<ModuleFunctionId> for ModuleDefWithBodyId {
    fn from(id: ModuleFunctionId) -> Self {
        Self::Function(id)
    }
}

impl From<ModuleConstId> for ModuleDefWithBodyId {
    fn from(id: ModuleConstId) -> Self {
        Self::Const(id)
    }
}

/// The id of a function declared inside one body.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalFunctionId(pub(crate) Idx<LocalFunctionData>);

impl std::fmt::Debug for LocalFunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fun#{}", arena_index(self.0))
    }
}

impl ArenaIndex for LocalFunctionId {
    fn into_raw(self) -> RawIdx {
        self.0.into_raw()
    }

    fn from_raw(index: RawIdx, token: ArenaToken) -> Self {
        Self(<Idx<LocalFunctionData> as ArenaIndex>::from_raw(
            index, token,
        ))
    }
}

/// The id of a constant declared inside one body.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalConstId(pub(crate) Idx<LocalConstData>);

impl std::fmt::Debug for LocalConstId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "const#{}", arena_index(self.0))
    }
}

impl ArenaIndex for LocalConstId {
    fn into_raw(self) -> RawIdx {
        self.0.into_raw()
    }

    fn from_raw(index: RawIdx, token: ArenaToken) -> Self {
        Self(<Idx<LocalConstData> as ArenaIndex>::from_raw(index, token))
    }
}

/// The id of an entity declared inside one body, whatever its kind.
///
/// A local entity has no name of its own:
/// it is not observable outside its owner, and the identity of the enclosing body
/// already pins every position inside it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LocalDefId {
    /// A function.
    Function(LocalFunctionId),
    /// A constant.
    Const(LocalConstId),
}

impl LocalDefId {
    /// The kind of the entity.
    pub fn kind(&self) -> ItemKind {
        match self {
            Self::Function(_) => ItemKind::Function,
            Self::Const(_) => ItemKind::Const,
        }
    }
}

impl std::fmt::Debug for LocalDefId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function(id) => std::fmt::Debug::fmt(id, f),
            Self::Const(id) => std::fmt::Debug::fmt(id, f),
        }
    }
}

impl From<LocalFunctionId> for LocalDefId {
    fn from(id: LocalFunctionId) -> Self {
        Self::Function(id)
    }
}

impl From<LocalConstId> for LocalDefId {
    fn from(id: LocalConstId) -> Self {
        Self::Const(id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn function(name: &str, disambiguator: u32) -> FunctionLoc {
        FunctionLoc(ItemLocData {
            name: Some(Name::new(name)),
            disambiguator,
        })
    }

    fn constant(name: &str) -> ConstLoc {
        ConstLoc(ItemLocData {
            name: Some(Name::new(name)),
            disambiguator: 0,
        })
    }

    fn class(name: &str) -> ClassLoc {
        ClassLoc(ItemLocData {
            name: Some(Name::new(name)),
            disambiguator: 0,
        })
    }

    #[test]
    fn a_typed_name_widens_into_an_erased_one() {
        let loc = function("foo", 0);

        assert_eq!(ItemLoc::from(loc.clone()), ItemLoc::Function(loc));
    }

    #[test]
    fn narrowing_a_name_of_another_kind_fails_loudly() {
        let error = FunctionLoc::try_from(ItemLoc::from(class("foo"))).unwrap_err();

        assert_eq!(error.expected, ItemKind::Function);
        assert_eq!(error.found, ItemKind::Class);
        assert_eq!(error.to_string(), "expected a Function, found a Class");
    }

    #[test]
    fn an_erased_name_is_looked_up_by_a_widened_one() {
        let loc = function("foo", 0);
        let mut map = HashMap::new();
        map.insert(ItemLoc::from(loc.clone()), "the entity");

        assert_eq!(map.get(&loc.into()), Some(&"the entity"));
    }

    #[test]
    fn the_orders_of_the_forms_agree() {
        let function = BodyLoc::from(function("foo", 0));
        let const_ = BodyLoc::from(constant("foo"));

        assert!(ItemKind::Function < ItemKind::Const);
        assert!(function < const_);
        assert!(ItemLoc::from(function) < ItemLoc::from(const_));
    }

    #[test]
    fn a_duplicated_name_is_disambiguated() {
        assert_ne!(function("foo", 0), function("foo", 1));
        assert!(function("foo", 0) < function("foo", 1));
    }
}
