//! Structuring the control flow of a body ([ADR-0022][adr-0022]).
//!
//! WASM has no arbitrary control flow: its branches name a frame of a tree of `block`, `loop`,
//! and `if`. This pass builds that tree out of the graph, so that encoding writes it and the
//! module needs no dispatch loop and no program counter.
//!
//! The algorithm is the one of Norman Ramsey's *Beyond Relooper: recursive translation of
//! unstructured control flow to structured control flow* (ICFP 2022), as Waffle's `stackify`
//! implements it (<https://github.com/bytecodealliance/waffle>, Apache-2.0 WITH LLVM-exception,
//! which the Apache-2.0 half of this crate's licence is compatible with):
//!
//! - a block more than one forward branch goes to is a join, and the walk opens a `block` whose
//!   label it is, so that every branch to it leaves the region the join ends;
//! - a block a backward branch enters is a loop header, and the walk opens a `loop` whose label
//!   it is, so that the back edge goes to its top;
//! - a branch to neither is to a block the branch dominates, and the walk emits that block and
//!   its subtree right where the branch stands;
//! - a conditional whose arms are both branches becomes an `if`, whose arms are what each of
//!   the branches becomes.
//!
//! A body whose graph is not reducible --- a backward branch into a block that does not
//! dominate it --- and a body that switches have no structure here, and are left to the
//! dispatch form of the encoder.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use std::cmp::Reverse;

use mlkc_lir_wasm::{BlockId, Body, Cfg, Node, Structure, Terminator, ValueId};
use mlkc_span::Span;

/// Structures the control flow of a body, or nothing where the body is not one this builds.
pub(crate) fn run(body: &Body) -> Option<Structure> {
    // A `switch` is a `br_table` over the arms, which is a region of its own, and not one this
    // pass builds; the dispatch form of the encoder emits it.
    if body
        .blocks
        .iter()
        .any(|(_, block)| matches!(block.term, Terminator::Switch { .. }))
    {
        return None;
    }

    let cfg = Cfg::of(body);
    let ids: Vec<BlockId> = body.blocks.iter().map(|(id, _)| id).collect();
    let mut joins = vec![false; body.blocks.len()];
    let mut headers = vec![false; body.blocks.len()];
    let mut branched = vec![false; body.blocks.len()];

    for (position, &at) in cfg.reverse_postorder().iter().enumerate() {
        let from = ids[at];

        for target in body.blocks[from].term.targets() {
            let to = cfg
                .position_of(target.block)
                .expect("a target of a reachable block to be reachable");
            let at = target.block.index();

            if to <= position {
                // A backward branch is a loop, and the body is not reducible if the block it
                // enters does not dominate the block the branch leaves.
                if !cfg.dominates(target.block, from) {
                    return None;
                }

                headers[at] = true;
            } else if branched[at] {
                joins[at] = true;
            } else {
                branched[at] = true;
            }
        }
    }

    Some(
        Builder {
            body,
            cfg,
            ids,
            joins,
            headers,
            frames: Vec::new(),
            steps: Vec::new(),
            results: Vec::new(),
            joins_below: Vec::new(),
        }
        .compute(),
    )
}

/// One frame of the structure being built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frame {
    /// A `block` whose label is the block its branches leave the region for.
    Block { out: BlockId },
    /// A `loop` whose label is its header.
    Loop { header: BlockId },
    /// An `if`, whose label is the end of the arm being written and which no branch names.
    If,
}

impl Frame {
    /// The block a branch to this frame goes to, where the frame is one.
    fn label(self) -> Option<BlockId> {
        match self {
            Self::Block { out } => Some(out),
            Self::Loop { header } => Some(header),
            Self::If => None,
        }
    }
}

