use mlkc_la_arena::{ArenaIndex, ArenaToken, Idx, RawIdx};
use mlkc_vfs::FileId;

use crate::item_data::{ClassData, ConstData, FunctionData, ImplData, UseData, ValueData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub FileId);

/// Key to item, defined on the module level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleItemKey<T> {
    pub module_id: ModuleId,
    pub id: T,
}

/// Key to item, defined on local level inside expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalItemKey<T> {
    pub module_id: ModuleId,
    pub owner_id: ModuleDefWithBodyId,
    pub id: T,
}

macro_rules! define_def_id {
    (
        $data:ident, $plain_id:ident, $module_id:ident, $local_id:ident, $enum_id:ident, $key:ident
    ) => {
        pub type $plain_id = Idx<$data>;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $module_id(pub(crate) Idx<$data>);

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $local_id(pub(crate) Idx<$data>);

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $enum_id {
            Module($module_id),
            Local($local_id),
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $key {
            Module(ModuleItemKey<$module_id>),
            Local(LocalItemKey<$local_id>),
        }

        impl $key {
            #[inline]
            pub fn module_id(&self) -> ModuleId {
                match self {
                    Self::Module(loc) => loc.module_id,
                    Self::Local(loc) => loc.module_id,
                }
            }

            #[inline]
            pub fn owner_id(&self) -> Option<ModuleDefWithBodyId> {
                match self {
                    Self::Module(_) => None,
                    Self::Local(loc) => Some(loc.owner_id),
                }
            }

            #[inline]
            pub fn as_module_key(&self) -> Option<ModuleItemKey<$module_id>> {
                match self {
                    Self::Module(loc) => Some(*loc),
                    Self::Local(_) => None,
                }
            }

            #[inline]
            pub fn as_local_key(&self) -> Option<LocalItemKey<$local_id>> {
                match self {
                    Self::Module(_) => None,
                    Self::Local(loc) => Some(*loc),
                }
            }
        }

        impl ArenaIndex for $module_id {
            fn into_raw(self) -> RawIdx {
                self.0.into_raw()
            }

            fn from_raw(i: RawIdx, token: ArenaToken) -> Self {
                Self(mlkc_la_arena::Idx::from_raw(i, token))
            }
        }

        impl ArenaIndex for $local_id {
            fn into_raw(self) -> RawIdx {
                self.0.into_raw()
            }

            fn from_raw(i: RawIdx, token: ArenaToken) -> Self {
                Self(mlkc_la_arena::Idx::from_raw(i, token))
            }
        }
    };
}

define_def_id!(UseData, PlainUseId, ModuleUseId, LocalUseId, UseId, UseKey);
define_def_id!(
    FunctionData,
    PlainFunctionId,
    ModuleFunctionId,
    LocalFunctionId,
    FunctionId,
    FunctionKey
);
define_def_id!(
    ValueData,
    PlainValueId,
    ModuleValueId,
    LocalValueId,
    ValueId,
    ValueKey
);
define_def_id!(
    ImplData,
    PlainImplId,
    ModuleImplId,
    LocalImplId,
    ImplId,
    ImplKey
);
define_def_id!(
    ClassData,
    PlainClassId,
    ModuleClassId,
    LocalClassId,
    ClassId,
    ClassKey
);
define_def_id!(
    ConstData,
    PlainConstId,
    ModuleConstId,
    LocalConstId,
    ConstId,
    ConstKey
);

macro_rules! define_def_group {
    (
        $module_id:ident,
        $local_id:ident,
        $group_id:ident,
        $key:ident,
        {
            $( $variant:ident ( $module_ty:ident, $local_ty:ident, $def_ty:ident ) ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $module_id {
            $( $variant($module_ty), )*
        }

        $(
            impl From<$module_ty> for $module_id {
                fn from(id: $module_ty) -> Self {
                    Self::$variant(id)
                }
            }
        )*

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $local_id {
            $( $variant($local_ty), )*
        }

        $(
            impl From<$local_ty> for $local_id {
                fn from(id: $local_ty) -> Self {
                    Self::$variant(id)
                }
            }
        )*

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $group_id {
            $( $variant($def_ty), )*
        }

        $(
            impl From<$def_ty> for $group_id {
                fn from(id: $def_ty) -> Self {
                    Self::$variant(id)
                }
            }
        )*

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $key {
            Module(ModuleItemKey<$module_id>),
            Local(LocalItemKey<$local_id>),
        }

        impl $key {
            #[inline]
            pub fn module_id(&self) -> ModuleId {
                match self {
                    Self::Module(loc) => loc.module_id,
                    Self::Local(loc) => loc.module_id,
                }
            }

            #[inline]
            pub fn owner_id(&self) -> Option<ModuleDefWithBodyId> {
                match self {
                    Self::Module(_) => None,
                    Self::Local(loc) => Some(loc.owner_id),
                }
            }

            #[inline]
            pub fn as_module_key(&self) -> Option<ModuleItemKey<$module_id>> {
                match self {
                    Self::Module(loc) => Some(*loc),
                    Self::Local(_) => None,
                }
            }

            #[inline]
            pub fn as_local_key(&self) -> Option<LocalItemKey<$local_id>> {
                match self {
                    Self::Module(_) => None,
                    Self::Local(loc) => Some(*loc),
                }
            }
        }
    };
}

define_def_group!(
    ModuleDefId, LocalDefId, DefId, DefKey,
    {
        Function(ModuleFunctionId, LocalFunctionId, FunctionId),
        Value(ModuleValueId, LocalValueId, ValueId),
        Impl(ModuleImplId, LocalImplId, ImplId),
        Class(ModuleClassId, LocalClassId, ClassId),
        Const(ModuleConstId, LocalConstId, ConstId),
    }
);

define_def_group!(
    ModuleDefWithBodyId, LocalDefWithBodyId, DefWithBodyId, DefWithBodyKey,
    {
        Function(ModuleFunctionId, LocalFunctionId, FunctionId),
        Const(ModuleConstId, LocalConstId, ConstId),
    }
);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TypeVarId {
    pub owner: DefId,
    pub index: u32,
}

macro_rules! define_def_map {
    (
        $map_name:ident,
        $enum_name:ident,
        { $( $field:ident : $variant:ident($id_type:ident) ),* $(,)? }
    ) => {
        #[derive(Debug, Clone)]
        pub struct $map_name<T> {
            $( pub $field: mlkc_la_arena::ArenaMap<$id_type, T>, )*
        }

        impl<T> Default for $map_name<T> {
            fn default() -> Self {
                Self {
                    $( $field: mlkc_la_arena::ArenaMap::default(), )*
                }
            }
        }

        impl<T> $map_name<T> {
            pub fn insert(&mut self, id: $enum_name, data: T) -> Option<T> {
                match id {
                    $( $enum_name::$variant(id) => self.$field.insert(id, data), )*
                }
            }

            pub fn get(&self, id: $enum_name) -> Option<&T> {
                match id {
                    $( $enum_name::$variant(id) => self.$field.get(id), )*
                }
            }
        }
    };
}

define_def_map!(
    ModuleDefMap,
    ModuleDefId,
    {
        functions: Function(ModuleFunctionId),
        classes:   Class(ModuleClassId),
        values:    Value(ModuleValueId),
        consts:    Const(ModuleConstId),
        impls:     Impl(ModuleImplId),
    }
);

define_def_map!(
    LocalDefMap,
    LocalDefId,
    {
        functions: Function(LocalFunctionId),
        classes:   Class(LocalClassId),
        values:    Value(LocalValueId),
        consts:    Const(LocalConstId),
        impls:     Impl(LocalImplId),
    }
);
