//! The WASM LIR: the target's instructions in SSA form ([ADR-0022][adr-0022]).
//!
//! This is the IR between MIR and `wasm-encoder`, and the backend owns it. MIR is uniform and
//! target-independent ([ADR-0018][adr-0018], [ADR-0019][adr-0019]); this is the other side of
//! the seam: every value has a type of the target --- `i32`, `(ref i31)`, `eqref`, and, as
//! structures and strings are lowered, their GC types --- and the instructions are the ones
//! WASM has, so that a box, an unbox, a cast, and a value that need not live in a local are all
//! rewrites over a program, and not modes of an emitter.
//!
//! It does not depend on `wasm-encoder`: encoding maps its instructions, and the IR stays a
//! value of the compiler, with a reading and a diff, like every other IR
//! ([ADR-0003][adr-0003], [ADR-0006][adr-0006]).
//!
//! # The shape
//!
//! - **Value-based SSA**: an instruction reads values and produces one; every value is defined
//!   once, by a parameter of the body, by a parameter of a block, or by one instruction.
//! - **Blocks with parameters**, as in MIR: a join takes a parameter, so there are no phi nodes.
//! - **No locals of its own**: a value is virtual until the allocation pass fills
//!   [`Locals`], which says which values live in a WASM local and which are emitted where they
//!   are read. The parameters of the function are the locals the ABI declares.
//! - **Spans on instructions**, which is what the line tables of the artifact are made of
//!   ([ADR-0020][adr-0020]).
//!
//! # Modules
//!
//! - [`body`] --- the data of one body, and the builder a stage builds with.
//! - [`ty`] --- the types of the target a value has.
//! - [`cfg`] --- the edges of a body, an order to walk them in, and the dominator tree.
//! - [`structure`] --- the structured control flow the structuring pass builds.
//! - [`verify`] --- the verifier: what tells a well-formed body from one that is not.
//! - [`dump`] --- a reading of a body, for a person and for a diff.
//!
//! [adr-0003]: ../../docs/adr/0003-id-based-ir.md
//! [adr-0006]: ../../docs/adr/0006-snapshot-testing.md
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md
//! [adr-0019]: ../../docs/adr/0019-mir.md
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

pub mod body;
pub mod cfg;
pub mod dump;
pub mod structure;
pub mod ty;
pub mod verify;

pub use crate::{
    body::{
        Block, BlockId, BlockTarget, Body, BodyBuilder, FuncIndex, Inst, Locals, Op, Terminator,
        ValueData, ValueId,
    },
    cfg::Cfg,
    structure::{Node, Structure},
    ty::{RefTy, Ty},
    verify::Invalid,
};
