//! Emitting one function: a MIR body to a WASM function body.
//!
//! The emitter is handed an SSA body and a [`FunctionCtx`], and owns its output: the encoded
//! body, where its instructions came from, and what every value refined to. It borrows nothing
//! from its inputs ([ADR-0009][adr-0009]), so the driver stores an artifact and drops the MIR
//! it came from.
//!
//! # Locals
//!
//! Before `localify` exists ([ADR-0020][adr-0020]), every value and every block parameter gets
//! a local of its own, typed by its refinement: an immediate is a `(ref i31)` and everything
//! else is an `eqref`. A parameter enters as an `eqref` --- the uniform word ABI --- and the
//! entry block casts it to its refinement once.
//!
//! # Control flow
//!
//! A body of one block that returns is emitted directly, as the straight line it is. Every
//! other body is emitted as a dispatch loop: a `pc` local, one `block` per MIR block, and a
//! `br_table` over the block ids. The dispatch loop is always correct for a well-formed body,
//! and it is what the emitter falls back to until reducible bodies are structured into
//! `if`/`loop` regions; the body that the current language lowers to --- a single block ---
//! pays nothing for it.
//!
//! An edge into a block passes one word per parameter. The emitter evaluates every argument
//! before storing any of them and stores them into the parameter locals in reverse, so that an
//! edge which permutes the parameters of its target copies all of them.
//!
//! [adr-0009]: ../../docs/adr/0009-pass-contract.md
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md

use std::fmt;

use mlkc_hir_def::Name;
use mlkc_mir::{
    BlockId, BlockTarget, Body, Callee, Const, Operand, Place, PrimOp, Rvalue, Stmt, StmtKind,
    Terminator, ValueId,
};
use mlkc_span::Span;
use wasm_encoder::{
    AbstractHeapType, BlockType, Function, HeapType, Instruction, RefType, ValType,
};

use crate::{
    module::{FnSignature, ModuleLayout},
    refine::{Refinement, Refinements},
};

/// What codegen reads of one body besides the body itself ([ADR-0009][adr-0009]).
///
/// [adr-0009]: ../../docs/adr/0009-pass-contract.md
#[derive(Debug)]
pub struct FunctionCtx<'a> {
    /// The name the function is called by, for the debug tables.
    pub name: &'a str,
    /// What the function takes and gives back.
    pub signature: &'a FnSignature,
    /// The name every parameter was declared under, in order.
    pub param_names: &'a [Option<Name>],
    /// How the functions of the module are numbered, and what they are.
    pub layout: &'a ModuleLayout,
}

/// One compiled function: its body bytes, and where its instructions came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncArtifact {
    /// The encoded body, without the size prefix.
    pub body: Vec<u8>,
    /// Source positions of instructions inside `body`, by byte offset.
    pub origins: Vec<Origin>,
    /// The refinement of every value, for the debugger and for the assembler.
    pub debug: Vec<ValueDebug>,
}

/// Where the instruction at a byte offset of a body was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    /// The byte offset inside [`FuncArtifact::body`] of the first byte of the instruction.
    pub offset: u32,
    /// The source position of the instruction.
    pub span: Span,
}

/// What a MIR value refined to, and where it lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDebug {
    /// The MIR value.
    pub value: ValueId,
    /// The refinement the emitter gave it.
    pub refinement: Refinement,
    /// The WASM local the value lives in.
    pub local: u32,
    /// The name the value was bound under in the source, if it has one.
    pub name: Option<Name>,
}

/// What codegen reports about a body it cannot emit.
///
/// A backend does not reject a well-formed SSA body for its shape ([ADR-0020][adr-0020]); what
/// it reports is a construct it does not lower *yet*, and the driver drops the artifact with
/// it. A `Unexpected` variant is a compiler bug: the body violates the contract the stages
/// before codegen promise.
///
/// [adr-0020]: ../../docs/adr/0020-wasm-backend.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenDiag {
    /// The emitter met a construct it does not lower yet.
    Unsupported {
        /// What the construct is, as a phrase a person reads: "a string constant".
        what: &'static str,
        /// Where it is written.
        span: Span,
    },
    /// The emitter met a body that breaks the invariant of the SSA form.
    Unexpected {
        /// What is wrong.
        what: String,
        /// Where it is written.
        span: Span,
    },
}

