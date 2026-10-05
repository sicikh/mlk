//! The vocabulary of the maps the crates of the workspace share.
//!
//! A stage that keeps a map it reads back as a sequence --- the definitions of a module,
//! the types a body was checked to --- needs that read to be in a defined order
//! ([ADR-0009]), and the hasher the compiler uses everywhere is `rustc-hash`'s.
//! [`FxIndexMap`] names that pair once, instead of spelling it out crate by crate.
//!
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

/// An [`IndexMap`](indexmap::IndexMap) hashed with
/// [`FxBuildHasher`](rustc_hash::FxBuildHasher).
///
/// The map keeps the order its entries were inserted in,
/// so an iteration over it is deterministic;
/// the hasher decides where an entry is looked up, never where it is read.
pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
