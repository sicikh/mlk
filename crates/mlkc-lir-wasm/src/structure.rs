//! The structured control flow of a body ([ADR-0022][adr-0022]).
//!
//! The LIR is a control-flow graph, and WASM is not: its control flow is a tree of `block`,
//! `loop`, and `if` frames, and a branch names the frame it leaves. The structure of a body
//! is that tree: the plan the structuring pass builds, and what encoding writes out. It is a
//! record of a decision, like the [`Locals`](crate::Locals) table, and not a second form of the
//! IR: the blocks and their instructions are the body's, and a [`Node::Leaf`] is where the
//! instructions of one of them are emitted.
//!
//! The algorithm is the one of Ramsey's *Beyond Relooper* --- a dominator-tree walk that opens
//! a `block` for every join and a `loop` for every header, as Waffle's `stackify` does it
//! ([ADR-0022][adr-0022]).
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_span::Span;

use crate::{BlockId, Body, ValueId, cfg::Cfg, ty::Ty, verify::Invalid};

/// The structured control flow of a body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Structure {
    /// The nodes of the body, in the order they are emitted.
    pub nodes: Vec<Node>,
}

/// One frame of the structured control flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A `block`: a region whose branches to `out` leave it.
    ///
    /// The code that follows the region is the code of `out`.
    Block {
        /// The block the region's branches go to.
        out: BlockId,
        /// What the region holds.
        body: Vec<Node>,
    },
    /// A `loop`: a region whose branches to `header` go back to its top.
    Loop {
        /// The block the region repeats.
        header: BlockId,
        /// What the region holds.
        body: Vec<Node>,
    },
    /// An `if` over a condition.
    ///
    /// The first arm is emitted between `if` and `else`, and the second between `else` and
    /// `end`; both end in a branch, a return, or an `unreachable`, so nothing falls out.
    If {
        /// The condition: an `i32` the target reads as true when it is not zero.
        cond: ValueId,
        /// Where control goes when the condition holds.
        then_: Vec<Node>,
        /// Where control goes when it does not.
        else_: Vec<Node>,
        /// Where the `if` was read from.
        span: Span,
    },
    /// The instructions of one block, emitted where it stands.
    ///
    /// The terminator of the block is not here: it is what the nodes around the leaf are.
    Leaf {
        /// The block.
        block: BlockId,
    },
    /// The values an edge passes, as the parameters of the block it goes to.
    Params {
        /// The block the edge goes to.
        target: BlockId,
        /// The arguments, one per parameter of the target, in the order of the parameters.
        args: Vec<ValueId>,
        /// Where the edge was read from.
        span: Span,
    },
    /// A branch to a label: the frame `depth` frames out, which is the frame of `target`.
    Br {
        /// How many frames out the label stands.
        depth: u32,
        /// The block the label belongs to.
        target: BlockId,
        /// Where the branch was read from.
        span: Span,
    },
    /// The body gives back a value.
    Return {
        /// The value.
        value: ValueId,
        /// Where the terminator was read from.
        span: Span,
    },
    /// Control that is not meant to be reached.
    Unreachable {
        /// Where the terminator was read from.
        span: Span,
    },
}

impl Body {
    /// Checks that the structure of the body says where every reachable block is emitted.
    pub fn validate_structure(&self) -> Result<(), Invalid> {
        let Some(structure) = &self.structure else {
            return Err(Invalid::NotStructured);
        };

        let cfg = Cfg::of(self);
        let mut seen = vec![false; self.blocks.len()];
        let mut frames: Vec<Option<BlockId>> = Vec::new();

        self.check_nodes(&structure.nodes, &mut seen, &mut frames)?;

        // Every block a path from the entry reaches is emitted, and the check above says each
        // is emitted once.
        for (id, _) in self.blocks.iter() {
            if cfg.position_of(id).is_some() && !seen[id.index()] {
                return Err(Invalid::StructureMissing { block: id });
            }
        }

        Ok(())
    }