/// One step of the walk.
///
/// The walk is written over a stack of steps rather than as recursion: a body may be deeply
/// nested, and the compiler runs in a place with a stack of its own.
#[derive(Debug, Clone)]
enum Step {
    /// Walk the dominator subtree of a block, opening a frame for it where it is one.
    Subtree { block: BlockId },
    /// The subtree is done: the joins it was opened for are forgotten.
    EndSubtree,
    /// Emit the block, and then the first `start` joins of the subtree are behind it.
    Within { block: BlockId, start: usize },
    /// Close a `loop` over the body being built.
    FinishLoop { header: BlockId },
    /// Close a `block` over the body being built.
    FinishBlock { out: BlockId },
    /// The arm of the `if` that was being written is done, and the other one begins.
    Else,
    /// Close an `if` over the two arms.
    FinishIf { cond: ValueId, span: Span },
    /// Emit where an edge goes, from the block it leaves.
    Branch {
        from: BlockId,
        target: BlockId,
        args: Vec<ValueId>,
        span: Span,
    },
}

/// The one structuring of one body.
struct Builder<'a> {
    body: &'a Body,
    cfg: Cfg,
    /// The block every position names.
    ids: Vec<BlockId>,
    /// The joins of the body, by the block's place in its arena.
    joins: Vec<bool>,
    /// The loop headers of the body, by the block's place in its arena.
    headers: Vec<bool>,
    /// The frames around what is being written, innermost last.
    frames: Vec<Frame>,
    /// The steps that are left, innermost first.
    steps: Vec<Step>,
    /// The nodes being built, innermost last.
    results: Vec<Vec<Node>>,
    /// The joins the subtree being walked was opened for, innermost last.
    joins_below: Vec<Vec<BlockId>>,
}

impl<'a> Builder<'a> {
    /// Walks the graph and builds the structure.
    fn compute(mut self) -> Structure {
        self.results.push(Vec::new());
        self.steps.push(Step::Subtree {
            block: self.body.entry,
        });

        while let Some(step) = self.steps.pop() {
            self.step(step);
        }

        Structure {
            nodes: self.results.pop().expect("the walk to end where it began"),
        }
    }

    /// Runs one step.
    fn step(&mut self, step: Step) {
        match step {
            Step::Subtree { block } => self.subtree(block),
            Step::EndSubtree => {
                self.joins_below.pop();
            },
            Step::Within { block, start } => self.within(block, start),
            Step::FinishLoop { header } => {
                self.frames.pop();

                let body = self.results.pop().expect("the body of the loop");

                self.results
                    .last_mut()
                    .expect("a loop to be written somewhere")
                    .push(Node::Loop { header, body });
            },
            Step::FinishBlock { out } => {
                self.frames.pop();

                let body = self.results.pop().expect("the body of the block");

                self.results
                    .last_mut()
                    .expect("a block to be written somewhere")
                    .push(Node::Block { out, body });
            },
            Step::Else => self.results.push(Vec::new()),
            Step::FinishIf { cond, span } => {
                let else_ = self.results.pop().expect("the else arm");
                let then_ = self.results.pop().expect("the then arm");

                self.frames.pop();

                self.results
                    .last_mut()
                    .expect("an if to be written somewhere")
                    .push(Node::If {
                        cond,
                        then_,
                        else_,
                        span,
                    });
            },
            Step::Branch {
                from,
                target,
                args,
                span,
            } => self.branch(from, target, args, span),
        }
    }

    /// Walks the dominator subtree of a block.
    ///
    /// The joins of the subtree are the children of the block in the dominator tree that more
    /// than one branch goes to, innermost first: the first of them is the label a `block` is
    /// opened for, and everything before it is what the block holds.
    fn subtree(&mut self, block: BlockId) {
        let mut children: Vec<BlockId> = self
            .cfg
            .dominator_children(block.index())
            .iter()
            .map(|at| self.ids[*at])
            .filter(|child| self.joins[child.index()])
            .collect();

        children.sort_unstable_by_key(|child| {
            Reverse(
                self.cfg
                    .position_of(*child)
                    .expect("a child of the tree to be reachable"),
            )
        });

        let header = block.index();

        self.joins_below.push(children);
        self.steps.push(Step::EndSubtree);

        if self.headers[header] {
            self.frames.push(Frame::Loop { header: block });
            self.results.push(Vec::new());
            self.steps.push(Step::FinishLoop { header: block });
            self.steps.push(Step::Within { block, start: 0 });
        } else {
            self.steps.push(Step::Within { block, start: 0 });
        }
    }

