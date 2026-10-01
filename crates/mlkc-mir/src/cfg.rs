//! The shape of a body's graph.
//!
//! The edges of every block and an order to walk them in are what the verifier, the SSA
//! construction, and every analysis over a body read. They are computed once, here, and read
//! by position: a position is what an arena id is, and an algorithm that walks the graph is
//! written over positions ([`mlkc_la_arena::Idx::index`]).
//!
//! A block that no path from the entry reaches is in the graph all the same --- an edge to it
//! exists, and an analysis may look at it --- but it is not in the reverse postorder, and the
//! promises of a form are vacuous for it: there is no path for them to be about.

use crate::{BlockTarget, Body, Terminator};

/// The edges of a body, and the order a walk visits its blocks in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    /// The successors of every block, by position, in the order the terminator lists them.
    succs: Vec<Vec<usize>>,
    /// The predecessors of every block, by position, in the order the blocks are allocated.
    preds: Vec<Vec<usize>>,
    /// The blocks reached from the entry, in reverse postorder, by position.
    rpo: Vec<usize>,
}

impl Cfg {
    /// The graph of `body`.
    pub fn of(body: &Body) -> Self {
        let mut succs = vec![Vec::new(); body.blocks.len()];
        let mut preds = vec![Vec::new(); body.blocks.len()];

        for (id, block) in body.blocks.iter() {
            let from = id.index();

            for target in targets(&block.term) {
                let to = target.block.index();

                if to < body.blocks.len() {
                    succs[from].push(to);
                    preds[to].push(from);
                }
            }
        }

        let rpo = reverse_postorder(body.entry.index(), &succs);

        Self { succs, preds, rpo }
    }

    /// The successors of a block, by position.
    pub fn successors(&self, position: usize) -> &[usize] {
        &self.succs[position]
    }

    /// The predecessors of a block, by position, in the order the blocks are allocated.
    pub fn predecessors(&self, position: usize) -> &[usize] {
        &self.preds[position]
    }

    /// The blocks reached from the entry, in reverse postorder, by position.
    pub fn reverse_postorder(&self) -> &[usize] {
        &self.rpo
    }
}

/// The blocks a terminator goes to, in the order it lists them.
pub(crate) fn targets(term: &Terminator) -> Vec<&BlockTarget> {
    match term {
        Terminator::Goto { target, .. } => vec![target],
        Terminator::Branch { then_, else_, .. } => vec![then_, else_],
        Terminator::Switch {
            arms, otherwise, ..
        } => {
            arms.iter()
                .map(|(_, target)| target)
                .chain([otherwise])
                .collect()
        },
        Terminator::Return { .. } | Terminator::Unreachable { .. } => Vec::new(),
    }
}

/// The blocks reached from the entry, in reverse postorder.
fn reverse_postorder(entry: usize, succs: &[Vec<usize>]) -> Vec<usize> {
    let mut visited = vec![false; succs.len()];
    let mut postorder = Vec::new();
    let mut stack = vec![(entry, 0)];

    visited[entry] = true;

    while let Some((block, next)) = stack.last_mut() {
        if *next < succs[*block].len() {
            let child = succs[*block][*next];
            *next += 1;

            if !visited[child] {
                visited[child] = true;
                stack.push((child, 0));
            }
        } else {
            postorder.push(*block);
            stack.pop();
        }
    }

    postorder.reverse();
    postorder
}

#[cfg(test)]
mod tests {
    use mlkc_hir_ty::Ty;
    use mlkc_span::Span;

    use super::*;
    use crate::{
        Block, BlockId, BlockTarget, Body, BodyBuilder, Const, Operand, Place, Rvalue, Stmt,
        StmtKind, Terminator, ValueData,
    };

    /// A body of four blocks: an entry that branches to two of them, and a fourth that nothing
    /// reaches.
    fn body_of_four_blocks() -> (Body, BlockId, BlockId, BlockId) {
        let mut builder = BodyBuilder::new(crate::test_support::owner());
        let cond = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let second = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Return {
                value: Operand::Value(cond),
                span: Span::dummy(),
            },
        });
        let third = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let unreachable = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });

        *builder.block_mut(entry) = Block {
            params: Vec::new(),
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Value(cond),
                    rvalue: Rvalue::Const(Const::Unit),
                },
                span: Span::dummy(),
            }],
            term: Terminator::Branch {
                cond: Operand::Const(Const::Unit),
                then_: BlockTarget {
                    block: second,
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: third,
                    args: Vec::new(),
                },
                span: Span::dummy(),
            },
        };

        (builder.finish(entry), entry, third, unreachable)
    }

    #[test]
    fn the_edges_of_a_body_are_read_both_ways() {
        let (body, entry, third, unreachable) = body_of_four_blocks();
        let cfg = Cfg::of(&body);
        let entry = entry.index();
        let third = third.index();
        let unreachable = unreachable.index();

        assert_eq!(cfg.successors(entry).len(), 2);
        assert_eq!(cfg.predecessors(third), [entry]);

        // The blocks reached from the entry are in reverse postorder, the entry first; a block
        // nothing reaches is in the graph and is not in the order.
        assert_eq!(cfg.reverse_postorder().len(), 3);
        assert_eq!(cfg.reverse_postorder()[0], entry);
        assert!(!cfg.reverse_postorder().contains(&unreachable));
    }
}