    /// Checks one list of nodes, with the frames around them.
    fn check_nodes(
        &self,
        nodes: &[Node],
        seen: &mut [bool],
        frames: &mut Vec<Option<BlockId>>,
    ) -> Result<(), Invalid> {
        for node in nodes {
            match node {
                Node::Block { out, body } => {
                    self.check_block(*out)?;
                    frames.push(Some(*out));
                    self.check_nodes(body, seen, frames)?;
                    frames.pop();
                },
                Node::Loop { header, body } => {
                    self.check_block(*header)?;
                    frames.push(Some(*header));
                    self.check_nodes(body, seen, frames)?;
                    frames.pop();
                },
                Node::If {
                    cond, then_, else_, ..
                } => {
                    self.expect(*cond, Ty::I32)?;
                    frames.push(None);
                    self.check_nodes(then_, seen, frames)?;
                    self.check_nodes(else_, seen, frames)?;
                    frames.pop();
                },
                Node::Leaf { block } => {
                    self.check_block(*block)?;

                    if seen[block.index()] {
                        return Err(Invalid::StructureTwice { block: *block });
                    }

                    seen[block.index()] = true;
                },
                Node::Params { target, args, .. } => {
                    self.check_block(*target)?;

                    let params = &self.blocks[*target].params;

                    if args.len() != params.len() {
                        return Err(Invalid::StructureParams {
                            target: *target,
                            expected: params.len(),
                            found: args.len(),
                        });
                    }

                    for (arg, param) in args.iter().zip(params) {
                        let expected = self.values[*param].ty;
                        let found = self.values[*arg].ty;

                        if !found.is_subtype_of(expected) {
                            return Err(Invalid::OperandType {
                                operand: *arg,
                                expected,
                                found,
                            });
                        }
                    }
                },
                Node::Br { depth, target, .. } => {
                    self.check_block(*target)?;

                    // The depth counts the frames from the innermost out, as WASM does.
                    let at = (frames.len() as u32)
                        .checked_sub(*depth + 1)
                        .ok_or(Invalid::StructureBranch { depth: *depth })?;

                    if frames[at as usize] != Some(*target) {
                        return Err(Invalid::StructureTarget {
                            depth: *depth,
                            target: *target,
                        });
                    }
                },
                Node::Return { value, .. } => {
                    let found = self.values[*value].ty;

                    if !found.is_subtype_of(self.ret) {
                        return Err(Invalid::ReturnType {
                            value: *value,
                            expected: self.ret,
                            found,
                        });
                    }
                },
                Node::Unreachable { .. } => {},
            }
        }

        Ok(())
    }

