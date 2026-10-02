//! The LIR of one body: the target's instructions in SSA form ([ADR-0022][adr-0022]).
//!
//! A body is a graph of blocks, a block is a list of instructions and one terminator, and every
//! value is defined once: by a parameter of the body, by a parameter of a block, or by the one
//! instruction a statement is. An instruction reads values and computes one --- the stack is
//! not here, the stack is what encoding does --- and the values it reads are the operands of a
//! target instruction, named the way the target names it.
//!
//! A value is virtual: it has a type and no storage. The [`Locals`] of a body are what the
//! allocation pass decides --- which values live in a WASM local, and which are emitted where
//! they are read --- and a body straight out of selection has none; the parameters of the
//! function are the locals the ABI declares, and the first ones of the table are theirs.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_hir_def::LocalFunctionId;
use mlkc_intern::Interned;
use mlkc_la_arena::{Arena, Idx};
use mlkc_span::Span;

use crate::ty::{RefTy, Ty};

/// The id of a block inside one body.
pub type BlockId = Idx<Block>;

/// The id of a value inside one body.
pub type ValueId = Idx<ValueData>;

/// The index of a function in the module the body belongs to.
pub type FuncIndex = u32;

/// The LIR of one body.
///
/// The body is anonymous: what it is the body of, and the signature it crosses the ABI as, are
/// the context of the function it belongs to, and not a property of its instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    /// The values the body is entered with: one per parameter, in the order they are declared.
    ///
    /// A parameter is a value of the ABI's shape, and the local it lives in is the one the ABI
    /// declares for it; nothing in the body computes it.
    pub params: Vec<ValueId>,
    /// What the body gives back.
    pub ret: Ty,
    /// The block the body enters.
    pub entry: BlockId,
    /// The blocks.
    pub blocks: Arena<Block>,
    /// The values: the parameters of the body and of its blocks, and the ones the instructions
    /// define.
    pub values: Arena<ValueData>,
    /// Where the values live, once the allocation pass has decided.
    pub locals: Locals,
}

/// One block: its parameters, its instructions, and the terminator it ends in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The block parameters: where a value coming from several predecessors is born.
    pub params: Vec<ValueId>,
    /// The instructions, in the order they run.
    pub insts: Vec<Inst>,
    /// The terminator: where control goes.
    pub term: Terminator,
}

/// What a value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueData {
    /// Where the value is written.
    pub span: Span,
    /// The type of the machine value.
    pub ty: Ty,
}

/// One instruction: the value it defines, what it computes, and where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inst {
    /// The value the instruction defines.
    pub value: ValueId,
    /// What the instruction computes.
    pub op: Op,
    /// Where the instruction is written.
    pub span: Span,
}

/// What an instruction computes: one instruction of the target, over the values it reads.
///
/// An operator is named the way WASM names it, and an instruction the backend cannot write yet
/// carries what a report names it by: a string constant, a call to a function declared inside a
/// body, an indirect call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// `i32.const`: a number.
    I32Const(i32),
    /// `i32.eqz`.
    I32Eqz(ValueId),
    /// `i32.eq`.
    I32Eq(ValueId, ValueId),
    /// `i32.ne`.
    I32Ne(ValueId, ValueId),
    /// `i32.lt_s`.
    I32LtS(ValueId, ValueId),
    /// `i32.le_s`.
    I32LeS(ValueId, ValueId),
    /// `i32.gt_s`.
    I32GtS(ValueId, ValueId),
    /// `i32.ge_s`.
    I32GeS(ValueId, ValueId),
    /// `i32.add`.
    I32Add(ValueId, ValueId),
    /// `i32.sub`.
    I32Sub(ValueId, ValueId),
    /// `i32.mul`.
    I32Mul(ValueId, ValueId),
    /// `i32.div_s`; a zero divisor traps.
    I32DivS(ValueId, ValueId),
    /// `i32.and`.
    I32And(ValueId, ValueId),
    /// `i32.or`.
    I32Or(ValueId, ValueId),
    /// `ref.i31`: the box of a number.
    RefI31(ValueId),
    /// `i31.get_s`: the number inside a box, which never traps: the box is not null.
    I31GetS(ValueId),
    /// `ref.cast`: the value, as the reference type it is cast to; a value of another type traps.
    RefCast(RefTy, ValueId),
    /// `ref.eq`.
    RefEq(ValueId, ValueId),
    /// A call of a function of the module, by its index.
    Call {
        /// The index of the function.
        function: FuncIndex,
        /// The arguments, in the order they are passed.
        args: Vec<ValueId>,
    },
    /// A call of a function declared inside the enclosing body.
    CallLocal {
        /// The function declared inside the body.
        function: LocalFunctionId,
        /// The arguments, in the order they are passed.
        args: Vec<ValueId>,
    },
    /// A call of a function held in a value.
    CallIndirect {
        /// The value the function is in.
        callee: ValueId,
        /// The arguments, in the order they are passed.
        args: Vec<ValueId>,
    },
    /// A string constant.
    String(Interned<str>),
}

