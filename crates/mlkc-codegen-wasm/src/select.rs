//! Selection: what every MIR value is, written as the instructions of the target ([ADR-0022]).
//!
//! Selection is mechanical. The refinements ([ADR-0020]) say what every value is known to be;
//! a value of an immediate kind is closed by `ref.i31`, an instruction that reads a number
//! opens it with `i31.get_s`, and a value that is only a word is cast where something narrower
//! reads it. Selection decides nothing else: it does not inline, it does not keep a value
//! virtual, and it does not skip a box. What it writes is what the types say, and the passes
//! that follow make it small.
//!
//! A copy of a value --- a statement of the SSA form that only reads another value --- is not an
//! instruction of the target, so it is not selected: what the copy defines is what it read.
//! What a block no path from the entry reaches holds is not selected either: its code can never
//! run, and its values are not definitions of the body.
//!
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_lir_wasm::{
    Block, BlockId, BlockTarget as LirTarget, BodyBuilder, Inst, Op, RefTy, Terminator, Ty,
    ValueData, ValueId,
};
use mlkc_mir::{
    BlockId as MirBlockId, BlockTarget, Body as MirBody, Callee, Const, Operand, Place, PrimOp,
    Rvalue, Stmt, StmtKind, Terminator as MirTerminator, ValueId as MirValueId,
};
use mlkc_span::Span;

use crate::{
    emit::FunctionCtx,
    refine::{AbiType, Refinements},
};

/// The representation a value of this kind has in the target.
pub(crate) fn ty_of(abi: AbiType) -> Ty {
    match abi {
        AbiType::Immediate => Ty::I31,
        AbiType::Word => Ty::EQREF,
    }
}

/// Lowers one SSA body into the LIR of the back end, with no pass run over it.
///
/// # Panics
///
/// Panics when `mir` is not a well-formed SSA body, which is the contract every stage before
/// selection promises ([ADR-0019][adr-0019]): a body that breaks it is a compiler bug.
///
/// [adr-0019]: ../../docs/adr/0019-mir.md
pub(crate) fn run(mir: &MirBody, ctx: &FunctionCtx<'_>) -> mlkc_lir_wasm::Body {
    if let Err(invalid) = mir.validate_ssa() {
        panic!("the input of selection is an SSA body, and this one is not: {invalid}");
    }

    Selector::new(mir, ctx).select()
}

/// The one selection of one body.
struct Selector<'a> {
    mir: &'a MirBody,
    ctx: &'a FunctionCtx<'a>,
    /// What every value of the body is known to be.
    refinements: Refinements,
    /// What the result of the function crosses the ABI as.
    ret: AbiType,
    /// The LIR block of every MIR block, by the MIR block's place in its arena.
    blocks: Vec<Option<BlockId>>,
    /// The LIR value of every MIR value that was selected, by the MIR value's place in its
    /// arena.
    values: Vec<Option<ValueId>>,
    builder: BodyBuilder,
    /// The block being filled.
    current: BlockId,
    /// The instructions of the block being filled.
    insts: Vec<Inst>,
}

impl<'a> Selector<'a> {
    fn new(mir: &'a MirBody, ctx: &'a FunctionCtx<'a>) -> Self {
        let builtins = ctx.layout.builtins();
        let refinements = Refinements::of(mir, builtins, ctx.layout);
        let shape = ctx.signature.shape(builtins);

        // A parameter is the local the ABI already declares for it, so the type selection gives
        // it and the type its signature declares have to be the same one: a disagreement is a
        // gap of the check, and not something selection repairs with a cast.
        debug_assert_eq!(
            mir.params
                .iter()
                .map(|value| refinements.get(*value).abi())
                .collect::<Vec<_>>(),
            shape.params,
            "every parameter to be the shape its signature declares",
        );

        let mut builder = BodyBuilder::new(ty_of(shape.ret));
        let mut values: Vec<Option<ValueId>> = vec![None; mir.values.len()];

        for param in &mir.params {
            let ty = ty_of(refinements.get(*param).abi());
            let value = builder.param(ValueData {
                span: mir.values[*param].span,
                ty,
            });

            values[param.index()] = Some(value);
        }

        // A block no path from the entry reaches is emitted as nothing: the code in it can
        // never run, and a value it defines is not a definition of the body.
        let order = mlkc_mir::cfg::Cfg::of(mir).reverse_postorder().to_vec();
        let mut reachable = vec![false; mir.blocks.len()];

        for at in &order {
            reachable[*at] = true;
        }

        let mut blocks: Vec<Option<BlockId>> = vec![None; mir.blocks.len()];

        for (at, block) in mir.blocks.iter() {
            let params = if reachable[at.index()] {
                block
                    .params
                    .iter()
                    .map(|param| {
                        let ty = ty_of(refinements.get(*param).abi());
                        let value = builder.value(ValueData {
                            span: mir.values[*param].span,
                            ty,
                        });

                        values[param.index()] = Some(value);

                        value
                    })
                    .collect()
            } else {
                Vec::new()
            };

            blocks[at.index()] = Some(builder.block(Block {
                params,
                insts: Vec::new(),
                term: Terminator::Unreachable {
                    span: Span::dummy(),
                },
            }));
        }

        let entry = blocks[mir.entry.index()].expect("the entry to be a block of the body");

        Self {
            mir,
            ctx,
            refinements,
            ret: shape.ret,
            blocks,
            values,
            builder,
            current: entry,
            insts: Vec::new(),
        }
    }