impl CodegenDiag {
    /// Where the construct is written.
    pub fn span(&self) -> Span {
        match self {
            Self::Unsupported { span, .. } | Self::Unexpected { span, .. } => *span,
        }
    }
}

impl fmt::Display for CodegenDiag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { what, .. } => {
                write!(f, "the WASM back end does not lower {what} yet")
            },
            Self::Unexpected { what, .. } => write!(f, "the WASM back end met {what}"),
        }
    }
}

/// Emits the WASM function body of one SSA body.
///
/// # Panics
///
/// Panics when `mir` is not a well-formed SSA body, which is the contract every stage before
/// codegen promises ([ADR-0019][adr-0019]): a body that breaks it is a compiler bug.
///
/// [adr-0019]: ../../docs/adr/0019-mir.md
pub fn emit_function(mir: &Body, ctx: &FunctionCtx<'_>) -> (FuncArtifact, Vec<CodegenDiag>) {
    if let Err(invalid) = mir.validate_ssa() {
        panic!("the input of the WASM back end is an SSA body, and this one is not: {invalid}");
    }

    assert_eq!(
        mir.params.len(),
        ctx.signature.params.len(),
        "a body has as many parameters as its signature says",
    );

    let mut emitter = Emitter::new(mir, ctx);

    emitter.run();

    emitter.finish()
}

/// The refinement of a value's WASM local.
fn local_type(refinement: Refinement) -> ValType {
    if refinement.is_immediate() {
        ValType::Ref(RefType::new_abstract(AbstractHeapType::I31, false, false))
    } else {
        ValType::Ref(RefType::EQREF)
    }
}

/// The one emitter of one function.
struct Emitter<'a> {
    mir: &'a Body,
    ctx: &'a FunctionCtx<'a>,
    refinements: Refinements,
    /// The WASM local of every value, by the value's place in its arena.
    value_locals: Vec<u32>,
    /// The local the dispatch form keeps the program counter in.
    pc_local: u32,
    /// The local the dispatch form compares a `switch` scrutinee in.
    scratch_local: u32,
    function: Function,
    origins: Vec<Origin>,
    diags: Vec<CodegenDiag>,
    /// Set once a construct could not be emitted: the rest of the body is skipped.
    failed: bool,
}

impl<'a> Emitter<'a> {
    fn new(mir: &'a Body, ctx: &'a FunctionCtx<'a>) -> Self {
        let refinements = Refinements::of(mir, ctx.layout.builtins(), ctx.layout);

        // The first locals are the ABI parameters, which the function type declares; the
        // values come after them, one local each, and the dispatch form keeps two more.
        let params = mir.params.len() as u32;
        let value_locals: Vec<u32> = (0..mir.values.len() as u32).map(|i| params + i).collect();
        let pc_local = params + mir.values.len() as u32;
        let scratch_local = pc_local + 1;

        let mut locals: Vec<ValType> = value_locals
            .iter()
            .map(|_| ValType::Ref(RefType::EQREF))
            .collect();

        for (value, data) in mir.values.iter() {
            let _ = data;
            locals[value.index()] = local_type(refinements.get(value));
        }

        // The two dispatch locals are declared always, so that the local indices of the values
        // do not depend on the shape of the body; `localify` will drop what is unused.
        locals.push(ValType::I32);
        locals.push(ValType::I32);

        Self {
            mir,
            ctx,
            refinements,
            value_locals,
            pc_local,
            scratch_local,
            function: Function::new_with_locals_types(locals),
            origins: Vec::new(),
            diags: Vec::new(),
            failed: false,
        }
    }

    /// Emits the whole body: the entry casts, then the blocks.
    fn run(&mut self) {
        let single = self.mir.blocks.len() == 1
            && matches!(
                &self.mir.blocks[self.mir.entry].term,
                Terminator::Return { .. } | Terminator::Unreachable { .. }
            );

        // A dispatch form is entered through the table, and the validator does not know which
        // `pc` reached it: every value local is initialized before the prologue writes the
        // parameters over theirs, or a read in a case the path did not reach is rejected. What
        // the initialization writes is never read: SSA defines every value before its use.
        if !single {
            self.initialize_values();
        }

        self.prologue();

        if single {
            self.direct(self.mir.entry);
        } else {
            self.dispatch();
        }
    }

