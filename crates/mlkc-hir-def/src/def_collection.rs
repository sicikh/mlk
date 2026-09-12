use mlkc_la_arena::Arena;

use crate::{
    item_data::{ClassData, ConstData, FunctionData, ImplData, UseData, ValueData},
    path::PathData,
    type_ref::TypeRef,
};

#[derive(Default, Debug, Eq, PartialEq, Hash)]
pub struct DefCollection {
    pub(crate) uses: Arena<UseData>,
    pub(crate) paths: Arena<PathData>,
    pub(crate) type_refs: Arena<TypeRef>,
    pub(crate) functions: Arena<FunctionData>,
    pub(crate) values: Arena<ValueData>,
    pub(crate) impls: Arena<ImplData>,
    pub(crate) classes: Arena<ClassData>,
    pub(crate) consts: Arena<ConstData>,
}

impl DefCollection {
    pub fn new() -> Self {
        DefCollection::default()
    }

    pub fn shrink_to_fit(&mut self) {
        self.uses.shrink_to_fit();
        self.paths.shrink_to_fit();
        self.type_refs.shrink_to_fit();
        self.functions.shrink_to_fit();
        self.values.shrink_to_fit();
        self.impls.shrink_to_fit();
        self.classes.shrink_to_fit();
        self.consts.shrink_to_fit();
    }
}
