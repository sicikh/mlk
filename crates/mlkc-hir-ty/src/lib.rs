//! A type, resolved, and the values a check leaves behind.
//!
//! The HIR holds a type as it is written: a [`TypeRef`] is a path, an `_`, or a mistake
//! ([ADR-0010]). This crate holds a type as it *means*: a class named by its
//! [`EntityLoc`](mlkc_hir_def::EntityLoc), a function, or a parameter of an entity.
//! Nothing here is an inference variable, a substitution, or a level: those belong to the
//! checker that produces a [`Ty`], and none of them is storable ([ADR-0017]).
//!
//! [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md
//! [`TypeRef`]: mlkc_hir_def::TypeRef
//!
//! # What a check leaves behind
//!
//! - [`Ty`] --- one type, a value of its own a driver can retain, compare, and serialize;
//! - [`ModuleTypes`] --- the types of a module's entities, resolved from the signatures the
//!   module writes, which is what its readers and its own bodies read;
//! - [`CheckedBody`] --- the types of the nodes of one body, which is what an IDE reads.
//!
//! # Modules
//!
//! - [`ty`] --- a type, and how it reads.
//! - [`module_types`] --- the type surface of a module.
//! - [`checked`] --- the types of one checked body.
//! - [`dump`] --- a reading of the values, for a person and for a diff.

pub mod checked;
pub mod dump;
pub mod module_types;
pub mod ty;

pub use crate::{
    checked::CheckedBody,
    module_types::ModuleTypes,
    ty::{INT_MAX, INT_MIN, Ty},
};