    /// Emits the block, and everything up to the next join of the subtree.
    fn within(&mut self, block: BlockId, start: usize) {
        let join = self
            .joins_below
            .last()
            .expect("a subtree to be walked")
            .get(start)
            .copied();

        let Some(join) = join else {
            self.leaf(block);

            return;
        };

        self.steps.push(Step::Subtree { block: join });
        self.frames.push(Frame::Block { out: join });
        self.results.push(Vec::new());
        self.steps.push(Step::FinishBlock { out: join });
        self.steps.push(Step::Within {
            block,
            start: start + 1,
        });
    }

    /// Emits the instructions of a block, and what its terminator becomes.
    fn leaf(&mut self, block: BlockId) {
        self.results
            .last_mut()
            .expect("a leaf to be written somewhere")
            .push(Node::Leaf { block });

        let term = &self.body.blocks[block].term;

        match term {
            Terminator::Goto { target, span } => {
                self.steps.push(Step::Branch {
                    from: block,
                    target: target.block,
                    args: target.args.clone(),
                    span: *span,
                });
            },
            Terminator::Branch {
                cond,
                then_,
                else_,
                span,
            } => {
                self.frames.push(Frame::If);
                self.steps.push(Step::FinishIf {
                    cond: *cond,
                    span: *span,
                });
                self.steps.push(Step::Branch {
                    from: block,
                    target: else_.block,
                    args: else_.args.clone(),
                    span: *span,
                });
                self.steps.push(Step::Else);
                self.steps.push(Step::Branch {
                    from: block,
                    target: then_.block,
                    args: then_.args.clone(),
                    span: *span,
                });
                self.results.push(Vec::new());
            },
            Terminator::Switch { .. } => {
                unreachable!("a body that switches is not structured, and not selected")
            },
            Terminator::Return { value, span } => {
                self.results
                    .last_mut()
                    .expect("a return to be written somewhere")
                    .push(Node::Return {
                        value: *value,
                        span: *span,
                    });
            },
            Terminator::Unreachable { span } => {
                self.results
                    .last_mut()
                    .expect("an unreachable to be written somewhere")
                    .push(Node::Unreachable { span: *span });
            },
        }
    }

    /// Emits where an edge goes: a branch to the frame of its target, or the target's subtree
    /// where the edge dominates it and no other edge enters it.
    fn branch(&mut self, from: BlockId, target: BlockId, args: Vec<ValueId>, span: Span) {
        let to = self
            .cfg
            .position_of(target)
            .expect("a target of a reachable block to be reachable");
        let position = self
            .cfg
            .position_of(from)
            .expect("the source of an edge to be reachable");

        if self.joins[to] || to <= position {
            let depth = self.resolve(target);

            self.params(target, args, span);
            self.results
                .last_mut()
                .expect("a branch to be written somewhere")
                .push(Node::Br {
                    depth,
                    target,
                    span,
                });
        } else {
            debug_assert!(
                self.cfg.dominates(from, target),
                "a forward branch to a block the block it leaves does not dominate",
            );

            self.params(target, args, span);
            self.steps.push(Step::Subtree { block: target });
        }
    }

    /// Emits the values an edge passes to the parameters of its target.
    fn params(&mut self, target: BlockId, args: Vec<ValueId>, span: Span) {
        if args.is_empty() {
            return;
        }

        self.results
            .last_mut()
            .expect("a transfer to be written somewhere")
            .push(Node::Params { target, args, span });
    }

    /// How many frames out the label of a block stands.
    ///
    /// The count is of the frames themselves, the arms of an `if` included: an `if` is a label
    /// of the target as well, and a branch that leaves it counts it.
    fn resolve(&self, target: BlockId) -> u32 {
        let depth = self
            .frames
            .iter()
            .rev()
            .position(|frame| frame.label() == Some(target))
            .expect("a branch target to be a frame of the walk");

        depth as u32
    }
}

#[cfg(test)]
mod tests {
    use mlkc_lir_wasm::{
        Block, BlockTarget, BodyBuilder, Inst, Op, Terminator, Ty, ValueData, ValueId,
    };
    use mlkc_span::Span;

    use super::run;

