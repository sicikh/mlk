//! What a host configures: how much debug information a module carries, and how hard the
//! pipeline optimizes ([ADR-0023][adr-0023]).
//!
//! The options are an input of the driver: a host pushes them like text, the driver versions
//! them, and what is built under them is a function of them ([ADR-0008][adr-0008]). A push that
//! changes them invalidates what read them, and a push that does not leaves everything where it
//! was.
//!
//! [adr-0008]: ../../docs/adr/0008-compiler-driver.md
//! [adr-0023]: ../../docs/adr/0023-debug-information.md

pub use mlkc_codegen_wasm::DebugLevel;

/// What a host asks the pipeline for ([ADR-0023][adr-0023]).
///
/// [adr-0023]: ../../docs/adr/0023-debug-information.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// How much debug information the assembled modules carry ([`DebugLevel`]).
    pub debug: DebugLevel,
    /// How hard the passes try to make the program small and fast ([`OptLevel`]).
    pub opt: OptLevel,
}

/// How hard the pipeline optimizes a program ([ADR-0023][adr-0023]).
///
/// The level gates the optional passes of the pipeline and nothing else: the passes a body
/// needs to be correct --- selection, collapsing, structuring, and allocation
/// ([ADR-0022][adr-0022]) --- run at every level. No optional pass exists yet, so the levels
/// assemble the same program today; [`OptLevel::Full`] is where constant folding, copy
/// propagation, dead code elimination, and inlining ([ADR-0019][adr-0019]) will be gated.
///
/// [adr-0019]: ../../docs/adr/0019-mir.md
/// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
/// [adr-0023]: ../../docs/adr/0023-debug-information.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    /// The passes the pipeline needs to be correct, and no optimization.
    #[default]
    None,
    /// Every optimization pass the pipeline has.
    Full,
}