    /// Initializes every value local, so that a dispatch form validates under Wasm GC.
    ///
    /// An immediate is a `(ref i31)`, which has no default value: a `local.get` of one the
    /// validator cannot prove to be written is an error. The parameters of the entry block are
    /// left out: the prologue writes them.
    fn initialize_values(&mut self) {
        for (value, _) in self.mir.values.iter() {
            if self.mir.params.contains(&value) {
                continue;
            }

            let local = self.value_locals[value.index()];

            if self.refinements.get(value).is_immediate() {
                self.insn(Span::dummy(), Instruction::I32Const(0));
                self.insn(Span::dummy(), Instruction::RefI31);
            } else {
                self.insn(
                    Span::dummy(),
                    Instruction::RefNull(HeapType::Abstract {
                        shared: false,
                        ty: AbstractHeapType::Eq,
                    }),
                );
            }

            self.insn(Span::dummy(), Instruction::LocalSet(local));
        }
    }

    /// Casts every parameter into the local of the value it is.
    fn prologue(&mut self) {
        let mir = self.mir;

        for (parameter, value) in mir.params.iter().enumerate() {
            let span = mir.values[*value].span;

            self.insn(span, Instruction::LocalGet(parameter as u32));

            // The ABI is a word per parameter; what a parameter is known to be is made
            // precise once, here, rather than at every use.
            if self.refinements.get(*value).is_immediate() {
                self.insn(span, Instruction::RefCastNonNull(HeapType::I31));
            }

            self.insn(
                span,
                Instruction::LocalSet(self.value_locals[value.index()]),
            );
        }
    }

    /// Emits a body of one block as the straight line it is.
    fn direct(&mut self, id: BlockId) {
        let mir = self.mir;
        let block = &mir.blocks[id];

        self.stmts(&block.stmts);
        self.terminator(&block.term, None);
    }

    /// Emits every block of the body as a case of a dispatch loop.
    fn dispatch(&mut self) {
        let mir = self.mir;
        let count = mir.blocks.len() as u32;

        // `done` wraps the loop so that an out-of-range `pc` has somewhere to go; every
        // terminator branches to the loop or returns, so the fall-through is unreachable.
        self.insn(Span::dummy(), Instruction::Block(BlockType::Empty));
        self.insn(Span::dummy(), Instruction::Loop(BlockType::Empty));

        // One case block per MIR block, innermost first; branching to the `i`th label lands
        // right after the `i`th `end`, which is where the code of the block is emitted.
        for _ in 0..count {
            self.insn(Span::dummy(), Instruction::Block(BlockType::Empty));
        }

        self.insn(Span::dummy(), Instruction::LocalGet(self.pc_local));
        self.insn(
            Span::dummy(),
            Instruction::BrTable((0..count).collect::<Vec<_>>().into(), count + 1),
        );

        // The first `end` closes the innermost case; after it comes the code of block zero.
        self.insn(Span::dummy(), Instruction::End);

        let ids: Vec<BlockId> = mir.blocks.iter().map(|(id, _)| id).collect();

        for (index, id) in ids.iter().enumerate() {
            let index = index as u32;
            let block = &mir.blocks[*id];

            // The loop is as many labels above this point as there are case blocks left.
            let dispatch_depth = count - 1 - index;

            self.stmts(&block.stmts);
            self.terminator(&block.term, Some(dispatch_depth));

            if index + 1 < count {
                self.insn(Span::dummy(), Instruction::End);
            }
        }

        self.insn(Span::dummy(), Instruction::End);
        self.insn(Span::dummy(), Instruction::End);

        // Nothing falls out of the dispatch: `pc` is always a block, and every block ends in a
        // branch or a return. The instruction is what tells the validator so.
        self.insn(Span::dummy(), Instruction::Unreachable);
    }