    /// Selects every block a path from the entry reaches, in reverse postorder, and finishes
    /// the body.
    fn select(mut self) -> mlkc_lir_wasm::Body {
        let ids: Vec<MirBlockId> = self.mir.blocks.iter().map(|(id, _)| id).collect();
        let order = mlkc_mir::cfg::Cfg::of(self.mir)
            .reverse_postorder()
            .to_vec();

        for at in order {
            self.current = self.blocks[at].expect("every block to be allocated");
            self.insts = Vec::new();

            let block = &self.mir.blocks[ids[at]];

            for stmt in &block.stmts {
                self.stmt(stmt);
            }

            let term = self.terminator(&block.term);
            let insts = std::mem::take(&mut self.insts);
            let built = self.builder.block_mut(self.current);

            built.insts = insts;
            built.term = term;
        }

        let entry = self.blocks[self.mir.entry.index()].expect("the entry to be a block");

        self.builder.finish(entry)
    }

    /// Selects one statement: what it computes, as the value it defines.
    fn stmt(&mut self, stmt: &'a Stmt) {
        let StmtKind::Assign { place, rvalue } = &stmt.kind;
        let Place::Value(place) = place else {
            panic!("the input of selection is an SSA body, and this one writes a slot");
        };

        // A copy of a value is not an instruction of the target: what the copy defines is what
        // it read.
        let selected = match rvalue {
            Rvalue::Use(Operand::Value(value)) => self.mapped(*value),
            Rvalue::Use(operand) => self.operand_value(operand, stmt.span),
            Rvalue::Const(constant) => self.constant(constant, stmt.span),
            Rvalue::Prim { op, args } => self.prim(*op, args, stmt.span),
            Rvalue::Call { callee, args } => self.call(callee, args, stmt.span),
        };

        debug_assert_eq!(
            self.builder.value_data(selected).ty,
            ty_of(self.refinements.get(*place).abi()),
            "what a statement defines to be the kind the refinements gave it",
        );

        self.values[place.index()] = Some(selected);
    }

    /// The LIR value a MIR value was selected to.
    fn mapped(&self, value: MirValueId) -> ValueId {
        self.values[value.index()].unwrap_or_else(|| {
            panic!(
                "{} of the MIR is not selected by the time it is read",
                mlkc_mir::dump::value_label(value),
            )
        })
    }

    /// Selects an operand as the value it is.
    fn operand_value(&mut self, operand: &Operand, span: Span) -> ValueId {
        match operand {
            Operand::Value(value) => self.mapped(*value),
            Operand::Const(constant) => self.constant(constant, span),
            Operand::Local(_) => {
                panic!("the input of selection is an SSA body, and this one reads a slot")
            },
        }
    }

    /// Selects a constant as the word it is.
    fn constant(&mut self, constant: &Const, span: Span) -> ValueId {
        match constant {
            Const::Int(value) => self.boxed(*value, span),
            Const::Bool(value) => self.boxed(i32::from(*value), span),
            Const::Unit => self.boxed(0, span),
            Const::Str(value) => self.inst(Op::String(value.clone()), Ty::EQREF, span),
        }
    }

    /// Selects a number, boxed.
    fn boxed(&mut self, number: i32, span: Span) -> ValueId {
        let number = self.inst(Op::I32Const(number), Ty::I32, span);

        self.inst(Op::RefI31(number), Ty::I31, span)
    }

