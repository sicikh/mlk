//! The construction of MIR: the lowering of a checked body, and the SSA form of it.
//!
//! The lowering is total over the bodies the check accepted: semantics is checked in one place,
//! so a construct the check let through that MIR cannot lower is a bug of the check, and the
//! pass stops with an internal compiler exception rather than a diagnostic ([ADR-0019]).
//!
//! [ADR-0019]: ../../docs/adr/0019-mir.md

mod lower;
mod ssa;

#[cfg(test)]
pub(crate) mod test_support;

pub use crate::{lower::lower_body, ssa::construct_ssa};