    /// Checks that a block a node names is a block of the body.
    fn check_block(&self, block: BlockId) -> Result<(), Invalid> {
        if block.index() < self.blocks.len() {
            Ok(())
        } else {
            Err(Invalid::MissingBlock { block })
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_span::Span;

    use super::{Node, Structure};
    use crate::{
        Block, BlockTarget, Body, BodyBuilder, Inst, Op, Terminator, Ty, ValueData, ValueId,
        verify::Invalid,
    };

    /// A body of a diamond: the entry branches to two arms that each give a value to the join.
    fn diamond() -> (Body, ValueId, ValueId, ValueId, ValueId) {
        let mut builder = BodyBuilder::new(Ty::I32);
        let cond = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let then_value = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let else_value = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let join_value = builder.value(ValueData {
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

        (
            builder.finish(entry),
            cond,
            then_value,
            else_value,
            join_value,
        )
    }

    /// What the structure of [`diamond`] is: a block around the arms, and the join after it.
    fn structure_of_the_diamond(
        body: &mut Body,
        cond: ValueId,
        then_value: ValueId,
        else_value: ValueId,
        join_value: ValueId,
    ) {
        let entry = body.entry;
        let then_ = body.blocks.iter().nth(1).expect("the then block").0;
        let else_ = body.blocks.iter().nth(2).expect("the else block").0;
        let join = body.blocks.iter().nth(3).expect("the join").0;

        body.structure = Some(Structure {
            nodes: vec![
                Node::Block {
                    out: join,
                    body: vec![Node::Leaf { block: entry }, Node::If {
                        cond,
                        then_: vec![
                            Node::Leaf { block: then_ },
                            Node::Params {
                                target: join,
                                args: vec![then_value],
                                span: Span::dummy(),
                            },
                            Node::Br {
                                depth: 1,
                                target: join,
                                span: Span::dummy(),
                            },
                        ],
                        else_: vec![
                            Node::Leaf { block: else_ },
                            Node::Params {
                                target: join,
                                args: vec![else_value],
                                span: Span::dummy(),
                            },
                            Node::Br {
                                depth: 1,
                                target: join,
                                span: Span::dummy(),
                            },
                        ],
                        span: Span::dummy(),
                    }],
                },
                Node::Leaf { block: join },
                Node::Return {
                    value: join_value,
                    span: Span::dummy(),
                },
            ],
        });
    }

    #[test]
    fn a_structure_that_emits_every_block_holds_the_invariant() {
        let (mut body, cond, then_value, else_value, join_value) = diamond();

        assert_eq!(body.validate_structure(), Err(Invalid::NotStructured));

        structure_of_the_diamond(&mut body, cond, then_value, else_value, join_value);

        assert_eq!(body.validate_structure(), Ok(()));
        assert_eq!(
            crate::dump::structure(body.structure.as_ref().expect("the structure")),
            "  block b3:\n    leaf b0\n    if v0:\n      leaf b1\n      params b3(v1)\n      br 1 -> b3\n    else:\n      leaf b2\n      params b3(v2)\n      br 1 -> b3\n  leaf b3\n  return v3\n",
        );
    }

    #[test]
    fn a_block_no_leaf_emits_is_a_mistake() {
        let (mut body, cond, then_value, else_value, join_value) = diamond();

        structure_of_the_diamond(&mut body, cond, then_value, else_value, join_value);

        let structure = body.structure.as_mut().expect("the structure");

        structure.nodes.pop();
        structure.nodes.pop();

        assert!(matches!(
            body.validate_structure(),
            Err(Invalid::StructureMissing { .. }),
        ));
    }

    #[test]
    fn a_block_two_leaves_emit_is_a_mistake() {
        let (mut body, cond, then_value, else_value, join_value) = diamond();

        structure_of_the_diamond(&mut body, cond, then_value, else_value, join_value);

        let structure = body.structure.as_mut().expect("the structure");

        structure.nodes.push(Node::Leaf { block: body.entry });

        assert!(matches!(
            body.validate_structure(),
            Err(Invalid::StructureTwice { .. }),
        ));
    }

    #[test]
    fn a_branch_to_a_frame_that_is_not_the_label_is_a_mistake() {
        let (mut body, cond, then_value, else_value, join_value) = diamond();

        structure_of_the_diamond(&mut body, cond, then_value, else_value, join_value);

        // The frame at depth zero is the `if`, which is not the label the branch names.
        set_branch_depth(&mut body, 0);

        assert!(matches!(
            body.validate_structure(),
            Err(Invalid::StructureTarget { depth: 0, .. }),
        ));

        // And a depth past the outermost frame is no frame at all.
        set_branch_depth(&mut body, 2);

        assert!(matches!(
            body.validate_structure(),
            Err(Invalid::StructureBranch { depth: 2 }),
        ));
    }

    /// Writes the depth of the branch of the then arm of the structure.
    fn set_branch_depth(body: &mut Body, depth: u32) {
        let Some(Node::Block { body: nodes, .. }) =
            body.structure.as_mut().map(|it| &mut it.nodes[0])
        else {
            panic!("the block of the structure");
        };
        let Some(Node::If { then_, .. }) = nodes.get_mut(1) else {
            panic!("the if of the structure");
        };
        let Node::Br { depth: asked, .. } = &mut then_[2] else {
            panic!("the branch of the arm");
        };

        *asked = depth;
    }
}
