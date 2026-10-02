//! The WASM back end: MIR to the backend's LIR, and LIR to a module ([ADR-0020][adr-0020]).
//!
//! The back end is three stages:
//!
//! - [`lower_function`] compiles one SSA body into the LIR of the backend
//!   ([ADR-0022][adr-0022]), in parallel with every other body: selection, and the passes that
//!   follow it ([ADR-0005][adr-0005], [ADR-0009][adr-0009]);
//! - [`emit_function`] encodes one body of the LIR, making no decisions of its own;
//! - [`assemble_module`] assigns indices, lays out the code section, and writes the sections,
//!   deterministically, so that two compilations of one module agree byte for byte.
//!
//! A word carries no kind ([ADR-0018][adr-0018]), but the backend often knows one: a parameter
//! whose signature says `Int` is an immediate, and the result of `int-add` is one. What it
//! knows is a [`Refinement`], computed by a forward dataflow over the SSA body: a value known to
//! be an immediate gets a `(ref i31)` local, so arithmetic needs no cast, and a value that is
//! only a word gets an `eqref` one, so a cast is emitted where precision is needed. Selection
//! writes those decisions down as instructions of the target, and the passes collapse what the
//! types made wider than the machine needs.
//!
//! The module this crate builds uses Wasm GC: a structure is a GC struct type, and the types of
//! a module are named, never erased. The value model of [ADR-0018][adr-0018] is visible in the
//! module, and a value is a word unless the checker makes it narrower.
//!
//! # The ABI
//!
//! A function crosses the boundary as the shape of its signature ([`FnShape`]): a parameter or
//! a result of an immediate type --- `Int`, `Bool`, `Unit` --- is an `(ref i31)`, and every
//! other one is a word, an `eqref`. An immediate therefore needs no cast at the entry, and only
//! a value that is known to be a word is cast where something more precise is needed.
//!
//! # What is not emitted yet
//!
//! The emitter reports [`CodegenDiag::Unsupported`] for the constructs it does not lower yet
//! rather than emitting something wrong: string constants, calls to functions declared inside a
//! body, indirect calls, and field access. A call to a function of another module, and a call to
//! one declared `#[extern]`, is an import ([`ModuleImport`]), and the linker resolves it
//! ([ADR-0021][adr-0021]). Control flow of a body with more than one block is emitted as a
//! dispatch loop; structuring it into `if`/`loop` regions, allocating locals by live range, and
//! the DWARF custom sections are the next milestones of [ADR-0020][adr-0020].
//!
//! [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md
//! [adr-0009]: ../../docs/adr/0009-pass-contract.md
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../docs/adr/0021-translation-units.md
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

mod allocate;
mod collapse;
mod emit;
mod lower;
mod module;
mod refine;
mod select;

pub use crate::{
    emit::{CodegenDiag, FuncArtifact, FunctionCtx, Origin, ValueDebug, emit_function},
    lower::lower_function,
    module::{
        DebugLevel, ExportDecl, FnShape, FnSignature, ImportDecl, ModuleFunction, ModuleImport,
        ModuleLayout, ModuleMir, WasmModule, assemble_module, compile_module, layout,
    },
    refine::{AbiType, Refinement},
};
