//! The verifier of the LIR: what tells a well-formed body from one that is not.
//!
//! The LIR has one form --- SSA over machine values --- and the verifier checks it: every value
//! is defined once, every use is dominated by its definition, every instruction reads the types
//! it takes, every edge passes the types its target wants, and the allocation table, when the
//! allocation pass has filled it, says where every value that needs storage lives.
//!
//! It is what the back end relies on before it encodes a body ([ADR-0022][adr-0022]): selection,
//! the passes, and encoding are all written against the invariant this checks.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use std::fmt;

use crate::{
    BlockId, Body, Op, Terminator, ValueId,
    cfg::Cfg,
    ty::{RefTy, Ty},
};

/// What a body that does not hold the invariant of the LIR is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invalid {
    /// The body is entered at a block it does not have.
    MissingEntry {
        /// The block the body enters.
        entry: BlockId,
    },
    /// An edge goes to a block the body does not have.
    MissingBlock {
        /// The block the edge goes to.
        block: BlockId,
    },
    /// An instruction reads a value the body does not have.
    MissingValue {
        /// The value.
        value: ValueId,
    },
    /// An edge passes a number of arguments its target does not take.
    EdgeArity {
        /// The block the edge leaves.
        block: BlockId,
        /// The block the edge goes to.
        target: BlockId,
        /// How many arguments the target takes.
        expected: usize,
        /// How many the edge passes.
        found: usize,
    },
    /// An edge passes a value of a type its target does not accept.
    EdgeType {
        /// The block the edge leaves.
        block: BlockId,
        /// The block the edge goes to.
        target: BlockId,
        /// The value the edge passes.
        value: ValueId,
        /// What the parameter takes.
        expected: Ty,
        /// What the argument is.
        found: Ty,
    },
    /// The type a value is declared to have is not the type its instruction produces.
    ResultType {
        /// The value.
        value: ValueId,
        /// What the instruction produces.
        expected: Ty,
        /// What the value is declared to be.
        found: Ty,
    },
    /// An instruction reads an operand of a type it does not take.
    OperandType {
        /// The operand.
        operand: ValueId,
        /// What the instruction takes.
        expected: Ty,
        /// What the operand is.
        found: Ty,
    },
    /// The body gives back a value its result type does not accept.
    ReturnType {
        /// The value.
        value: ValueId,
        /// What the body gives back.
        expected: Ty,
        /// What the value is.
        found: Ty,
    },
    /// A value is defined more than once.
    ValueDefinedTwice {
        /// The value.
        value: ValueId,
    },
    /// A value is read, and nothing defines it.
    ValueNotDefined {
        /// The value.
        value: ValueId,
    },
    /// A use of a value is not dominated by its definition.
    NotDominated {
        /// The value.
        value: ValueId,
        /// The block the use is in.
        block: BlockId,
    },
    /// A value is defined after a use of it in the same block.
    DefinedAfterUse {
        /// The value.
        value: ValueId,
        /// The block.
        block: BlockId,
    },
    /// The allocation pass has not run: the encoder needs to know where a value lives.
    NotAllocated,
    /// The allocation pass left a value without a local where it needs one, or a local it gave
    /// does not exist.
    MissingAllocation {
        /// The value.
        value: ValueId,
    },
    /// A value does not have the type of the local it was given.
    AllocatedType {
        /// The value.
        value: ValueId,
        /// The type of the local.
        expected: Ty,
        /// The type of the value.
        found: Ty,
    },
    /// The program counter of a dispatched body is not a local of the body.
    MissingPc {
        /// The local the pass named.
        pc: u32,
    },
    /// The scratch local of a body that switches is not a local of the body.
    MissingScratch {
        /// The local the pass named.
        scratch: u32,
    },
    /// A body with no structure is asked about its structure.
    NotStructured,
    /// A block a path from the entry reaches is emitted by no leaf.
    StructureMissing {
        /// The block.
        block: BlockId,
    },
    /// A block is emitted by more than one leaf.
    StructureTwice {
        /// The block.
        block: BlockId,
    },
    /// An edge passes a number of arguments the block it goes to does not take.
    StructureParams {
        /// The block the edge goes to.
        target: BlockId,
        /// How many arguments the target takes.
        expected: usize,
        /// How many the edge passes.
        found: usize,
    },
    /// A branch names a frame that is not around it.
    StructureBranch {
        /// How many frames out the branch goes.
        depth: u32,
    },
    /// A branch names a frame that belongs to another block.
    StructureTarget {
        /// How many frames out the branch goes.
        depth: u32,
        /// The block the branch says the label belongs to.
        target: BlockId,
    },
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry { entry } => {
                write!(
                    f,
                    "the body is entered at {}, which is not a block of it",
                    block_text(*entry),
                )
            },
            Self::MissingBlock { block } => {
                write!(
                    f,
                    "an edge goes to {}, which is not a block of the body",
                    block_text(*block),
                )
            },
            Self::MissingValue { value } => {
                write!(
                    f,
                    "{} is read, and the body has no such value",
                    value_text(*value),
                )
            },
            Self::EdgeArity {
                block,
                target,
                expected,
                found,
            } => {
                write!(
                    f,
                    "an edge of {} to {} passes {found} arguments, and the target takes {expected}",
                    block_text(*block),
                    block_text(*target),
                )
            },
            Self::EdgeType {
                block,
                target,
                value,
                expected,
                found,
            } => {
                write!(
                    f,
                    "an edge of {} to {} passes {} of `{found}`, and the parameter takes \
                     `{expected}`",
                    block_text(*block),
                    block_text(*target),
                    value_text(*value),
                )
            },
            Self::ResultType {
                value,
                expected,
                found,
            } => {
                write!(
                    f,
                    "{} is `{found}`, and the instruction that defines it produces `{expected}`",
                    value_text(*value),
                )
            },
            Self::OperandType {
                operand,
                expected,
                found,
            } => {
                write!(
                    f,
                    "{} is `{found}`, and the instruction that reads it takes `{expected}`",
                    value_text(*operand),
                )
            },
            Self::ReturnType {
                value,
                expected,
                found,
            } => {
                write!(
                    f,
                    "the body returns {} of `{found}`, and gives back `{expected}`",
                    value_text(*value),
                )
            },
            Self::ValueDefinedTwice { value } => {
                write!(f, "{} is defined more than once", value_text(*value))
            },
            Self::ValueNotDefined { value } => {
                write!(f, "{} is read, and nothing defines it", value_text(*value))
            },
            Self::NotDominated { value, block } => {
                write!(
                    f,
                    "a use of {} in {} is not dominated by its definition",
                    value_text(*value),
                    block_text(*block),
                )
            },
            Self::DefinedAfterUse { value, block } => {
                write!(
                    f,
                    "{} is defined after a use of it in {}",
                    value_text(*value),
                    block_text(*block),
                )
            },
            Self::NotAllocated => {
                f.write_str("the allocation pass has not run over every value of the body")
            },
            Self::MissingAllocation { value } => {
                write!(
                    f,
                    "{} needs a local, and the allocation gave it none",
                    value_text(*value),
                )
            },
            Self::AllocatedType {
                value,
                expected,
                found,
            } => {
                write!(
                    f,
                    "{} of `{found}` was given a local of `{expected}`",
                    value_text(*value),
                )
            },
            Self::MissingPc { pc } => {
                write!(
                    f,
                    "the program counter names local {pc}, which the body has not"
                )
            },
            Self::MissingScratch { scratch } => {
                write!(
                    f,
                    "the scratch local is local {scratch}, which the body has not",
                )
            },
            Self::NotStructured => f.write_str("the body has no structure to read"),
            Self::StructureMissing { block } => {
                write!(
                    f,
                    "{} is reached by a path, and no leaf of the structure emits it",
                    block_text(*block),
                )
            },
            Self::StructureTwice { block } => {
                write!(
                    f,
                    "{} is emitted by more than one leaf of the structure",
                    block_text(*block),
                )
            },
            Self::StructureParams {
                target,
                expected,
                found,
            } => {
                write!(
                    f,
                    "an edge to {} passes {found} arguments, and the block takes {expected}",
                    block_text(*target),
                )
            },
            Self::StructureBranch { depth } => {
                write!(
                    f,
                    "a branch goes {depth} frames out, and there are not that many",
                )
            },
            Self::StructureTarget { depth, target } => {
                write!(
                    f,
                    "a branch to {} goes {depth} frames out, and that frame is another block",
                    block_text(*target),
                )
            },
        }
    }
}