    /// Emits the statements of a block.
    fn stmts(&mut self, stmts: &'a [Stmt]) {
        for stmt in stmts {
            if self.failed {
                return;
            }

            self.stmt(stmt);
        }
    }

    /// Emits one statement.
    fn stmt(&mut self, stmt: &'a Stmt) {
        let StmtKind::Assign { place, rvalue } = &stmt.kind;

        let Place::Value(place) = place else {
            self.unsupported("an assignment to a slot", stmt.span);

            return;
        };

        match rvalue {
            Rvalue::Use(operand) => {
                let refinement = self.refinements.get(*place);

                self.operand_into(operand, refinement, stmt.span);
            },
            Rvalue::Const(constant) => {
                self.constant(constant, stmt.span);
            },
            Rvalue::Prim { op, args } => {
                if !self.prim(*op, args, stmt.span) {
                    return;
                }
            },
            Rvalue::Call { callee, args } => {
                if !self.call(callee, args, *place, stmt.span) {
                    return;
                }
            },
        }

        self.insn(
            stmt.span,
            Instruction::LocalSet(self.value_locals[place.index()]),
        );
    }

    /// Emits a primitive, and the store of its result; `false` when it cannot be emitted.
    fn prim(&mut self, op: PrimOp, args: &'a [Operand], span: Span) -> bool {
        use PrimOp::*;

        match op {
            IntAdd | IntSub | IntMul | IntDiv | IntEq | IntNe | IntLt | IntLe | IntGt | IntGe
            | BoolAnd | BoolOr | BoolEq | BoolNe => {
                let [lhs, rhs] = args else {
                    self.unexpected("a binary primitive with not two operands", span);

                    return false;
                };

                self.operand_i32(lhs, span);
                self.operand_i32(rhs, span);

                self.insn(span, match op {
                    IntAdd => Instruction::I32Add,
                    IntSub => Instruction::I32Sub,
                    IntMul => Instruction::I32Mul,
                    IntDiv => Instruction::I32DivS,
                    IntEq | BoolEq => Instruction::I32Eq,
                    IntNe | BoolNe => Instruction::I32Ne,
                    IntLt => Instruction::I32LtS,
                    IntLe => Instruction::I32LeS,
                    IntGt => Instruction::I32GtS,
                    IntGe => Instruction::I32GeS,
                    BoolAnd => Instruction::I32And,
                    BoolOr => Instruction::I32Or,
                    _ => unreachable!("the arm above fixes the operator"),
                });

                self.insn(span, Instruction::RefI31);
            },
            IntNeg | BoolNot => {
                let [operand] = args else {
                    self.unexpected("a unary primitive with not one operand", span);

                    return false;
                };

                if op == IntNeg {
                    self.insn(span, Instruction::I32Const(0));
                }

                self.operand_i32(operand, span);

                self.insn(
                    span,
                    if op == IntNeg {
                        Instruction::I32Sub
                    } else {
                        Instruction::I32Eqz
                    },
                );

                self.insn(span, Instruction::RefI31);
            },
            RefEq => {
                let [lhs, rhs] = args else {
                    self.unexpected("a binary primitive with not two operands", span);

                    return false;
                };

                self.operand_word(lhs, span);
                self.operand_word(rhs, span);
                self.insn(span, Instruction::RefEq);
                self.insn(span, Instruction::RefI31);
            },
        }

        true
    }

    /// Emits a call, and the store of its result; `false` when it cannot be emitted.
    fn call(&mut self, callee: &Callee, args: &'a [Operand], place: ValueId, span: Span) -> bool {
        let index = match callee {
            Callee::Entity(entity) => {
                match self.ctx.layout.function_index(entity) {
                    Some(index) => index,
                    None => {
                        self.unsupported(
                            "a call to a function the module neither declares nor imports",
                            span,
                        );

                        return false;
                    },
                }
            },
            Callee::Local(_) => {
                self.unsupported("a call to a function declared inside a body", span);

                return false;
            },
            Callee::Indirect(_) => {
                self.unsupported("an indirect call", span);

                return false;
            },
        };

        for argument in args {
            self.operand_word(argument, span);
        }

        self.insn(span, Instruction::Call(index));

        // A call gives back a word; where the result is known to be an immediate, it is made
        // one again here, once, rather than at every use.
        if self.refinements.get(place).is_immediate() {
            self.insn(span, Instruction::RefCastNonNull(HeapType::I31));
        }

        true
    }

