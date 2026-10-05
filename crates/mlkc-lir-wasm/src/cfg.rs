//! The shape of a body's graph.
//!
//! The edges of every block and an order to walk them in are what the verifier, the selection,
//! and every analysis over a body read. They are computed once, here, and read by position: a
//! position is what an arena id is, and an algorithm that walks the graph is written over
//! positions ([`mlkc_la_arena::Idx::index`]).
//!
//! A block that no path from the entry reaches is in the graph all the same --- an edge to it
//! exists, and an analysis may look at it --- but it is not in the reverse postorder, and the
//! promises of the form are vacuous for it: there is no path for them to be about.

#[cfg(test)]
use crate::Terminator;
use crate::{BlockId, Body};

/// The edges of a body, and the order a walk visits its blocks in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    /// The successors of every block, by position, in the order the terminator lists them.
    succs: Vec<Vec<usize>>,
    /// The predecessors of every block, by position, in the order the blocks are allocated.
    preds: Vec<Vec<usize>>,
    /// The blocks reached from the entry, in reverse postorder, by position.
    rpo: Vec<usize>,
    /// The immediate dominator of every reachable block, by position.
    idom: Vec<Option<usize>>,
    /// The children of every block in the dominator tree, by position, in reverse postorder.
    children: Vec<Vec<usize>>,
}

impl Cfg {
    /// The graph of `body`.
    pub fn of(body: &Body) -> Self {
        let mut succs = vec![Vec::new(); body.blocks.len()];
        let mut preds = vec![Vec::new(); body.blocks.len()];

        for (id, block) in body.blocks.iter() {
            let from = id.index();

            for target in block.term.targets() {
                let to = target.block.index();

                if to < body.blocks.len() {
                    succs[from].push(to);
                    preds[to].push(from);
                }
            }
        }

        let rpo = reverse_postorder(body.entry.index(), &succs);
        let mut number = vec![usize::MAX; body.blocks.len()];

        for (position, block) in rpo.iter().enumerate() {
            number[*block] = position;
        }

        let idom = dominators(body.entry.index(), &rpo, &preds, &number);
        let mut children = vec![Vec::new(); body.blocks.len()];

        for (block, parent) in idom.iter().enumerate() {
            if let Some(parent) = parent
                && *parent != block
            {
                children[*parent].push(block);
            }
        }

        for siblings in &mut children {
            siblings.sort_unstable_by_key(|block| number[*block]);
        }

        Self {
            succs,
            preds,
            rpo,
            idom,
            children,
        }
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

    /// The position of a block in the reverse postorder, or `None` where no path reaches it.
    pub fn position_of(&self, block: BlockId) -> Option<usize> {
        self.rpo.iter().position(|it| *it == block.index())
    }

    /// Whether `dominator` dominates `dominated`.
    ///
    /// A block dominates itself, and a block no path from the entry reaches is dominated by
    /// itself alone: there is no path to it for the promise to be about.
    pub fn dominates(&self, dominator: BlockId, dominated: BlockId) -> bool {
        let mut block = dominated.index();

        loop {
            if block == dominator.index() {
                return true;
            }

            match self.idom[block] {
                Some(next) if next != block => block = next,
                _ => return false,
            }
        }
    }

    /// The children of a block in the dominator tree, by position, in reverse postorder.
    pub fn dominator_children(&self, position: usize) -> &[usize] {
        &self.children[position]
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

/// The immediate dominator of every reachable block, in the style of Cooper, Harvey, and
/// Kennedy: iterate the reverse postorder until nothing changes.
///
/// The entry dominates itself, and a block no path reaches has none.
fn dominators(
    entry: usize,
    rpo: &[usize],
    preds: &[Vec<usize>],
    number: &[usize],
) -> Vec<Option<usize>> {
    let mut idom = vec![None; number.len()];
    idom[entry] = Some(entry);

    loop {
        let mut changed = false;

        for &block in rpo {
            if block == entry {
                continue;
            }

            let mut new = None;

            for &pred in &preds[block] {
                if idom[pred].is_none() {
                    continue;
                }

                new = Some(match new {
                    None => pred,
                    Some(current) => intersect(current, pred, &idom, number),
                });
            }

            if new != idom[block] {
                idom[block] = new;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    idom
}

/// The common dominator of two blocks, in the style of Cooper, Harvey, and Kennedy.
fn intersect(a: usize, b: usize, idom: &[Option<usize>], number: &[usize]) -> usize {
    let mut a = a;
    let mut b = b;

    while a != b {
        while number[a] > number[b] {
            a = idom[a].expect("a reachable block has a dominator");
        }

        while number[b] > number[a] {
            b = idom[b].expect("a reachable block has a dominator");
        }
    }

    a
}

#[cfg(test)]
mod tests {
    use mlkc_span::Span;

    use super::*;
    use crate::{Block, BlockTarget, BodyBuilder, Ty, ValueData};

    /// A body of four blocks: an entry that branches to two of them, and a fourth that nothing
    /// reaches.
    #[test]
    fn the_edges_of_a_body_are_read_both_ways() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let cond = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let second = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: cond,
                span: Span::dummy(),
            },
        });
        let third = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let unreachable = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Branch {
            cond,
            then_: BlockTarget {
                block: second,
                args: Vec::new(),
            },
            else_: BlockTarget {
                block: third,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };

        let body = builder.finish(entry);
        let cfg = Cfg::of(&body);

        assert_eq!(cfg.successors(entry.index()).len(), 2);
        assert_eq!(cfg.predecessors(third.index()), [entry.index()]);
        assert_eq!(cfg.reverse_postorder().len(), 3);
        assert_eq!(cfg.reverse_postorder()[0], entry.index());
        assert!(!cfg.reverse_postorder().contains(&unreachable.index()));
        assert_eq!(cfg.position_of(third), Some(1));
        assert_eq!(cfg.position_of(unreachable), None);
    }

    /// A body of a diamond: the entry branches to two arms, and the arms meet in a fourth
    /// block nothing else enters.
    #[test]
    fn the_dominators_of_a_body_are_read_off_the_graph() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let cond = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let then_ = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let else_ = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let join = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: cond,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Branch {
            cond,
            then_: BlockTarget {
                block: then_,
                args: Vec::new(),
            },
            else_: BlockTarget {
                block: else_,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };
        builder.block_mut(then_).term = Terminator::Goto {
            target: BlockTarget {
                block: join,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };
        builder.block_mut(else_).term = Terminator::Goto {
            target: BlockTarget {
                block: join,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };

        let body = builder.finish(entry);
        let cfg = Cfg::of(&body);

        // The entry dominates everything, an arm dominates nothing but itself, and the join is
        // a child of the entry in the dominator tree.
        assert!(cfg.dominates(entry, join));
        assert!(cfg.dominates(then_, then_));
        assert!(!cfg.dominates(then_, join));
        assert!(!cfg.dominates(else_, then_));
        assert_eq!(cfg.dominator_children(entry.index()).len(), 3);
        assert!(
            cfg.dominator_children(entry.index())
                .contains(&join.index())
        );
        assert!(cfg.dominator_children(then_.index()).is_empty());
    }
}