impl Body {
    /// Checks that the body holds the invariant of the LIR.
    pub fn validate(&self) -> Result<(), Invalid> {
        self.check_ids()?;
        self.check_edges()?;
        self.check_types()?;
        self.check_ssa()
    }

    /// Checks that the allocation table says where every value that needs storage lives.
    ///
    /// A value the allocation inlined has none, and that is a decision, not a mistake: what is
    /// checked is that every parameter has the local of the ABI, that every local a value was
    /// given exists and has the type of the value, and that the special locals of the dispatch
    /// form are locals of the body.
    pub fn validate_locals(&self) -> Result<(), Invalid> {
        if self.locals.values.len() != self.values.len() {
            return Err(Invalid::NotAllocated);
        }

        let parameters = self.params.len() as u32;
        let locals = parameters + self.locals.locals.len() as u32;

        if let Some(pc) = self.locals.pc
            && pc >= locals
        {
            return Err(Invalid::MissingPc { pc });
        }

        if let Some(scratch) = self.locals.scratch
            && scratch >= locals
        {
            return Err(Invalid::MissingScratch { scratch });
        }

        for (value, data) in self.values.iter() {
            let local = self.locals.values[value.index()];

            // A parameter is the local the ABI declared for it, and nothing else may be.
            if let Some(local) = local
                && local < parameters
                && self.params.get(local as usize) != Some(&value)
            {
                return Err(Invalid::MissingAllocation { value });
            }

            let needed = self.params.contains(&value)
                || self
                    .blocks
                    .iter()
                    .any(|(_, block)| block.params.contains(&value));

            if needed && local.is_none() {
                return Err(Invalid::MissingAllocation { value });
            }

            let Some(local) = local else {
                continue;
            };

            if local < parameters {
                continue;
            }

            let Some(ty) = self.locals.locals.get((local - parameters) as usize) else {
                return Err(Invalid::MissingAllocation { value });
            };

            if *ty != data.ty {
                return Err(Invalid::AllocatedType {
                    value,
                    expected: *ty,
                    found: data.ty,
                });
            }
        }

        Ok(())
    }

