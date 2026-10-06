//! The temporary type check of MLK: one body at a time, with a signature written for every
//! top-level declaration.
//!
//! This crate is the first producer of [`mlkc_hir_ty`]'s values, and it is explicitly temporary:
//! the language has no generics, no `impl`s, and no fields yet, and the check that will replace
//! this one is not designed ([ADR-0017]). What lasts is the values it leaves behind.
//!
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md
//!
//! # The two passes
//!
//! - [`resolve_module_types`] resolves the signatures a module writes into a
//!   [`ModuleTypes`](mlkc_hir_ty::ModuleTypes), the type surface its readers and its own bodies
//!   read. It reads names and never a body, so an edit inside a body cannot change it.
//! - [`check_body`] checks one body against the signatures the module's entities wrote, and
//!   answers with a [`CheckedBody`](mlkc_hir_ty::CheckedBody).
//!
//! Both take the resolution of the module (what its names denote, and what each of its imports
//! resolved to) and a [`CheckDeps`](mlkc_hir_ty::CheckDeps): the projects, the closure of the
//! modules the paths reach,
//! the type surfaces of the modules the check reads, and the classes of the language. A path
//! that names a module --- a module of the project, a module of another project, or a name a
//! module re-exports --- is walked over that closure ([ADR-0016]), so a check of a body reads
//! exactly the values its input function was built from.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//!
//! A body is the unit: checking it never needs the *result* of checking another body, which is
//! what keeps the bodies of a module independent, parallel, and separately invalidated. Because
//! of that, a top-level declaration must write every one of its types for now: inferring a
//! signature from a body would need an order between the bodies of a module, and that is
//! deferred ([ADR-0017]).
//!
//! # The algorithm
//!
//! Bidirectional Hindley–Milner, with one unification and level-based generalization in the
//! style of Rémy, as the note [okmij] explains: `infer` computes a type where nothing fixes it,
//! `check` verifies one where something does. A `let` owns the variables its right side created,
//! and what is still owned at the end of the `let` becomes a parameter of the enclosing entity.
//! Nothing of it reaches a stored type: a variable that is neither resolved nor generalized
//! becomes an error before anything is recorded.
//!
//! [okmij]: https://okmij.org/ftp/ML/generalization.html

mod check;
mod diagnostic;
mod engine;
mod resolve;
mod signatures;

pub use crate::{
    check::check_body,
    diagnostic::{TypeDiag, TypeError, TypePlace},
    signatures::resolve_module_types,
};
