//! The WASM back end: MIR to functions, and functions to a module ([ADR-0020][adr-0020]).
//!
//! The back end is two stages, because a module is assembled after its functions exist and
//! because the debug tables need addresses only the layout knows:
//!
//! - [`emit_function`] compiles one SSA body, in parallel with every other body, and owns its
//!   output ([ADR-0005][adr-0005], [ADR-0009][adr-0009]);
//! - [`assemble_module`] assigns indices, lays out the code section, and writes the sections,
//!   deterministically, so that two compilations of one module agree byte for byte.
//!
//! A word carries no kind ([ADR-0018][adr-0018]), but the emitter often knows one: a parameter
//! whose signature says `Int` is an immediate, and the result of `int-add` is one. What it
//! knows is a [`Refinement`], computed by a forward dataflow over the body: a value known to be
//! an immediate gets a `(ref i31)` local, so arithmetic needs no cast, and a value that is only
//! a word gets an `eqref` one, so a cast is emitted where precision is needed. The uniform word
//! ABI is therefore paid for once per value that crosses the ABI, not per access.
//!
//! The module this crate builds uses Wasm GC: a structure is a GC struct type, and the types of
//! a module are named, never erased. The value model of [ADR-0018][adr-0018] is visible in the
//! module, and a value is `eqref` at rest.
//!
//! # The ABI
//!
//! Every function is exported and called as the same WASM signature: one `eqref` per parameter,
//! and one `eqref` back. An immediate is an `i31ref` and is a subtype of `eqref`, so a value
//! crosses the boundary as itself; what makes a non-word value precise again is one cast at the
//! entry of the function that needs it.
//!
//! # What is not emitted yet
//!
//! The emitter reports [`CodegenDiag::Unsupported`] for the constructs it does not lower yet
//! rather than emitting something wrong: string constants, calls into other modules and to
//! functions declared inside a body, indirect calls, and field access. Control flow of a body
//! with more than one block is emitted as a dispatch loop; structuring it into `if`/`loop`
//! regions, allocating locals by live range, and the DWARF custom sections are the next
//! milestones of [ADR-0020][adr-0020].
//!
//! [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md
//! [adr-0009]: ../../docs/adr/0009-pass-contract.md
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md

mod emit;
mod module;
mod refine;

pub use crate::{
    emit::{CodegenDiag, FuncArtifact, FunctionCtx, Origin, ValueDebug, emit_function},
    module::{
        DebugLevel, FnSignature, ModuleFunction, ModuleLayout, ModuleMir, WasmModule,
        assemble_module, layout,
    },
    refine::Refinement,
};