impl Op {
    /// The operator as a dump reads it.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::I32Const(_) => "i32.const",
            Self::I32Eqz(_) => "i32.eqz",
            Self::I32Eq(..) => "i32.eq",
            Self::I32Ne(..) => "i32.ne",
            Self::I32LtS(..) => "i32.lt_s",
            Self::I32LeS(..) => "i32.le_s",
            Self::I32GtS(..) => "i32.gt_s",
            Self::I32GeS(..) => "i32.ge_s",
            Self::I32Add(..) => "i32.add",
            Self::I32Sub(..) => "i32.sub",
            Self::I32Mul(..) => "i32.mul",
            Self::I32DivS(..) => "i32.div_s",
            Self::I32And(..) => "i32.and",
            Self::I32Or(..) => "i32.or",
            Self::RefI31(_) => "ref.i31",
            Self::I31GetS(_) => "i31.get_s",
            Self::RefCast(..) => "ref.cast",
            Self::RefEq(..) => "ref.eq",
            Self::Call { .. } => "call",
            Self::CallLocal { .. } => "call-local",
            Self::CallIndirect { .. } => "call-indirect",
            Self::String(_) => "str",
        }
    }

    /// The operands of the instruction, in the order it reads them.
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::I32Const(_) | Self::String(_) => Vec::new(),
            Self::I32Eqz(operand)
            | Self::RefI31(operand)
            | Self::I31GetS(operand)
            | Self::RefCast(_, operand) => vec![*operand],
            Self::I32Eq(lhs, rhs)
            | Self::I32Ne(lhs, rhs)
            | Self::I32LtS(lhs, rhs)
            | Self::I32LeS(lhs, rhs)
            | Self::I32GtS(lhs, rhs)
            | Self::I32GeS(lhs, rhs)
            | Self::I32Add(lhs, rhs)
            | Self::I32Sub(lhs, rhs)
            | Self::I32Mul(lhs, rhs)
            | Self::I32DivS(lhs, rhs)
            | Self::I32And(lhs, rhs)
            | Self::I32Or(lhs, rhs)
            | Self::RefEq(lhs, rhs) => vec![*lhs, *rhs],
            Self::Call { args, .. } | Self::CallLocal { args, .. } => args.clone(),
            Self::CallIndirect { callee, args } => {
                let mut operands = Vec::with_capacity(args.len() + 1);

                operands.push(*callee);
                operands.extend(args);

                operands
            },
        }
    }

    /// Rewrites every operand with `map`.
    pub fn map_operands(&mut self, mut map: impl FnMut(ValueId) -> ValueId) {
        match self {
            Self::I32Const(_) | Self::String(_) => {},
            Self::I32Eqz(operand)
            | Self::RefI31(operand)
            | Self::I31GetS(operand)
            | Self::RefCast(_, operand) => *operand = map(*operand),
            Self::I32Eq(lhs, rhs)
            | Self::I32Ne(lhs, rhs)
            | Self::I32LtS(lhs, rhs)
            | Self::I32LeS(lhs, rhs)
            | Self::I32GtS(lhs, rhs)
            | Self::I32GeS(lhs, rhs)
            | Self::I32Add(lhs, rhs)
            | Self::I32Sub(lhs, rhs)
            | Self::I32Mul(lhs, rhs)
            | Self::I32DivS(lhs, rhs)
            | Self::I32And(lhs, rhs)
            | Self::I32Or(lhs, rhs)
            | Self::RefEq(lhs, rhs) => {
                *lhs = map(*lhs);
                *rhs = map(*rhs);
            },
            Self::Call { args, .. } | Self::CallLocal { args, .. } => {
                for arg in args {
                    *arg = map(*arg);
                }
            },
            Self::CallIndirect { callee, args } => {
                *callee = map(*callee);

                for arg in args {
                    *arg = map(*arg);
                }
            },
        }
    }
}

