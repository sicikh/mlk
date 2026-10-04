//! The interpreter of MIR: the meaning of a body, read off the graph.
//!
//! The interpreter is the reference a back end is measured against: what a program computes when
//! it is interpreted is what the WASM module of the same program has to compute, and a test that
//! runs both compares the two ([ADR-0020][adr-0020]). It reads the same bodies codegen reads, so
//! a disagreement is a bug of codegen or of the interpreter and not of a form between them.
//!
//! The two forms of [ADR-0019][adr-0019] are one language here. The forms differ in their
//! invariants, never in their types: an SSA value and a CFG slot are two operands of the same
//! enum, a join reads a block parameter in one form and a slot in the other, and an edge passes
//! arguments in one and none in the other. One evaluation loop therefore runs both, and a test
//! runs the CFG form and the SSA form of a body side by side: what they compute has to agree
//! ([`crate::run`]).
//!
//! A closure is a word like any other ([ADR-0026][adr-0026]): the interpreter holds the code of
//! the lambda and the words it captured, a call through a closure runs that code with that
//! environment, and a read of a capture is a read of the environment the call was entered with.
//!
//! A function the program does not write --- one declared `#[extern]` --- is asked of the
//! [`Host`], by the canonical name the linker uses ([ADR-0021][adr-0021]). The interpreter has
//! no standard library of its own: `print-int` is what a host says it is, and the tests and the
//! runs of the real hosts implement it.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../docs/adr/0021-translation-units.md
//! [adr-0026]: ../../docs/adr/0026-closure-representation.md

mod eval;
mod program;
mod value;

pub use crate::{
    eval::{Host, Trap, run},
    program::{Extern, Program},
    value::{Closure, Value},
};
