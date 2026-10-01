//! The construction of MIR: the lowering of a checked body, and the SSA form of it.

pub mod diagnostic;
mod lower;
mod ssa;

#[cfg(test)]
pub(crate) mod test_support;

pub use crate::{
    diagnostic::{MirDiag, MirError, Operator},
    lower::lower_body,
    ssa::construct_ssa,
};
