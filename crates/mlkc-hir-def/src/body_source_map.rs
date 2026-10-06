//! Where the nodes of a body are written.
//!
//! The HIR holds no positions: a body is values, and a range is what a host that marks a buffer
//! with reads. The lowering is the only stage that knows which syntax a node was read from, so
//! the map is built as the body is, and kept beside it.
//!
//! A range is the range of the syntax the node was read from, with the trivia around it left
//! out. A node the lowering made up rather than read --- an expression the parser did not find
//! --- has no range at all, and a parenthesized expression reads as the expression it holds,
//! parentheses included.
//!
//! A path is not one of the nodes the map holds: a path is a value a body holds once however
//! many places it is written in, and the places are the expressions that write it, each of which
//! reads as itself.

use mlkc_la_arena::ArenaMap;
use mlkc_text_size::TextRange;

use crate::{ExprId, PatId};

/// Where the nodes of one body are written, by the id of the node.
///
/// A map is a value of one lowering of one body: the ids are the positions of the arenas of
/// that body, and a range is a place in the file it was read from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BodySourceMap {
    exprs: ArenaMap<ExprId, TextRange>,
    pats: ArenaMap<PatId, TextRange>,
}

impl BodySourceMap {
    /// Where the expression with this id is written, if the lowering read it from the source.
    pub fn expr(&self, id: ExprId) -> Option<TextRange> {
        self.exprs.get(id).copied()
    }

    /// Where the pattern with this id is written, if the lowering read it from the source.
    pub fn pat(&self, id: PatId) -> Option<TextRange> {
        self.pats.get(id).copied()
    }

    /// Records where an expression is written, replacing what it had.
    ///
    /// A range written twice is the last one: a parenthesized expression is the expression it
    /// holds, and what a reader marks for it is the parentheses as well.
    pub fn set_expr(&mut self, id: ExprId, range: TextRange) {
        self.exprs.insert(id, range);
    }

    /// Records where a pattern is written, replacing what it had.
    pub fn set_pat(&mut self, id: PatId, range: TextRange) {
        self.pats.insert(id, range);
    }
}
