//! Lowering one SSA body into the LIR of the back end ([ADR-0022][adr-0022]).
//!
//! This is the stage the driver calls `lir`: selection, which writes what the types say, and
//! the passes that make it small --- collapsing what the target has nothing to do, structuring
//! the control flow into the frames of the target, and deciding where the values that are left
//! live. What comes back is a value of the compiler, keyed and memoized per HIR body by the
//! driver, and handed to encoding as it stands.
//!
//! Every function is lowered on its own, the body of an entity, a function declared in a `local`,
//! and a lambda alike: MIR lifted them into functions of the module ([ADR-0019][adr-0019]), so
//! one body becomes one function and nothing here reads a nesting.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_lir_wasm::Body;
use mlkc_mir::{Body as MirBody, CodeRef};

use crate::{allocate, collapse, emit::FunctionCtx, module::LoweredFunction, select, structure};

/// Lowers one SSA body into the LIR of the back end.
///
/// # Panics
///
/// Panics when the body the passes produce does not hold the invariant of the LIR, which is a
/// bug of a pass and not of the program ([ADR-0009][adr-0009]).
///
/// [adr-0009]: ../../docs/adr/0009-pass-contract.md
pub fn lower_function(mir: &MirBody, ctx: &FunctionCtx<'_>) -> LoweredFunction {
    // The SSA form of the body was checked when the MIR was built ([ADR-0019][adr-0019]).
    //
    // [adr-0019]: ../../docs/adr/0019-mir.md
    if let Err(invalid) = mir.validate_ssa() {
        panic!("the input of selection is an SSA body, and this one is not: {invalid}");
    }

    let body = lower_code(mir.code(), ctx);

    LoweredFunction {
        function: mir.function.clone(),
        name: ctx.name.to_owned(),
        body,
    }
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