/// Where a block ends, and where control goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    /// Control goes to one block.
    Goto {
        /// Where it goes.
        target: BlockTarget,
        /// Where the terminator is written.
        span: Span,
    },
    /// Control goes to one of two blocks, by a condition.
    Branch {
        /// The condition: an `i32` the target reads as true when it is not zero.
        cond: ValueId,
        /// Where control goes when the condition holds.
        then_: BlockTarget,
        /// Where control goes when it does not.
        else_: BlockTarget,
        /// Where the terminator is written.
        span: Span,
    },
    /// Control goes to one of several blocks, by what a number is.
    Switch {
        /// The number that decides.
        scrutinee: ValueId,
        /// The values and where control goes for each of them.
        arms: Vec<(i32, BlockTarget)>,
        /// Where control goes when no arm matches.
        otherwise: BlockTarget,
        /// Where the terminator is written.
        span: Span,
    },
    /// The body gives back a value.
    Return {
        /// The value.
        value: ValueId,
        /// Where the terminator is written.
        span: Span,
    },
    /// Control that is not meant to be reached.
    Unreachable {
        /// Where the terminator is written.
        span: Span,
    },
}

/// A block an edge goes to, and the values the edge passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTarget {
    /// The block.
    pub block: BlockId,
    /// The arguments: one per parameter of the block, in the order of the parameters.
    pub args: Vec<ValueId>,
}

impl Terminator {
    /// The blocks the terminator goes to, in the order it lists them.
    pub fn targets(&self) -> Vec<&BlockTarget> {
        match self {
            Self::Goto { target, .. } => vec![target],
            Self::Branch { then_, else_, .. } => vec![then_, else_],
            Self::Switch {
                arms, otherwise, ..
            } => {
                arms.iter()
                    .map(|(_, target)| target)
                    .chain([otherwise])
                    .collect()
            },
            Self::Return { .. } | Self::Unreachable { .. } => Vec::new(),
        }
    }

    /// The blocks the terminator goes to, to rewrite them.
    pub fn targets_mut(&mut self) -> Vec<&mut BlockTarget> {
        match self {
            Self::Goto { target, .. } => vec![target],
            Self::Branch { then_, else_, .. } => vec![then_, else_],
            Self::Switch {
                arms, otherwise, ..
            } => {
                arms.iter_mut()
                    .map(|(_, target)| target)
                    .chain([otherwise])
                    .collect()
            },
            Self::Return { .. } | Self::Unreachable { .. } => Vec::new(),
        }
    }

    /// The values the terminator reads, the arguments of its targets among them.
    pub fn operands(&self) -> Vec<ValueId> {
        let mut operands = Vec::new();

        match self {
            Self::Branch { cond, .. } => operands.push(*cond),
            Self::Switch { scrutinee, .. } => operands.push(*scrutinee),
            Self::Return { value, .. } => operands.push(*value),
            Self::Goto { .. } | Self::Unreachable { .. } => {},
        }

        for target in self.targets() {
            operands.extend(&target.args);
        }

        operands
    }

    /// Rewrites every value the terminator reads with `map`.
    pub fn map_operands(&mut self, mut map: impl FnMut(ValueId) -> ValueId) {
        match self {
            Self::Branch { cond, .. } => *cond = map(*cond),
            Self::Switch { scrutinee, .. } => *scrutinee = map(*scrutinee),
            Self::Return { value, .. } => *value = map(*value),
            Self::Goto { .. } | Self::Unreachable { .. } => {},
        }

        for target in self.targets_mut() {
            for arg in &mut target.args {
                *arg = map(*arg);
            }
        }
    }
}

