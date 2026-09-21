//! The parse rules of MLK.
//!
//! Every rule is named `parse_*` after the node of `mlk.ungram` it parses, takes the parser
//! and returns a [`ParsedSyntax`]:
//!
//! - [`ParsedSyntax::Present`] — the rule parsed the node into the tree. It consumed at
//!   least one token, and it reported every mistake it ran into on the way.
//! - [`ParsedSyntax::Absent`] — the source does not hold the node at the current position.
//!   The rule must not have consumed a token and must not have reported anything: whether
//!   the absence is an error depends on the context, which only the caller knows.
//!
//! A rule that has to keep parsing after a mistake recovers from it: it wraps the
//! unexpected tokens into one of the `Bogus*` nodes of the grammar. The tokens then stay
//! in the tree in the place where they were written, and the rest of the file parses as if
//! they were not there.
//!
//! Nodes that the grammar declares as a list are parsed by a type implementing
//! [`ParseNodeList`] or [`ParseSeparatedList`]; the node of a list is created even when
//! the list is empty, so that the children of its parent keep their slots.

pub(crate) mod attribute;
pub(crate) mod auxiliary;
pub(crate) mod expr;
pub(crate) mod module;
pub(crate) mod parse_error;
pub(crate) mod pat;
pub(crate) mod ty;