    /// Selects an operand as the number inside it.
    fn operand_i32(&mut self, operand: &Operand, span: Span) -> ValueId {
        match operand {
            Operand::Value(value) => {
                let value = self.mapped(*value);

                self.number(value, span)
            },
            Operand::Const(constant) => {
                match constant {
                    Const::Int(value) => self.inst(Op::I32Const(*value), Ty::I32, span),
                    Const::Bool(value) => self.inst(Op::I32Const(i32::from(*value)), Ty::I32, span),
                    Const::Unit => self.inst(Op::I32Const(0), Ty::I32, span),
                    Const::Str(_) => panic!("a string is read as a number"),
                }
            },
            Operand::Local(_) => {
                panic!("the input of selection is an SSA body, and this one reads a slot")
            },
        }
    }

    /// Selects an operand as the word it is, without opening it.
    fn operand_word(&mut self, operand: &Operand, span: Span) -> ValueId {
        match operand {
            Operand::Value(value) => self.mapped(*value),
            Operand::Const(constant) => self.constant(constant, span),
            Operand::Local(_) => {
                panic!("the input of selection is an SSA body, and this one reads a slot")
            },
        }
    }

    /// Selects an operand as the representation a shape crosses the ABI as.
    fn operand_into(&mut self, operand: &Operand, abi: AbiType, span: Span) -> ValueId {
        match abi {
            AbiType::Immediate => {
                let value = self.operand_value(operand, span);

                self.immediate(value, span)
            },
            AbiType::Word => self.operand_word(operand, span),
        }
    }

    /// The number inside a value, opening it where it is a word.
    fn number(&mut self, value: ValueId, span: Span) -> ValueId {
        match self.builder.value_data(value).ty {
            Ty::I32 => value,
            Ty::I31 => self.inst(Op::I31GetS(value), Ty::I32, span),
            Ty::Ref(_) => {
                let cast = self.inst(Op::RefCast(RefTy::I31, value), Ty::I31, span);

                self.inst(Op::I31GetS(cast), Ty::I32, span)
            },
        }
    }

    /// A value as an immediate, narrowing it where it is only a word.
    fn immediate(&mut self, value: ValueId, span: Span) -> ValueId {
        match self.builder.value_data(value).ty {
            Ty::I31 => value,
            Ty::Ref(_) => self.inst(Op::RefCast(RefTy::I31, value), Ty::I31, span),
            Ty::I32 => self.inst(Op::RefI31(value), Ty::I31, span),
        }
    }

    /// Selects a primitive, boxed into the word it computes.
    fn prim(&mut self, op: PrimOp, args: &'a [Operand], span: Span) -> ValueId {
        use PrimOp::*;

        let number = match op {
            IntAdd => self.binary(args, span, Op::I32Add),
            IntSub => self.binary(args, span, Op::I32Sub),
            IntMul => self.binary(args, span, Op::I32Mul),
            IntDiv => self.binary(args, span, Op::I32DivS),
            IntEq | BoolEq => self.binary(args, span, Op::I32Eq),
            IntNe | BoolNe => self.binary(args, span, Op::I32Ne),
            IntLt => self.binary(args, span, Op::I32LtS),
            IntLe => self.binary(args, span, Op::I32LeS),
            IntGt => self.binary(args, span, Op::I32GtS),
            IntGe => self.binary(args, span, Op::I32GeS),
            BoolAnd => self.binary(args, span, Op::I32And),
            BoolOr => self.binary(args, span, Op::I32Or),
            IntNeg => {
                let [operand] = args else {
                    panic!("a unary primitive to take one operand");
                };
                let zero = self.inst(Op::I32Const(0), Ty::I32, span);
                let operand = self.operand_i32(operand, span);

                self.inst(Op::I32Sub(zero, operand), Ty::I32, span)
            },
            BoolNot => {
                let [operand] = args else {
                    panic!("a unary primitive to take one operand");
                };
                let operand = self.operand_i32(operand, span);

                self.inst(Op::I32Eqz(operand), Ty::I32, span)
            },
            RefEq => {
                let [lhs, rhs] = args else {
                    panic!("a binary primitive to take two operands");
                };
                let lhs = self.operand_word(lhs, span);
                let rhs = self.operand_word(rhs, span);

                self.inst(Op::RefEq(lhs, rhs), Ty::I32, span)
            },
        };

        self.inst(Op::RefI31(number), Ty::I31, span)
    }

    /// Selects a binary instruction over two numbers.
    fn binary(
        &mut self,
        args: &'a [Operand],
        span: Span,
        op: fn(ValueId, ValueId) -> Op,
    ) -> ValueId {
        let [lhs, rhs] = args else {
            panic!("a binary primitive to take two operands");
        };
        let lhs = self.operand_i32(lhs, span);
        let rhs = self.operand_i32(rhs, span);

        self.inst(op(lhs, rhs), Ty::I32, span)
    }

