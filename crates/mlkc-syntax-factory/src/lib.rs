//! The shape of every node, for the builder that assembles the parse.
//!
//! The parser's rules emit events rather than nodes (`mlkc-parser-core`);
//! the sink of the parse assembles them into a green tree through [`TreeBuilder`],
//! which asks a [`SyntaxFactory`] for a node of a kind made of the children it collected.
//! The generated factory here is that mapping for MLK:
//! it knows the slots of every node,
//! so a required child the source does not write, or an optional one it leaves out,
//! is an empty slot in the node's shape rather than a child that is missing.
//! The parser writes through [`SyntaxTreeBuilder`], which is that builder with this factory.
//!
//! The factory is generated from `xtask/codegen/mlk.ungram` and is not edited by hand;
//! the tree machinery itself is re-exported from [`mlkc_rowan`].

use mlkc_rowan::TreeBuilder;
use mlkc_syntax::MlkLanguage;

mod generated;
pub use crate::generated::SyntaxFactory;

pub type SyntaxTreeBuilder = TreeBuilder<'static, MlkLanguage, SyntaxFactory>;