    /// Checks that the ids a body names are ids of it.
    fn check_ids(&self) -> Result<(), Invalid> {
        if self.entry.index() >= self.blocks.len() {
            return Err(Invalid::MissingEntry { entry: self.entry });
        }

        for (_, block) in self.blocks.iter() {
            for inst in &block.insts {
                self.check_value(inst.value)?;

                for operand in inst.op.operands() {
                    self.check_value(operand)?;
                }
            }

            for operand in block.term.operands() {
                self.check_value(operand)?;
            }

            for target in block.term.targets() {
                if target.block.index() >= self.blocks.len() {
                    return Err(Invalid::MissingBlock {
                        block: target.block,
                    });
                }
            }
        }

        Ok(())
    }

    /// Checks that every edge passes the arguments its target takes.
    fn check_edges(&self) -> Result<(), Invalid> {
        for (id, block) in self.blocks.iter() {
            for target in block.term.targets() {
                let destination = &self.blocks[target.block];

                if destination.params.len() != target.args.len() {
                    return Err(Invalid::EdgeArity {
                        block: id,
                        target: target.block,
                        expected: destination.params.len(),
                        found: target.args.len(),
                    });
                }

                for (param, argument) in destination.params.iter().zip(&target.args) {
                    let expected = self.values[*param].ty;
                    let found = self.values[*argument].ty;

                    if !found.is_subtype_of(expected) {
                        return Err(Invalid::EdgeType {
                            block: id,
                            target: target.block,
                            value: *argument,
                            expected,
                            found,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Checks the type of every instruction and of every terminator.
    fn check_types(&self) -> Result<(), Invalid> {
        for (_, block) in self.blocks.iter() {
            for inst in &block.insts {
                self.check_inst(inst)?;
            }

            match &block.term {
                Terminator::Goto { .. } | Terminator::Unreachable { .. } => {},
                Terminator::Branch { cond, .. } => self.expect(*cond, Ty::I32)?,
                Terminator::Switch { scrutinee, .. } => self.expect(*scrutinee, Ty::I32)?,
                Terminator::Return { value, .. } => {
                    let found = self.values[*value].ty;

                    if !found.is_subtype_of(self.ret) {
                        return Err(Invalid::ReturnType {
                            value: *value,
                            expected: self.ret,
                            found,
                        });
                    }
                },
            }
        }

        Ok(())
    }

    /// Checks one instruction: the types it reads, and the type of the value it defines.
    ///
    /// A call is the one instruction whose shape the body does not carry: what a callee takes
    /// and gives back is a property of the module, not of the body ([ADR-0022][adr-0022]), so
    /// what is checked of a call is that its result is a reference --- which is what every
    /// function of the ABI gives back --- and that its arguments are values of the body.
    ///
    /// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
    fn check_inst(&self, inst: &crate::Inst) -> Result<(), Invalid> {
        use Op::*;

        let produced = match &inst.op {
            I32Const(_) => Ty::I32,
            I32Eqz(operand) => {
                self.expect(*operand, Ty::I32)?;

                Ty::I32
            },
            I32Eq(lhs, rhs)
            | I32Ne(lhs, rhs)
            | I32LtS(lhs, rhs)
            | I32LeS(lhs, rhs)
            | I32GtS(lhs, rhs)
            | I32GeS(lhs, rhs)
            | I32Add(lhs, rhs)
            | I32Sub(lhs, rhs)
            | I32Mul(lhs, rhs)
            | I32DivS(lhs, rhs)
            | I32And(lhs, rhs)
            | I32Or(lhs, rhs) => {
                self.expect(*lhs, Ty::I32)?;
                self.expect(*rhs, Ty::I32)?;

                Ty::I32
            },
            RefI31(operand) => {
                self.expect(*operand, Ty::I32)?;

                Ty::I31
            },
            I31GetS(operand) => {
                self.expect(*operand, Ty::I31)?;

                Ty::I32
            },
            RefCast(ty, operand) => {
                // A cast is to a reference that is not null --- the value it produces is read
                // as one --- and what it casts must be one of that type already or a reference
                // of a type it is one of.
                if *ty == RefTy::Eq {
                    return Err(Invalid::OperandType {
                        operand: *operand,
                        expected: Ty::I31,
                        found: Ty::Ref(*ty),
                    });
                }

                let expected = Ty::Ref(*ty);
                let found = self.values[*operand].ty;

                if !found.is_ref() {
                    return Err(Invalid::OperandType {
                        operand: *operand,
                        expected,
                        found,
                    });
                }

                // A cast to a concrete type narrows a word: whether the type is under the one
                // the value already has is the module's hierarchy, which the engine checks.
                let ordered = match (expected, found) {
                    (Ty::Ref(RefTy::Type(_)), _) => true,
                    (Ty::Ref(wanted), Ty::Ref(have)) => {
                        Ty::Ref(wanted).is_subtype_of(Ty::Ref(have))
                    },
                    _ => false,
                };

                if !ordered {
                    return Err(Invalid::OperandType {
                        operand: *operand,
                        expected,
                        found,
                    });
                }

                expected
            },
            RefEq(lhs, rhs) => {
                self.expect_ref(*lhs)?;
                self.expect_ref(*rhs)?;

                Ty::I32
            },
            RefFunc { .. } => {
                // The code of a function is a reference to the type the module declares for
                // it, which the body's context knows and the verifier does not.
                if !self.values[inst.value].ty.is_ref() {
                    return Err(Invalid::ResultType {
                        value: inst.value,
                        expected: Ty::EQREF,
                        found: self.values[inst.value].ty,
                    });
                }

                return Ok(());
            },
            RefNull(ty) => Ty::Ref(*ty),
            StructNew { ty, fields } => {
                for field in fields {
                    self.expect_ref(*field)?;
                }

                Ty::Ref(RefTy::Type(*ty))
            },
            StructGet { value, .. } => {
                // The field's type is the module's; what is checked here is that both sides
                // are references.
                self.expect_ref(*value)?;

                if !self.values[inst.value].ty.is_ref() {
                    return Err(Invalid::ResultType {
                        value: inst.value,
                        expected: Ty::EQREF,
                        found: self.values[inst.value].ty,
                    });
                }

                return Ok(());
            },
            Call { .. } | CallLocal { .. } | CallRef { .. } => {
                // The declared type of the result is what the context of the function says;
                // the instruction checks nothing but that it is a reference.
                if !self.values[inst.value].ty.is_ref() {
                    return Err(Invalid::ResultType {
                        value: inst.value,
                        expected: Ty::EQREF,
                        found: self.values[inst.value].ty,
                    });
                }

                return Ok(());
            },
            String(_) => Ty::EQREF,
        };

        let declared = self.values[inst.value].ty;

        if declared != produced {
            return Err(Invalid::ResultType {
                value: inst.value,
                expected: produced,
                found: declared,
            });
        }

        Ok(())
    }

    /// Checks the invariant of the SSA form: every value is defined once, and every use is
    /// dominated by its definition.
    fn check_ssa(&self) -> Result<(), Invalid> {
        let mut defs: Vec<Option<Def>> = vec![None; self.values.len()];

        for value in &self.params {
            let at = value.index();

            if defs[at].is_some() {
                return Err(Invalid::ValueDefinedTwice { value: *value });
            }

            defs[at] = Some(Def::Block(self.entry));
        }

        for (id, block) in self.blocks.iter() {
            for param in &block.params {
                let value = param.index();

                if defs[value].is_some() {
                    return Err(Invalid::ValueDefinedTwice { value: *param });
                }

                defs[value] = Some(Def::Block(id));
            }

            for (index, inst) in block.insts.iter().enumerate() {
                let defined = inst.value.index();

                if defs[defined].is_some() {
                    return Err(Invalid::ValueDefinedTwice { value: inst.value });
                }

                defs[defined] = Some(Def::Stmt(id, index));
            }
        }

        for (id, _) in self.values.iter() {
            if defs[id.index()].is_none() {
                return Err(Invalid::ValueNotDefined { value: id });
            }
        }

        // Every use is dominated by its definition. A use in a block no path from the entry
        // reaches is not checked: there is no path to it for the promise to be about.
        let cfg = Cfg::of(self);

        for (id, block) in self.blocks.iter() {
            if cfg.position_of(id).is_none() {
                continue;
            }

            for (index, inst) in block.insts.iter().enumerate() {
                for operand in inst.op.operands() {
                    self.check_definition(operand, id, index, &defs, &cfg)?;
                }
            }

            let term = block.insts.len();

            for operand in block.term.operands() {
                self.check_definition(operand, id, term, &defs, &cfg)?;
            }
        }

        Ok(())
    }

    /// Checks that a use of a value at `at` in `block` reads a definition that dominates it.
    fn check_definition(
        &self,
        value: ValueId,
        block: BlockId,
        at: usize,
        defs: &[Option<Def>],
        cfg: &Cfg,
    ) -> Result<(), Invalid> {
        match defs[value.index()].expect("every value is defined") {
            Def::Block(defined) => {
                if !cfg.dominates(defined, block) {
                    return Err(Invalid::NotDominated { value, block });
                }
            },
            Def::Stmt(defined, index) => {
                if defined == block {
                    if index >= at {
                        return Err(Invalid::DefinedAfterUse { value, block });
                    }
                } else if !cfg.dominates(defined, block) {
                    return Err(Invalid::NotDominated { value, block });
                }
            },
        }

        Ok(())
    }

    /// Checks that the type of a value is the one an instruction takes.
    pub(crate) fn expect(&self, value: ValueId, expected: Ty) -> Result<(), Invalid> {
        let found = self.values[value].ty;

        if found != expected {
            return Err(Invalid::OperandType {
                operand: value,
                expected,
                found,
            });
        }

        Ok(())
    }

    /// Checks that a value is a reference of any type.
    fn expect_ref(&self, value: ValueId) -> Result<(), Invalid> {
        let found = self.values[value].ty;

        if !found.is_ref() {
            return Err(Invalid::OperandType {
                operand: value,
                expected: Ty::EQREF,
                found,
            });
        }

        Ok(())
    }

    /// Checks that a value id is an id of the body.
    fn check_value(&self, value: ValueId) -> Result<(), Invalid> {
        if value.index() < self.values.len() {
            Ok(())
        } else {
            Err(Invalid::MissingValue { value })
        }
    }
}

/// Where a value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Def {
    /// A parameter of a body or of a block, defined where the block is entered.
    Block(BlockId),
    /// An instruction, defined where it runs.
    Stmt(BlockId, usize),
}

/// The label of a block, for a message.
fn block_text(block: BlockId) -> String {
    format!("b{}", block.index())
}

/// The label of a value, for a message.
fn value_text(value: ValueId) -> String {
    format!("v{}", value.index())
}

#[cfg(test)]
mod tests {
    use mlkc_span::Span;

    use super::Invalid;
    use crate::{
        Block, BlockTarget, Body, BodyBuilder, Inst, Locals, Op, RefTy, Terminator, Ty, ValueData,
        ValueId,
    };

    /// A value of `ty` in the arena of `builder`.
    fn value(builder: &mut BodyBuilder, ty: Ty) -> ValueId {
        builder.value(ValueData {
            span: Span::dummy(),
            ty,
        })
    }

    /// A body of one block: a cast of a word to an immediate, and the immediate back.
    fn cast_body() -> Body {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let cast = value(&mut builder, Ty::I31);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: cast,
                op: Op::RefCast(RefTy::I31, param),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: cast,
                span: Span::dummy(),
            },
        });

        builder.finish(entry)
    }

    #[test]
    fn a_body_of_one_block_holds_the_invariant() {
        let body = cast_body();

        assert_eq!(body.validate(), Ok(()));
    }

    #[test]
    fn the_allocation_is_checked_once_it_is_there() {
        let mut body = cast_body();

        assert_eq!(body.validate_locals(), Err(Invalid::NotAllocated));

        body.locals = Locals {
            locals: vec![Ty::I31],
            values: vec![Some(0), Some(1)],
            pc: None,
            scratch: None,
        };

        assert_eq!(body.validate_locals(), Ok(()));

        body.locals.locals[0] = Ty::EQREF;

        assert!(matches!(
            body.validate_locals(),
            Err(Invalid::AllocatedType { .. }),
        ));
    }

    #[test]
    fn a_block_parameter_needs_a_local() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let block_param = value(&mut builder, Ty::I32);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let target = builder.block(Block {
            params: vec![block_param],
            insts: Vec::new(),
            term: Terminator::Return {
                value: block_param,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Goto {
            target: BlockTarget {
                block: target,
                args: vec![param],
            },
            span: Span::dummy(),
        };

        let mut body = builder.finish(entry);

        body.locals = Locals {
            locals: Vec::new(),
            values: vec![Some(0), None],
            pc: None,
            scratch: None,
        };

        assert_eq!(
            body.validate_locals(),
            Err(Invalid::MissingAllocation { value: block_param }),
        );
    }

    #[test]
    fn an_edge_with_the_wrong_number_of_arguments_is_a_mistake() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let cast = value(&mut builder, Ty::I31);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: cast,
                op: Op::RefCast(RefTy::I31, param),
                span: Span::dummy(),
            }],
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let target = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: cast,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Goto {
            target: BlockTarget {
                block: target,
                args: vec![cast],
            },
            span: Span::dummy(),
        };

        let body = builder.finish(entry);

        assert!(matches!(
            body.validate(),
            Err(Invalid::EdgeArity {
                expected: 0,
                found: 1,
                ..
            }),
        ));
    }

    #[test]
    fn an_operand_of_the_wrong_type_is_a_mistake() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let word = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let number = value(&mut builder, Ty::I32);
        let sum = value(&mut builder, Ty::I32);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![
                Inst {
                    value: number,
                    op: Op::I32Const(1),
                    span: Span::dummy(),
                },
                Inst {
                    value: sum,
                    op: Op::I32Add(word, number),
                    span: Span::dummy(),
                },
            ],
            term: Terminator::Return {
                value: sum,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert!(matches!(
            body.validate(),
            Err(Invalid::OperandType {
                operand,
                expected: Ty::I32,
                found: Ty::EQREF,
            }) if operand == word,
        ));
    }

    #[test]
    fn a_value_of_a_body_that_nothing_defines_is_a_mistake() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let cast = value(&mut builder, Ty::I31);
        let orphan = value(&mut builder, Ty::I31);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: cast,
                op: Op::RefCast(RefTy::I31, param),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: cast,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(
            body.validate(),
            Err(Invalid::ValueNotDefined { value: orphan }),
        );
    }

    #[test]
    fn a_use_that_its_definition_does_not_dominate_is_a_mistake() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let cond = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let defined = value(&mut builder, Ty::I32);
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
                value: defined,
                op: Op::I32Const(1),
                span: Span::dummy(),
            }],
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let else_ = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: defined,
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

        let body = builder.finish(entry);

        assert_eq!(
            body.validate(),
            Err(Invalid::NotDominated {
                value: defined,
                block: else_,
            }),
        );
    }

    #[test]
    fn a_use_before_the_definition_in_one_block_is_a_mistake() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let defined = value(&mut builder, Ty::I32);
        let used = value(&mut builder, Ty::I32);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![
                Inst {
                    value: used,
                    op: Op::I32Eqz(defined),
                    span: Span::dummy(),
                },
                Inst {
                    value: defined,
                    op: Op::I32Const(1),
                    span: Span::dummy(),
                },
            ],
            term: Terminator::Return {
                value: used,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(
            body.validate(),
            Err(Invalid::DefinedAfterUse {
                value: defined,
                block: entry,
            }),
        );
    }

    #[test]
    fn a_cast_that_is_not_to_a_non_null_reference_is_a_mistake() {
        let mut builder = BodyBuilder::new(Ty::EQREF);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let cast = value(&mut builder, Ty::EQREF);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: cast,
                op: Op::RefCast(RefTy::Eq, param),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: cast,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert!(matches!(body.validate(), Err(Invalid::OperandType { .. }),));
    }
}
