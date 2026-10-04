//! The middle IR: a uniform control-flow graph over words.
//!
//! MIR is the value the checked HIR is lowered into, and the value everything after it reads:
//! the optimizations, the WASM back end, and the native one that follows ([ADR-0019][adr-0019]).
//! A body is a graph of blocks, a block is a list of assignments and one terminator, and every
//! value is a word ([ADR-0018][adr-0018]): the operators say what they compute, and no type of
//! the representation is written down.
//!
//! Every function of MIR is a function of its module: a function declared in a `local` and a
//! lambda are lifted into bodies of their own ([ADR-0026][adr-0026]), so nothing after MIR reads
//! a nesting. [`Bodies`] is the flat set one HIR body lowers into.
//!
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md
//! [adr-0019]: ../../docs/adr/0019-mir.md
//! [adr-0026]: ../../docs/adr/0026-closure-representation.md
//!
//! # The two forms
//!
//! One set of types serves two forms, told apart by their invariants and not by their types:
//!
//! - the **CFG form** is what the lowering produces: an assignment writes a slot, a join is a
//!   slot the predecessors assigned, and a branch passes no arguments;
//! - the **SSA form** is what `mlkc_mir_build::construct_ssa` produces: every value is defined
//!   once, a join is a block parameter, and a branch passes one argument per parameter.
//!
//! [`Body::validate_cfg`] and [`Body::validate_ssa`] are the invariants of the two, and the SSA
//! verifier is the one the optimizers and the back ends rely on.
//!
//! # Modules
//!
//! - [`body`] --- the data of one function, the set of functions of one HIR body, and the
//!   builder the stages build with.
//! - [`verify`] --- the verifier: what tells the two forms apart.
//! - [`dump`] --- a reading of a body, for a person and for a diff.

pub mod body;
pub mod cfg;
pub mod dump;
pub mod verify;

#[cfg(test)]
pub(crate) mod test_support;

pub use crate::{
    body::{
        Block, BlockId, BlockTarget, Bodies, Body, BodyBuilder, Callee, CaptureData, Code, CodeRef,
        Const, FunctionLoc, LambdaId, LiftedId, LocalData, LocalId, Operand, Place, PrimOp, Rvalue,
        Stmt, StmtKind, Terminator, ValueData, ValueId,
    },
    verify::{Form, Invalid},
};