    /// Emits a terminator. `dispatch` is the depth of the dispatch loop where it is written,
    /// and `None` in a body of one block.
    fn terminator(&mut self, term: &'a Terminator, dispatch: Option<u32>) {
        match term {
            Terminator::Return { value, span } => {
                self.operand_word(value, *span);
                self.insn(*span, Instruction::Return);
            },
            Terminator::Unreachable { span } => self.insn(*span, Instruction::Unreachable),
            Terminator::Goto { target, span } => {
                let Some(depth) = dispatch else {
                    self.unexpected("a body of one block that goes to a block", *span);

                    return;
                };

                self.edge(target, depth, *span);
            },
            Terminator::Branch {
                cond,
                then_,
                else_,
                span,
            } => {
                let Some(depth) = dispatch else {
                    self.unexpected("a body of one block that branches", *span);

                    return;
                };

                self.operand_i32(cond, *span);
                self.insn(*span, Instruction::If(BlockType::Empty));

                self.edge(then_, depth + 1, *span);
                self.insn(*span, Instruction::Else);
                self.edge(else_, depth + 1, *span);

                self.insn(*span, Instruction::End);
            },
            Terminator::Switch {
                scrutinee,
                arms,
                otherwise,
                span,
            } => {
                let Some(depth) = dispatch else {
                    self.unexpected("a body of one block that switches", *span);

                    return;
                };

                self.operand_i32(scrutinee, *span);
                self.insn(*span, Instruction::LocalSet(self.scratch_local));

                for (constant, target) in arms {
                    self.insn(*span, Instruction::LocalGet(self.scratch_local));

                    if !self.constant_i32(constant, *span) {
                        return;
                    }

                    self.insn(*span, Instruction::I32Eq);
                    self.insn(*span, Instruction::If(BlockType::Empty));

                    self.edge(target, depth + 1, *span);

                    self.insn(*span, Instruction::End);
                }

                self.edge(otherwise, depth, *span);
            },
        }
    }

    /// Emits an edge: every argument, then the parameter locals in reverse, then the branch.
    fn edge(&mut self, target: &'a BlockTarget, depth: u32, span: Span) {
        let mir = self.mir;
        let destination = &mir.blocks[target.block];
        let refinements: Vec<Refinement> = destination
            .params
            .iter()
            .map(|param| self.refinements.get(*param))
            .collect();

        debug_assert_eq!(
            destination.params.len(),
            target.args.len(),
            "an edge passes one argument per parameter of its target",
        );

        for (argument, refinement) in target.args.iter().zip(&refinements) {
            self.operand_into(argument, *refinement, span);
        }

        // The stores are in reverse, so that an edge which permutes the parameters of its
        // target reads every argument before it writes any of them.
        for param in destination.params.iter().rev() {
            self.insn(
                span,
                Instruction::LocalSet(self.value_locals[param.index()]),
            );
        }

        self.insn(span, Instruction::I32Const(target.block.index() as i32));
        self.insn(span, Instruction::LocalSet(self.pc_local));
        self.insn(span, Instruction::Br(depth));
    }

    /// Emits an operand as the word it is, boxed where it is an immediate constant.
    fn operand_word(&mut self, operand: &'a Operand, span: Span) {
        match operand {
            Operand::Value(value) => {
                self.insn(
                    span,
                    Instruction::LocalGet(self.value_locals[value.index()]),
                );
            },
            Operand::Const(constant) => {
                self.constant(constant, span);
            },
            Operand::Local(_) => self.unsupported("a read of a slot", span),
        }
    }

    /// Emits an operand as an `i31ref`, casting a word where the refinement is not precise.
    fn operand_into(&mut self, operand: &'a Operand, refinement: Refinement, span: Span) {
        match operand {
            Operand::Value(value) => {
                self.insn(
                    span,
                    Instruction::LocalGet(self.value_locals[value.index()]),
                );

                if refinement.is_immediate() && !self.refinements.get(*value).is_immediate() {
                    self.insn(span, Instruction::RefCastNonNull(HeapType::I31));
                }
            },
            Operand::Const(constant) => {
                self.constant(constant, span);
            },
            Operand::Local(_) => self.unsupported("a read of a slot", span),
        }
    }

