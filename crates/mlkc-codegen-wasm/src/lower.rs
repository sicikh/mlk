//! Lowering one SSA body into the LIR of the back end ([ADR-0022][adr-0022]).
//!
//! This is the stage the driver calls `lir`: selection, which writes what the types say, and
//! the passes that make it small --- collapsing what the target has nothing to do, structuring
//! the control flow into the frames of the target, and deciding where the values that are left
//! live. What comes back is a value of the compiler, keyed and memoized per body by the driver,
//! and handed to encoding as it stands.
//!
//! A body that wrote lambdas comes back as a tree: the body itself, and one lower function per
//! lambda it wrote, depth first ([ADR-0026][adr-0026]). The driver pulls one tree per body, and
//! the encoder numbers the lifted functions in the order the layout lists them.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md
//! [adr-0026]: ../../docs/adr/0026-closure-representation.md

use mlkc_hir_def::{BodyEntityLoc, BodyLoc, EntityLoc};
use mlkc_la_arena::Arena;
use mlkc_lir_wasm::Body;
use mlkc_mir::{Body as MirBody, CodeRef, LambdaData};

use crate::{
    allocate, collapse,
    emit::FunctionCtx,
    module::{FunctionKey, LoweredFunction, lambda_children, lambda_name},
    select, structure,
};

/// Lowers one SSA body into the LIR of the back end, and the lambdas it wrote with it.
///
/// # Panics
///
/// Panics when the body the passes produce does not hold the invariant of the LIR, which is a
/// bug of a pass and not of the program ([ADR-0009][adr-0009]).
///
/// [adr-0009]: ../../docs/adr/0009-pass-contract.md
pub fn lower_function(mir: &MirBody, ctx: &FunctionCtx<'_>) -> LoweredFunction {
    // The SSA form of the body and of every lambda it wrote was checked together when the MIR
    // was built ([ADR-0019][adr-0019]).
    //
    // [adr-0019]: ../../docs/adr/0019-mir.md
    if let Err(invalid) = mir.validate_ssa() {
        panic!("the input of selection is an SSA body, and this one is not: {invalid}");
    }

    let Some(key) = entity_key(ctx.owner) else {
        panic!("the LIR of a body that is not a function of the module");
    };
    let body = lower_code(mir.code(), ctx);
    let lambdas = lower_children(mir.code(), &mir.lambdas, ctx);
    let local_functions = mir
        .local_functions
        .iter()
        .map(|local| lower_local(local, mir, ctx))
        .collect();

    LoweredFunction {
        key,
        body,
        lambdas,
        local_functions,
    }
}

/// Lowers one function declared in a `local`: a function of the module of its own, entered
/// without an environment.
///
/// The lambdas of the function are entries of the arena of the body that declares it: the arena
/// is the owner's, and a lambda written in a function declared in a `local` is numbered with the
/// rest of the owner's ([ADR-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
fn lower_local(local: &MirBody, owner: &MirBody, ctx: &FunctionCtx<'_>) -> LoweredFunction {
    let data = local
        .local
        .as_ref()
        .expect("a body in the list of functions declared in a `local` to be one");
    let plan = ctx
        .layout
        .local(ctx.owner, data.id)
        .unwrap_or_else(|| panic!("a function declared in a `local` the module did not number"));
    let child = FunctionCtx {
        owner: ctx.owner,
        name: &plan.name,
        signature: &plan.signature,
        param_names: &plan.param_names,
        layout: ctx.layout,
        lambda: None,
    };
    let body = lower_code(local.code(), &child);
    let lambdas = lower_children(local.code(), &owner.lambdas, &child);

    LoweredFunction {
        key: FunctionKey::Local {
            owner: ctx.owner.clone(),
            local: data.id,
        },
        body,
        lambdas,
        local_functions: Vec::new(),
    }
}

/// Lowers the lambdas a piece of code wrote, depth first ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
fn lower_children(
    code: CodeRef<'_>,
    lambdas: &Arena<LambdaData>,
    ctx: &FunctionCtx<'_>,
) -> Vec<LoweredFunction> {
    let mut lowered = Vec::new();

    for lambda in lambda_children(code) {
        let data = &lambdas[lambda];
        let plan = ctx
            .layout
            .lambda(ctx.owner, lambda)
            .unwrap_or_else(|| panic!("a lambda the module did not number"));
        let name = lambda_name(ctx.owner, lambda);
        let child = FunctionCtx {
            owner: ctx.owner,
            name: &name,
            signature: &plan.signature,
            param_names: &plan.param_names,
            layout: ctx.layout,
            lambda: Some(plan),
        };
        let body = lower_code(data.code(), &child);
        let lambdas = lower_children(data.code(), lambdas, &child);

        lowered.push(LoweredFunction {
            key: FunctionKey::Lambda {
                owner: ctx.owner.clone(),
                lambda,
            },
            body,
            lambdas,
            local_functions: Vec::new(),
        });
    }

    lowered
}

/// Lowers one piece of code: selection, and the passes that make it small.
fn lower_code(code: CodeRef<'_>, ctx: &FunctionCtx<'_>) -> Body {
    let selected = select::run(code, ctx);
    let mut body = collapse::run(&selected);

    body.structure = structure::run(&body);

    allocate::run(&mut body);

    if let Err(invalid) = body.validate() {
        panic!("the LIR of a body is not well-formed after lowering: {invalid}");
    }

    if let Err(invalid) = body.validate_locals() {
        panic!("the allocation of a body is not well-formed: {invalid}");
    }

    if body.structure.is_some()
        && let Err(invalid) = body.validate_structure()
    {
        panic!("the structure of a body is not well-formed: {invalid}");
    }

    body
}

/// The entity key of the body of an entity that declares a function.
fn entity_key(owner: &BodyEntityLoc) -> Option<FunctionKey> {
    let BodyLoc::Function(loc) = &owner.item else {
        return None;
    };

    Some(FunctionKey::Entity(EntityLoc {
        module: owner.module,
        item: loc.clone(),
    }))
}
