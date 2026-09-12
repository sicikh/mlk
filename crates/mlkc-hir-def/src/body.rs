use mlkc_la_arena::{Arena, ArenaMap, Idx};

use crate::{
    def_collection::DefCollection,
    id::{LocalClassId, LocalConstId, LocalFunctionId, LocalImplId, LocalUseId, LocalValueId},
    item_data::{ClassData, ConstData, FunctionData, ImplData, UseData, ValueData},
    name::Name,
    path::{PathData, PathId},
    type_ref::{TypeRef, TypeRefId},
};

pub type ExprId = Idx<Expr>;
pub type PatId = Idx<Pat>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Body {
    /// Items, defined inside expressions of this body.
    defs: DefCollection,
    /// Paths, used inside expressions of this body.
    paths: Arena<PathData>,
    /// References to types inside this body.
    type_refs: Arena<TypeRef>,
    /// Expressions of this body. Expressions of local functions also stored here.
    exprs: Arena<Expr>,
    /// Patterns, used in this body and in local functions defined in this body.
    pats: Arena<Pat>,
    /// Patterns that are used as arguments for the owner of this body.
    pat_args: Vec<PatId>,
    /// Root expression.
    root: ExprId,
    /// Map to roots of local functions.
    local_function_roots: ArenaMap<LocalFunctionId, ExprId>,
    /// Map to roots of local consts.
    local_consts_roots: ArenaMap<LocalConstId, ExprId>,
}

#[rustfmt::skip]
implement_arenas_reader!(Body:
    paths,     paths_values,     PathId,    PathData,
    type_refs, type_refs_values, TypeRefId, TypeRef,
    exprs,     exprs_values,     ExprId,    Expr,
    pats,      pats_values,      PatId,     Pat,
);

#[rustfmt::skip]
implement_defs_reader!(Body:
    uses,      uses_values,      LocalUseId,      UseData,
    functions, functions_values, LocalFunctionId, FunctionData,
    values,    values_values,    LocalValueId,    ValueData,
    impls,     impls_values,     LocalImplId,     ImplData,
    classes,   classes_values,   LocalClassId,    ClassData,
    consts,    consts_values,    LocalConstId,    ConstData,
);

impl Body {
    #[expect(
        clippy::too_many_arguments,
        reason = "BodyLowerCtx (builder) is defined in another crate"
    )]
    pub fn new(
        defs: DefCollection,
        paths: Arena<PathData>,
        type_refs: Arena<TypeRef>,
        exprs: Arena<Expr>,
        pats: Arena<Pat>,
        pat_args: Vec<PatId>,
        root: ExprId,
        local_function_roots: ArenaMap<LocalFunctionId, ExprId>,
        local_consts_roots: ArenaMap<LocalConstId, ExprId>,
    ) -> Self {
        Self {
            defs,
            paths,
            type_refs,
            exprs,
            pats,
            pat_args,
            root,
            local_function_roots,
            local_consts_roots,
        }
    }
}

// #[rustfmt::skip]
// implement_arenas_builder!(BodyLoweringCtx:
//     alloc_path,     paths,     PathId,    PathData,
//     alloc_type_ref, type_refs, TypeRefId, TypeRef,
//     alloc_expr,     exprs,     ExprId,    Expr,
//     alloc_pat,      pats,      PatId,     Pat,
// );
//
// #[rustfmt::skip]
// implement_defs_builder!(BodyLoweringCtx:
//     alloc_use,      uses,      LocalUseId,      UseData,
//     alloc_function, functions, LocalFunctionId, FunctionData,
//     alloc_value,    values,    LocalValueId,    ValueData,
//     alloc_impl,     impls,     LocalImplId,     ImplData,
//     alloc_class,    classes,   LocalClassId,    ClassData,
//     alloc_const,    consts,    LocalConstId,    ConstData,
// );

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Missing,
    Path(PathId),
    Literal(Literal),
    Call {
        callee: ExprId,
        args: Vec<ExprId>,
    },
    SeqExpr {
        first: ExprId,
        then: ExprId,
    },
    LetExpr {
        pat: PatId,
        expr: ExprId,
        body: ExprId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pat {
    Missing,
    Wildcard,
    Bind(Name),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    Int(i64),
}