    /// Emits an operand as the unboxed `i32` an arithmetic instruction reads.
    fn operand_i32(&mut self, operand: &'a Operand, span: Span) {
        match operand {
            Operand::Value(value) => {
                self.insn(
                    span,
                    Instruction::LocalGet(self.value_locals[value.index()]),
                );

                if !self.refinements.get(*value).is_immediate() {
                    self.insn(span, Instruction::RefCastNonNull(HeapType::I31));
                }

                self.insn(span, Instruction::I31GetS);
            },
            Operand::Const(constant) => {
                self.constant_i32(constant, span);
            },
            Operand::Local(_) => self.unsupported("a read of a slot", span),
        }
    }

    /// Emits a constant as the word it is.
    fn constant(&mut self, constant: &'a Const, span: Span) -> bool {
        match constant {
            Const::Int(value) => {
                self.insn(span, Instruction::I32Const(*value));
                self.insn(span, Instruction::RefI31);
            },
            Const::Bool(value) => {
                self.insn(span, Instruction::I32Const(i32::from(*value)));
                self.insn(span, Instruction::RefI31);
            },
            Const::Unit => {
                self.insn(span, Instruction::I32Const(0));
                self.insn(span, Instruction::RefI31);
            },
            Const::Str(_) => {
                self.unsupported("a string constant", span);

                return false;
            },
        }

        true
    }

    /// Emits a constant as the unboxed `i32` a comparison reads.
    fn constant_i32(&mut self, constant: &'a Const, span: Span) -> bool {
        match constant {
            Const::Int(value) => self.insn(span, Instruction::I32Const(*value)),
            Const::Bool(value) => self.insn(span, Instruction::I32Const(i32::from(*value))),
            Const::Unit => self.insn(span, Instruction::I32Const(0)),
            Const::Str(_) => {
                self.unsupported("a string constant", span);

                return false;
            },
        }

        true
    }

    /// Records an instruction and where it came from.
    fn insn(&mut self, span: Span, instruction: Instruction<'_>) {
        if !span.is_dummy() {
            self.origins.push(Origin {
                offset: self.function.byte_len() as u32,
                span,
            });
        }

        self.function.instruction(&instruction);
    }

    /// Reports a construct the emitter does not lower yet, and stops emitting the body.
    fn unsupported(&mut self, what: &'static str, span: Span) {
        self.diags.push(CodegenDiag::Unsupported { what, span });
        self.failed = true;
    }

    /// Reports a body that breaks the invariant of the SSA form, and stops emitting it.
    fn unexpected(&mut self, what: &str, span: Span) {
        self.diags.push(CodegenDiag::Unexpected {
            what: what.to_owned(),
            span,
        });
        self.failed = true;
    }

    /// Finishes the artifact and the reports of what could not be emitted.
    fn finish(mut self) -> (FuncArtifact, Vec<CodegenDiag>) {
        // A function body is a sequence of instructions closed by `end`, which is the frame of
        // the function itself; every frame the emitter opened was closed before this.
        self.function.instruction(&Instruction::End);

        let debug = self.debug();

        let artifact = FuncArtifact {
            body: self.function.into_raw_body(),
            origins: self.origins,
            debug,
        };

        (artifact, self.diags)
    }

    /// What every value refined to, and where it lives.
    fn debug(&self) -> Vec<ValueDebug> {
        let mir = self.mir;
        let mut names: Vec<Option<Name>> = vec![None; mir.values.len()];

        for (index, value) in mir.params.iter().enumerate() {
            names[value.index()] = self.ctx.param_names.get(index).cloned().unwrap_or_default();
        }

        mir.values
            .iter()
            .map(|(value, _)| {
                ValueDebug {
                    value,
                    refinement: self.refinements.get(value),
                    local: self.value_locals[value.index()],
                    name: names[value.index()].clone(),
                }
            })
            .collect()
    }
}