    /// Selects a call, whose result is the shape its callee declares.
    fn call(&mut self, callee: &Callee, args: &'a [Operand], span: Span) -> ValueId {
        match callee {
            Callee::Entity(entity) => {
                let index = self.ctx.layout.function_index(entity).unwrap_or_else(|| {
                    panic!("a call to a function the module neither declares nor imports")
                });
                let signature = self
                    .ctx
                    .layout
                    .signature_of(entity)
                    .expect("a function with an index to have a signature");
                let shape = signature.shape(self.ctx.layout.builtins());
                let mut arguments = Vec::with_capacity(args.len());

                assert_eq!(
                    args.len(),
                    shape.params.len(),
                    "a call to pass one argument per parameter of its callee",
                );

                // An argument crosses as the shape of the callee says; an immediate crossing
                // into a word needs nothing, because an immediate is an `eq` reference.
                for (argument, abi) in args.iter().zip(&shape.params) {
                    arguments.push(self.operand_into(argument, *abi, span));
                }

                self.inst(
                    Op::Call {
                        function: index,
                        args: arguments,
                    },
                    ty_of(shape.ret),
                    span,
                )
            },
            Callee::Local(function) => {
                let mut arguments = Vec::with_capacity(args.len());

                for argument in args {
                    arguments.push(self.operand_value(argument, span));
                }

                self.inst(
                    Op::CallLocal {
                        function: *function,
                        args: arguments,
                    },
                    Ty::EQREF,
                    span,
                )
            },
            Callee::Indirect(callee) => {
                let callee = self.operand_word(callee, span);
                let mut arguments = Vec::with_capacity(args.len());

                for argument in args {
                    arguments.push(self.operand_value(argument, span));
                }

                self.inst(
                    Op::CallIndirect {
                        callee,
                        args: arguments,
                    },
                    Ty::EQREF,
                    span,
                )
            },
        }
    }

    /// Selects a terminator.
    fn terminator(&mut self, term: &'a MirTerminator) -> Terminator {
        match term {
            MirTerminator::Goto { target, span } => {
                Terminator::Goto {
                    target: self.edge(target, *span),
                    span: *span,
                }
            },
            MirTerminator::Branch {
                cond,
                then_,
                else_,
                span,
            } => {
                let cond = self.operand_i32(cond, *span);
                let then_ = self.edge(then_, *span);
                let else_ = self.edge(else_, *span);

                Terminator::Branch {
                    cond,
                    then_,
                    else_,
                    span: *span,
                }
            },
            MirTerminator::Switch {
                scrutinee,
                arms,
                otherwise,
                span,
            } => {
                let scrutinee = self.operand_i32(scrutinee, *span);
                let mut selected = Vec::with_capacity(arms.len());

                for (constant, target) in arms {
                    let value = match constant {
                        Const::Int(value) => *value,
                        Const::Bool(value) => i32::from(*value),
                        Const::Unit => 0,
                        Const::Str(_) => panic!("a string read by a switch"),
                    };
                    let target = self.edge(target, *span);

                    selected.push((value, target));
                }

                let otherwise = self.edge(otherwise, *span);

                Terminator::Switch {
                    scrutinee,
                    arms: selected,
                    otherwise,
                    span: *span,
                }
            },
            MirTerminator::Return { value, span } => {
                let value = self.operand_into(value, self.ret, *span);

                Terminator::Return { value, span: *span }
            },
            MirTerminator::Unreachable { span } => Terminator::Unreachable { span: *span },
        }
    }

    /// Selects an edge: its arguments as the parameters of its target want them.
    fn edge(&mut self, target: &'a BlockTarget, span: Span) -> LirTarget {
        let destination = &self.mir.blocks[target.block];

        debug_assert_eq!(
            destination.params.len(),
            target.args.len(),
            "an edge to pass one argument per parameter of its target",
        );

        let mut args = Vec::with_capacity(target.args.len());

        for (param, argument) in destination.params.iter().zip(&target.args) {
            let abi = self.refinements.get(*param).abi();

            args.push(self.operand_into(argument, abi, span));
        }

        LirTarget {
            block: self.blocks[target.block.index()].expect("every block to be allocated"),
            args,
        }
    }

    /// Appends an instruction, defining a value of `ty`.
    fn inst(&mut self, op: Op, ty: Ty, span: Span) -> ValueId {
        let value = self.builder.value(ValueData { span, ty });

        self.insts.push(Inst { value, op, span });

        value
    }
}
