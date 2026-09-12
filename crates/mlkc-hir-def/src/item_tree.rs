use mlkc_la_arena::Arena;

use crate::{
    def_collection::DefCollection,
    id::{
        ModuleClassId, ModuleConstId, ModuleFunctionId, ModuleImplId, ModuleUseId, ModuleValueId,
    },
    item_data::{ClassData, ConstData, FunctionData, ImplData, UseData, ValueData},
    path::{PathData, PathId},
    type_ref::{TypeRef, TypeRefId},
};

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct ItemTree {
    defs: DefCollection,
    paths: Arena<PathData>,
    type_refs: Arena<TypeRef>,
}

#[rustfmt::skip]
implement_defs_reader!(ItemTree:
    uses,      uses_values,      ModuleUseId,      UseData,
    functions, functions_values, ModuleFunctionId, FunctionData,
    values,    values_values,    ModuleValueId,    ValueData,
    impls,     impls_values,     ModuleImplId,     ImplData,
    classes,   classes_values,   ModuleClassId,    ClassData,
    consts,    consts_values,    ModuleConstId,    ConstData,
);

#[rustfmt::skip]
implement_arenas_reader!(ItemTree:
    paths,     paths_values,     PathId,    PathData,
    type_refs, type_refs_values, TypeRefId, TypeRef,
);

// #[rustfmt::skip]
// implement_defs_builder!(ItemTreeLoweringCtx:
//     alloc_use,      uses,      ModuleUseId,      UseData,
//     alloc_function, functions, ModuleFunctionId, FunctionData,
//     alloc_value,    values,    ModuleValueId,    ValueData,
//     alloc_impl,     impls,     ModuleImplId,     ImplData,
//     alloc_class,    classes,   ModuleClassId,    ClassData,
//     alloc_const,    consts,    ModuleConstId,    ConstData,
// );
//
// #[rustfmt::skip]
// implement_arenas_builder!(ItemTreeLoweringCtx:
//     alloc_path,     paths,     PathId,    PathData,
//     alloc_type_ref, type_refs, TypeRefId, TypeRef,
// );

impl ItemTree {
    pub fn new(defs: DefCollection, paths: Arena<PathData>, type_refs: Arena<TypeRef>) -> Self {
        Self {
            defs,
            paths,
            type_refs,
        }
    }
}