/// Where the values of a body live, as the allocation pass decided ([ADR-0022][adr-0022]).
///
/// The table is empty until the pass fills it: selection emits every definition where it stands
/// and leaves the question of storage open. A value the pass gives a local to is read and
/// written by `local.get` and `local.set`; a value it does not is emitted where it is read,
/// which is what makes a constant or an operator one instruction on the stack instead of two.
///
/// The indices are the ones of the WASM local space: the parameters of the ABI are the locals
/// `0..parameters`, and the local at position `i` of [`Locals::locals`] is the WASM local
/// numbered `parameters + i`.
///
/// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Locals {
    /// The locals the body declares after the parameters of the ABI, in the order they were
    /// allocated.
    pub locals: Vec<Ty>,
    /// The local every value lives in, by the value's place in its arena; `None` for a value
    /// that is emitted where it is read.
    pub values: Vec<Option<u32>>,
    /// The local the dispatch form keeps its program counter in.
    pub pc: Option<u32>,
    /// The local the dispatch form compares the scrutinee of a `switch` in.
    pub scratch: Option<u32>,
}

impl Locals {
    /// Whether the pass has decided nothing: a body straight out of selection.
    pub fn is_unallocated(&self) -> bool {
        self.locals.is_empty()
            && self.values.is_empty()
            && self.pc.is_none()
            && self.scratch.is_none()
    }
}

impl Body {
    /// The instruction that defines every value, by the value's place in its arena; `None` for
    /// a parameter of the body or of a block.
    pub fn definitions(&self) -> Vec<Option<(BlockId, usize)>> {
        let mut definitions = vec![None; self.values.len()];

        for (id, block) in self.blocks.iter() {
            for (at, inst) in block.insts.iter().enumerate() {
                definitions[inst.value.index()] = Some((id, at));
            }
        }

        definitions
    }
}

/// Builds one body.
///
/// The builder owns the arenas, so that a stage allocates blocks and values in the order it
/// meets them, and refers to a block it has not filled yet by its id.
#[derive(Debug)]
pub struct BodyBuilder {
    ret: Ty,
    params: Vec<ValueId>,
    blocks: Arena<Block>,
    values: Arena<ValueData>,
}

impl BodyBuilder {
    /// A builder of a body that gives back `ret`.
    pub fn new(ret: Ty) -> Self {
        Self {
            ret,
            params: Vec::new(),
            blocks: Arena::new(),
            values: Arena::new(),
        }
    }

    /// Allocates a parameter of the body, in the order the parameters are declared.
    pub fn param(&mut self, data: ValueData) -> ValueId {
        let value = self.values.alloc(data);

        self.params.push(value);

        value
    }

    /// Allocates a value.
    pub fn value(&mut self, data: ValueData) -> ValueId {
        self.values.alloc(data)
    }

    /// What the value with this id is, for a stage that reads back what it allocated.
    pub fn value_data(&self, id: ValueId) -> &ValueData {
        &self.values[id]
    }

    /// Allocates a block.
    pub fn block(&mut self, block: Block) -> BlockId {
        self.blocks.alloc(block)
    }

    /// The block with this id, to fill it after it was referred to.
    pub fn block_mut(&mut self, id: BlockId) -> &mut Block {
        &mut self.blocks[id]
    }

    /// Finishes the body, entered at `entry`.
    pub fn finish(self, entry: BlockId) -> Body {
        Body {
            params: self.params,
            ret: self.ret,
            entry,
            blocks: self.blocks,
            values: self.values,
            locals: Locals::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_span::Span;

    use super::*;

    #[test]
    fn a_body_reads_back_what_the_builder_allocated() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let boxed = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let block = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: boxed,
                op: Op::RefCast(RefTy::I31, param),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: boxed,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(block);

        assert_eq!(body.params, [param]);
        assert_eq!(body.ret, Ty::I31);
        assert_eq!(body.entry, block);
        assert_eq!(body.values[boxed].ty, Ty::I31);
        assert_eq!(body.blocks[block].term, Terminator::Return {
            value: boxed,
            span: Span::dummy(),
        });
        assert!(body.locals.is_unallocated());
    }

    #[test]
    fn a_definition_is_the_instruction_that_writes_a_value() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let defined = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let block = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: defined,
                op: Op::I32Eqz(param),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: defined,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(block);
        let definitions = body.definitions();

        assert_eq!(definitions[param.index()], None);
        assert_eq!(definitions[defined.index()], Some((block, 0)));
    }
}
