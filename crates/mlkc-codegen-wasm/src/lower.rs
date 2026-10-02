//! Lowering one SSA body into the LIR of the back end ([ADR-0022][adr-0022]).
//!
//! This is the stage the driver calls `lir`: selection, which writes what the types say, and
//! the passes that make it small --- collapsing what the target has nothing to do, and deciding
//! where the values that are left live. What comes back is a value of the compiler, keyed and
//! memoized per body by the driver, and handed to encoding as it stands.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_lir_wasm::Body;
use mlkc_mir::Body as MirBody;

use crate::{allocate, collapse, emit::FunctionCtx, select};

/// Lowers one SSA body into the LIR of the back end, ready to encode.
///
/// # Panics
///
/// Panics when the body the passes produce does not hold the invariant of the LIR, which is a
/// bug of a pass and not of the program ([ADR-0009][adr-0009]).
///
/// [adr-0009]: ../../docs/adr/0009-pass-contract.md
pub fn lower_function(mir: &MirBody, ctx: &FunctionCtx<'_>) -> Body {
    let selected = select::run(mir, ctx);
    let mut body = collapse::run(&selected);

    allocate::run(&mut body);

    if let Err(invalid) = body.validate() {
        panic!("the LIR of a body is not well-formed after lowering: {invalid}");
    }

    if let Err(invalid) = body.validate_locals() {
        panic!("the allocation of a body is not well-formed: {invalid}");
    }

    body
}