    /// A value of `ty` in the arena of `builder`.
    fn value(builder: &mut BodyBuilder, ty: Ty) -> ValueId {
        builder.value(ValueData {
            span: Span::dummy(),
            ty,
        })
    }

    /// A body of a diamond: the entry branches to two arms that each give a value to the join.
    #[test]
    fn a_diamond_becomes_a_block_around_an_if() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let cond = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let then_value = value(&mut builder, Ty::I32);
        let else_value = value(&mut builder, Ty::I32);
        let join_value = value(&mut builder, Ty::I32);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let then_ = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: then_value,
                op: Op::I32Const(1),
                span: Span::dummy(),
            }],
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let else_ = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: else_value,
                op: Op::I32Const(2),
                span: Span::dummy(),
            }],
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let join = builder.block(Block {
            params: vec![join_value],
            insts: Vec::new(),
            term: Terminator::Return {
                value: join_value,
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
                args: vec![then_value],
            },
            span: Span::dummy(),
        };
        builder.block_mut(else_).term = Terminator::Goto {
            target: BlockTarget {
                block: join,
                args: vec![else_value],
            },
            span: Span::dummy(),
        };

        let body = builder.finish(entry);
        let structure = run(&body).expect("the body to be structured");

        // The join is entered from both arms, so a `block` is opened for it; the arms are the
        // two branches of the condition, and both leave the region for the join.
        assert_eq!(
            mlkc_lir_wasm::dump::structure(&structure),
            "  block b3:\n    leaf b0\n    if v0:\n      leaf b1\n      params b3(v1)\n      br 1 -> b3\n    else:\n      leaf b2\n      params b3(v2)\n      br 1 -> b3\n  leaf b3\n  return v3\n",
        );
    }

    /// A body of a loop: the entry enters a block that branches back to itself or out of it.
    #[test]
    fn a_back_edge_becomes_a_loop() {
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
        let header = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let out = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: cond,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Goto {
            target: BlockTarget {
                block: header,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };
        builder.block_mut(header).term = Terminator::Branch {
            cond,
            then_: BlockTarget {
                block: header,
                args: Vec::new(),
            },
            else_: BlockTarget {
                block: out,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };

        let body = builder.finish(entry);
        let structure = run(&body).expect("the body to be structured");

        // The back edge keeps its header, the block that leaves the loop is emitted where the
        // other arm stands, and the branch back counts the arm of the `if` it stands in.
        assert_eq!(
            mlkc_lir_wasm::dump::structure(&structure),
            "  leaf b0\n  loop b1:\n    leaf b1\n    if v0:\n      br 1 -> b1\n    else:\n      leaf b2\n      return v0\n",
        );
    }

    /// A body whose second of two blocks is entered twice by forward branches is a join even
    /// where nothing follows it.
    #[test]
    fn a_join_without_a_frame_is_still_a_join() {
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
                block: join,
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

        let body = builder.finish(entry);
        let structure = run(&body).expect("the body to be structured");

        assert_eq!(
            mlkc_lir_wasm::dump::structure(&structure),
            "  block b2:\n    leaf b0\n    if v0:\n      leaf b1\n      br 1 -> b2\n    else:\n      br 1 -> b2\n  leaf b2\n  return v0\n",
        );
        assert_eq!(
            body.validate_structure(),
            Err(mlkc_lir_wasm::Invalid::NotStructured)
        );

        // The structure is the one the body would be encoded from: the check reads it off the
        // body, so it is written back before it is validated.
        let mut body = body;

        body.structure = Some(structure);

        assert_eq!(body.validate_structure(), Ok(()));
    }

    /// A body that switches has no structure: the dispatch form of the encoder writes it.
    #[test]
    fn a_body_that_switches_has_no_structure() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let scrutinee = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let value_ = builder.param(ValueData {
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
        let one = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: value_,
                span: Span::dummy(),
            },
        });
        let otherwise = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: value_,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Switch {
            scrutinee,
            arms: vec![(1, BlockTarget {
                block: one,
                args: Vec::new(),
            })],
            otherwise: BlockTarget {
                block: otherwise,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };

        let body = builder.finish(entry);

        assert!(run(&body).is_none(), "a switch to be left to the dispatch");
    }
}
