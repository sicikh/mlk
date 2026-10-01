//! The resolution of a module: what its names denote, once the modules they name are read.
//!
//! The HIR of a module stops at the module boundary on purpose: a path that names an entity of
//! another module is kept as the path the module wrote ([ADR-0010]). This crate is the stage
//! that reads those paths: it resolves the import table of a module and the paths its surface
//! writes, and answers with the scope of the module --- what each of its names denotes --- and
//! what the walk found wrong ([ADR-0016]).
//!
//! [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//!
//! # What a resolution reads
//!
//! A resolution reads the module's own item tree, and the modules its paths name. What it reads
//! of another module is its [`Interface`](mlkc_hir_def::Interface) --- the names it exports,
//! and the paths of the names a `use` re-exports --- and the entries of the
//! [`ModuleIndex`](mlkc_hir_def::ModuleIndex) of the projects it may name. Nothing else: a
//! resolution never reads the resolution of another module, which is what keeps the pull graph
//! of a driver stratified ([ADR-0008]).
//!
//! Those two things, gathered as a value, are the [`Closure`] of a resolution: the driver
//! assembles it by walking the module's paths once ([ADR-0009]), and the pass that follows
//! walks the same paths over the value it was handed.
//!
//! # The walk
//!
//! A path is a root and the names after it. The root is the keyword `project`, the name of a
//! project the module may name --- one its own project depends on, or any project of the graph
//! for a module no project claims --- or the name of a module of its own project; the names
//! after the root are read inside what it denotes. A root of a path of the module's own surface
//! is not read here: the lowering of the module is handed the projects it may name, and which
//! project a root names is an anchor the path is born with ([ADR-0016]).

//! A name a path ends at is what the module it lands in exports --- and when that is a re-export,
//! the walk continues from the path the re-export wrote, read in the project of the module that
//! wrote it. A chain that returns to where it started resolves to nothing, and is reported: the
//! walk is a loop inside one pass, and no fixpoint is needed to follow it.
//!
//! # A path written in a body
//!
//! The paths a [`Resolution`] is about are the paths of the *surface* of a module, and the pass
//! itself never reads a body ([ADR-0004], [ADR-0016]). A path written in a *body* is read by
//! [`Walk::entity_of`]: the anchor the lowering left, the imports the resolution resolved, and
//! the names after them, walked the same way. The stages that own bodies are what ask --- the
//! check, and the MIR lowering after it --- and what they make of the answer is theirs.
//!
//! [ADR-0004]: ../../docs/adr/0004-module-system.md
//! [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

mod closure;
mod diagnostic;
mod resolve;
mod walk;

pub use crate::{
    closure::{Closure, Read},
    diagnostic::{ResolveDiag, ResolveError, ResolvePlace},
    resolve::{Resolution, ResolveDeps, hidden_name, resolve_module},
    walk::{Target, Walk},
};
