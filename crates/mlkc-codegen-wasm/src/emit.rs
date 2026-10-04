//! Emitting one function: the LIR of a body to a WASM function body ([ADR-0022][adr-0022]).
//!
//! The emitter is handed a body whose allocation pass has run ([`crate::allocate`]) and a
//! [`FunctionCtx`], and owns its output: the encoded body, where its instructions came from,
//! and what every value is. It borrows nothing from its inputs ([ADR-0009][adr-0009]), so the
//! driver stores an artifact and drops the LIR it came from.
//!
//! It makes no decisions. What an instruction is, is what the LIR says; a value that lives in a
//! local is a `local.get`, and a value the allocation pass left out is the instruction that
//! defines it, written where the value is read. What is left is the shape of the module: the
//! dispatch table, the initialization a dispatched body needs, and the spans of the
//! instructions.
//!
//! Writing the stack out of a tree of values is what Waffle's `stackify` does
//! (<https://github.com/bytecodealliance/waffle>, Apache-2.0 WITH LLVM-exception, which the
//! Apache-2.0 half of this crate's licence is compatible with); no code is taken from it.
//!
//! [adr-0009]: ../../docs/adr/0009-pass-contract.md
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use std::fmt;

use mlkc_hir_def::Name;
use mlkc_lir_wasm::{BlockId, Body, Node, Op, RefTy, Terminator, Ty, ValueId};
use mlkc_mir::FunctionLoc as MirFunctionLoc;
use mlkc_span::Span;
use wasm_encoder::{
    AbstractHeapType, BlockType, Function, HeapType, Instruction, RefType, ValType,
};

use crate::{
    module::{ClosurePlan, FnSignature, ModuleLayout},
    select,
};

/// What codegen reads of one body besides the body itself ([ADR-0009][adr-0009]).
///
/// [adr-0009]: ../../docs/adr/0009-pass-contract.md
#[derive(Debug)]
pub struct FunctionCtx<'a> {
    /// The function whose code this is.
    pub function: &'a MirFunctionLoc,
    /// The name the function is called by, for the debug tables.
    pub name: &'a str,
    /// What the function takes and gives back, without the environment.
    pub signature: &'a FnSignature,
    /// The name of every ABI parameter, in order: a lambda is entered with its environment
    /// first, which no name binds.
    pub param_names: &'a [Option<Name>],
    /// How the functions of the module are numbered, and what they are.
    pub layout: &'a ModuleLayout,
    /// The closure of a lambda whose body this is; `None` for a function that is not one.
    pub lambda: Option<&'a ClosurePlan>,
}

impl FunctionCtx<'_> {
    /// What the body is entered with: the environment first for a lambda, then the declared
    /// parameters, in the ABI ([ADR-0026][adr-0026]).
    ///
    /// [adr-0026]: ../../docs/adr/0026-closure-representation.md
    pub(crate) fn param_types(&self) -> Vec<Ty> {
        let mut types = Vec::new();

        if let Some(lambda) = self.lambda {
            types.push(Ty::Ref(RefTy::Type(lambda.types.closure)));
        }

        types.extend(
            self.signature
                .shape(self.layout.builtins())
                .params
                .iter()
                .map(|abi| select::ty_of(*abi)),
        );

        types
    }
}

/// One compiled function: its body bytes, and where its instructions came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncArtifact {
    /// The encoded body, without the size prefix.
    pub body: Vec<u8>,
    /// Source positions of instructions inside `body`, by byte offset.
    pub origins: Vec<Origin>,
    /// Every value that lives in a local, for the debugger and for the assembler; a value the
    /// allocation pass emits where it is read lives in no local.
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

/// What a value is, and where it lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDebug {
    /// The value of the LIR.
    pub value: ValueId,
    /// The type of the machine value.
    pub ty: Ty,
    /// The WASM local the value lives in.
    pub local: u32,
    /// The name the value was bound under in the source, if it has one.
    pub name: Option<Name>,
}

/// What codegen reports about a body it cannot emit.
///
/// A backend does not reject a well-formed body for its shape ([ADR-0022][adr-0022]); what it
/// reports is a construct it does not lower *yet*, and the driver drops the artifact with it.
/// Such a construct is selected only for a feature the language has and the backend does not:
/// a string constant, and a call to a function declared inside a body.
///
/// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenDiag {
    /// The emitter met a construct it does not lower yet.
    Unsupported {
        /// What the construct is, as a phrase a person reads: "a string constant".
        what: &'static str,
        /// Where it is written.
        span: Span,
    },
    /// The emitter met a body that breaks an invariant of the LIR.
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

/// Emits the WASM function body of one body of the LIR.
///
/// # Panics
///
/// Panics when `lir` is not a well-formed body, or when its allocation pass has not run, which
/// is the contract the stage before codegen promises ([ADR-0022][adr-0022]): a body that breaks
/// it is a compiler bug.
///
/// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
pub fn emit_function(lir: &Body, ctx: &FunctionCtx<'_>) -> (FuncArtifact, Vec<CodegenDiag>) {
    if let Err(invalid) = lir.validate() {
        panic!(
            "the input of the WASM back end is a well-formed body, and this one is not: {invalid}"
        );
    }

    if let Err(invalid) = lir.validate_locals() {
        panic!("the WASM back end encodes a body whose allocation has not run: {invalid}");
    }

    if lir.structure.is_some()
        && let Err(invalid) = lir.validate_structure()
    {
        panic!("the WASM back end encodes a body whose structure is not well-formed: {invalid}");
    }

    assert_eq!(
        lir.params
            .iter()
            .map(|value| lir.values[*value].ty)
            .collect::<Vec<_>>(),
        ctx.param_types(),
        "a body to have the parameters its signature declares",
    );

    let mut emitter = Emitter::new(lir, ctx);

    emitter.run();

    emitter.finish()
}

/// The WASM type of a type of the LIR.
///
/// A concrete reference type is declared nullable: a local of the type may stand for a value
/// that is not one of the target's defaultable types, and the encoder writes a null into it
/// before the validator requires it to be initialized. A read of such a local is followed by
/// `ref.as_non_null`, so what an instruction reads is never null.
fn val_type(ty: Ty) -> ValType {
    match ty {
        Ty::I32 => ValType::I32,
        Ty::Ref(RefTy::I31) => {
            ValType::Ref(RefType::new_abstract(AbstractHeapType::I31, false, false))
        },
        Ty::Ref(RefTy::Eq) => ValType::Ref(RefType::EQREF),
        Ty::Ref(RefTy::Type(index)) => {
            ValType::Ref(RefType {
                nullable: true,
                heap_type: HeapType::Concrete(index),
            })
        },
    }
}

/// Collects the blocks a structure closes a `block` around: the joins of the body.
fn collect_joins(nodes: &[Node], joins: &mut Vec<BlockId>) {
    for node in nodes {
        match node {
            Node::Block { out, body } => {
                joins.push(*out);
                collect_joins(body, joins);
            },
            Node::Loop { body, .. } => collect_joins(body, joins),
            Node::If { then_, else_, .. } => {
                collect_joins(then_, joins);
                collect_joins(else_, joins);
            },
            Node::Leaf { .. }
            | Node::Params { .. }
            | Node::Br { .. }
            | Node::Return { .. }
            | Node::Unreachable { .. } => {},
        }
    }
}

/// The one emitter of one function.
struct Emitter<'a> {
    lir: &'a Body,
    ctx: &'a FunctionCtx<'a>,
    /// The instruction that defines every value, by the value's place in its arena.
    definitions: Vec<Option<(BlockId, usize)>>,
    /// The block being emitted.
    current: BlockId,
    function: Function,
    origins: Vec<Origin>,
    diags: Vec<CodegenDiag>,
    /// Set once a construct could not be emitted: the rest of the body is skipped.
    failed: bool,
}

impl<'a> Emitter<'a> {
    fn new(lir: &'a Body, ctx: &'a FunctionCtx<'a>) -> Self {
        let locals = lir.locals.locals.iter().map(|ty| val_type(*ty));
        let definitions = lir.definitions();

        Self {
            lir,
            ctx,
            definitions,
            current: lir.entry,
            function: Function::new_with_locals_types(locals),
            origins: Vec::new(),
            diags: Vec::new(),
            failed: false,
        }
    }

    /// Emits the whole body: the structure the structuring pass built, or the dispatch loop
    /// where the body has none.
    fn run(&mut self) {
        let lir = self.lir;

        self.initialize();

        match &lir.structure {
            Some(structure) => self.structured(&structure.nodes),
            None => self.dispatch(),
        }
    }

    /// Emits the structured control flow of a body.
    ///
    /// The nodes are the frames of the target in the order they stand, and a leaf is where the
    /// instructions of one block are written: what the terminator of a block becomes is the
    /// nodes around its leaf, so encoding makes no decision here.
    fn structured(&mut self, nodes: &'a [Node]) {
        for node in nodes {
            if self.failed {
                return;
            }

            match node {
                Node::Block { body, .. } => {
                    self.insn(Span::dummy(), Instruction::Block(BlockType::Empty));
                    self.structured(body);
                    self.insn(Span::dummy(), Instruction::End);
                },
                Node::Loop { body, .. } => {
                    self.insn(Span::dummy(), Instruction::Loop(BlockType::Empty));
                    self.structured(body);
                    self.insn(Span::dummy(), Instruction::End);
                },
                Node::If {
                    cond,
                    then_,
                    else_,
                    span,
                } => {
                    self.value(*cond, *span);
                    self.insn(*span, Instruction::If(BlockType::Empty));
                    self.structured(then_);
                    self.insn(*span, Instruction::Else);
                    self.structured(else_);
                    self.insn(*span, Instruction::End);
                },
                Node::Leaf { block } => self.leaf(*block),
                Node::Params { target, args, span } => self.params(*target, args, *span),
                Node::Br { depth, span, .. } => self.insn(*span, Instruction::Br(*depth)),
                Node::Return { value, span } => {
                    self.value(*value, *span);
                    self.insn(*span, Instruction::Return);
                },
                Node::Unreachable { span } => self.insn(*span, Instruction::Unreachable),
            }
        }
    }

    /// Emits the instructions of a block, where the structure puts it.
    fn leaf(&mut self, block: BlockId) {
        let lir = self.lir;

        self.current = block;
        self.insts(&lir.blocks[block].insts);
    }

    /// Emits the values an edge passes, and the stores of the parameters they are for.
    fn params(&mut self, target: BlockId, args: &'a [ValueId], span: Span) {
        let lir = self.lir;
        let params = &lir.blocks[target].params;

        for arg in args {
            self.value(*arg, span);
        }

        // The stores are in reverse, so that an edge which permutes the parameters of its
        // target reads every argument before it writes any of them.
        for param in params.iter().rev() {
            let local = lir.locals.values[param.index()]
                .expect("a parameter of a block to live in a local");

            self.insn(span, Instruction::LocalSet(local));
        }
    }

    /// Emits what a body needs before its code runs.
    ///
    /// A local of a reference type that is not nullable --- an `(ref i31)` --- has no default
    /// value, and WASM's validator tracks whether one is initialized frame by frame: a store
    /// made inside a frame is forgotten where the frame ends. A dispatched body may read any
    /// local in any case, so every such local is initialized here. A structured body needs the
    /// same for the parameters of its joins: the branches that leave the arms of an `if` store
    /// them, and the reads stand after the frame the arms are in.
    ///
    /// What the initialization writes is never read in either form: SSA defines every value
    /// before its use, every edge stores what its target reads, and a parameter of the function
    /// is written by the caller. A `pc` of a body that is entered at a block that is not the
    /// first case is initialized as well, because the default of the local is case zero.
    fn initialize(&mut self) {
        if let Some(structure) = &self.lir.structure {
            let mut joins = Vec::new();

            collect_joins(&structure.nodes, &mut joins);

            for join in joins {
                for param in &self.lir.blocks[join].params {
                    let local = self.lir.locals.values[param.index()]
                        .expect("a parameter of a block to live in a local");

                    self.default_of(local, self.lir.values[*param].ty);
                }
            }

            return;
        }

        if self.lir.entry.index() != 0 {
            self.insn(
                Span::dummy(),
                Instruction::I32Const(self.lir.entry.index() as i32),
            );
            self.insn(Span::dummy(), Instruction::LocalSet(self.pc()));
        }

        let parameters = self.lir.params.len() as u32;

        for (index, ty) in self.lir.locals.locals.iter().enumerate() {
            let local = parameters + index as u32;

            self.default_of(local, *ty);
        }
    }

    /// Emits a default for a local whose type has none, and nothing for every other local.
    ///
    /// A concrete reference type has no default, so the local is initialized with the null of
    /// it; a read of the local is followed by `ref.as_non_null`, which never traps because SSA
    /// defines every value before its use.
    fn default_of(&mut self, local: u32, ty: Ty) {
        match ty {
            Ty::I31 => {
                self.insn(Span::dummy(), Instruction::I32Const(0));
                self.insn(Span::dummy(), Instruction::RefI31);
                self.insn(Span::dummy(), Instruction::LocalSet(local));
            },
            Ty::Ref(RefTy::Type(index)) => {
                self.insn(
                    Span::dummy(),
                    Instruction::RefNull(HeapType::Concrete(index)),
                );
                self.insn(Span::dummy(), Instruction::LocalSet(local));
            },
            Ty::I32 | Ty::Ref(RefTy::Eq) => {},
        }
    }

    /// Emits every block of the body as a case of a dispatch loop.
    fn dispatch(&mut self) {
        let lir = self.lir;
        let count = lir.blocks.len() as u32;
        let pc = self.pc();

        // `done` wraps the loop so that an out-of-range `pc` has somewhere to go; every
        // terminator branches to the loop or returns, so the fall-through is unreachable.
        self.insn(Span::dummy(), Instruction::Block(BlockType::Empty));
        self.insn(Span::dummy(), Instruction::Loop(BlockType::Empty));

        // One case block per LIR block, innermost first; branching to the `i`th label lands
        // right after the `i`th `end`, which is where the code of the block is emitted.
        for _ in 0..count {
            self.insn(Span::dummy(), Instruction::Block(BlockType::Empty));
        }

        self.insn(Span::dummy(), Instruction::LocalGet(pc));
        self.insn(
            Span::dummy(),
            Instruction::BrTable((0..count).collect::<Vec<_>>().into(), count + 1),
        );

        // The first `end` closes the innermost case; after it comes the code of block zero.
        self.insn(Span::dummy(), Instruction::End);

        let ids: Vec<BlockId> = lir.blocks.iter().map(|(id, _)| id).collect();

        for (index, id) in ids.iter().enumerate() {
            let index = index as u32;
            let block = &lir.blocks[*id];

            // The loop is as many labels above this point as there are case blocks left.
            let dispatch_depth = count - 1 - index;

            self.current = *id;
            self.insts(&block.insts);
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

    /// Emits the instructions of a block that live in locals.
    ///
    /// An instruction whose value the allocation pass left out is emitted where the value is
    /// read, which is [`Emitter::inlined`].
    fn insts(&mut self, insts: &'a [mlkc_lir_wasm::Inst]) {
        for inst in insts {
            if self.failed {
                return;
            }

            let Some(local) = self.lir.locals.values[inst.value.index()] else {
                continue;
            };

            self.emit_inst(inst);
            self.insn(inst.span, Instruction::LocalSet(local));
        }
    }

    /// Emits one instruction, leaving its value on the stack.
    fn emit_inst(&mut self, inst: &'a mlkc_lir_wasm::Inst) {
        let span = inst.span;

        match &inst.op {
            Op::I32Const(value) => self.insn(span, Instruction::I32Const(*value)),
            Op::I32Eqz(operand) => {
                self.value(*operand, span);
                self.insn(span, Instruction::I32Eqz);
            },
            Op::I32Eq(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32Eq),
            Op::I32Ne(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32Ne),
            Op::I32LtS(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32LtS),
            Op::I32LeS(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32LeS),
            Op::I32GtS(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32GtS),
            Op::I32GeS(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32GeS),
            Op::I32Add(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32Add),
            Op::I32Sub(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32Sub),
            Op::I32Mul(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32Mul),
            Op::I32DivS(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32DivS),
            Op::I32And(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32And),
            Op::I32Or(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::I32Or),
            Op::RefI31(operand) => {
                self.value(*operand, span);
                self.insn(span, Instruction::RefI31);
            },
            Op::I31GetS(operand) => {
                self.value(*operand, span);
                self.insn(span, Instruction::I31GetS);
            },
            Op::RefCast(ty, operand) => {
                self.value(*operand, span);
                self.insn(span, Instruction::RefCastNonNull(heap_type(*ty)));
            },
            Op::RefEq(lhs, rhs) => self.binary(*lhs, *rhs, span, Instruction::RefEq),
            Op::Call { function, args } => {
                for argument in args {
                    self.value(*argument, span);
                }

                self.insn(span, Instruction::Call(*function));
            },
            Op::CallRef {
                signature,
                callee,
                args,
            } => {
                for argument in args {
                    self.value(*argument, span);
                }

                self.value(*callee, span);
                self.insn(span, Instruction::CallRef(*signature));
            },
            Op::RefFunc { function } => self.insn(span, Instruction::RefFunc(*function)),
            Op::RefNull(ty) => self.insn(span, Instruction::RefNull(heap_type(*ty))),
            Op::StructNew { ty, fields } => {
                for field in fields {
                    self.value(*field, span);
                }

                self.insn(span, Instruction::StructNew(*ty));
            },
            Op::StructGet { ty, field, value } => {
                self.value(*value, span);
                self.insn(span, Instruction::StructGet {
                    struct_type_index: *ty,
                    field_index: *field,
                });
            },
            Op::String(_) => self.unsupported("a string constant", span),
        }
    }

    /// Emits a binary instruction over two values.
    fn binary(
        &mut self,
        lhs: ValueId,
        rhs: ValueId,
        span: Span,
        instruction: Instruction<'static>,
    ) {
        self.value(lhs, span);
        self.value(rhs, span);
        self.insn(span, instruction);
    }

    /// Emits a terminator. `dispatch` is the depth of the dispatch loop where it is written,
    /// and `None` in a body of one block.
    fn terminator(&mut self, term: &'a Terminator, dispatch: Option<u32>) {
        match term {
            Terminator::Return { value, span } => {
                self.value(*value, *span);
                self.insn(*span, Instruction::Return);
            },
            Terminator::Unreachable { span } => self.insn(*span, Instruction::Unreachable),
            Terminator::Goto { target, span } => {
                let depth = dispatch.expect("a body that goes to a block to be dispatched");

                self.edge(target, depth, *span);
            },
            Terminator::Branch {
                cond,
                then_,
                else_,
                span,
            } => {
                let depth = dispatch.expect("a body that branches to be dispatched");

                self.value(*cond, *span);
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
                let depth = dispatch.expect("a body that switches to be dispatched");
                let scratch = self
                    .lir
                    .locals
                    .scratch
                    .expect("a body that switches to have a scratch local");

                self.value(*scrutinee, *span);
                self.insn(*span, Instruction::LocalSet(scratch));

                for (value, target) in arms {
                    self.insn(*span, Instruction::LocalGet(scratch));
                    self.insn(*span, Instruction::I32Const(*value));
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
    fn edge(&mut self, target: &'a mlkc_lir_wasm::BlockTarget, depth: u32, span: Span) {
        let destination = &self.lir.blocks[target.block];
        let mut locals = Vec::with_capacity(destination.params.len());

        for param in &destination.params {
            locals.push(
                self.lir.locals.values[param.index()]
                    .expect("a parameter of a block to live in a local"),
            );
        }

        for argument in &target.args {
            self.value(*argument, span);
        }

        // The stores are in reverse, so that an edge which permutes the parameters of its
        // target reads every argument before it writes any of them.
        for local in locals.iter().rev() {
            self.insn(span, Instruction::LocalSet(*local));
        }

        self.insn(span, Instruction::I32Const(target.block.index() as i32));
        self.insn(span, Instruction::LocalSet(self.pc()));
        self.insn(span, Instruction::Br(depth));
    }

    /// Puts a value on the stack: from the local it lives in, or by emitting the instruction
    /// that defines it, where the allocation pass left it out.
    fn value(&mut self, value: ValueId, span: Span) {
        let Some(local) = self.lir.locals.values[value.index()] else {
            self.inlined(value);

            return;
        };

        self.insn(span, Instruction::LocalGet(local));

        // A local of a concrete reference type is declared nullable, and what reads it wants
        // a value of the type: the read is narrowed where it stands.
        if matches!(self.lir.values[value].ty, Ty::Ref(RefTy::Type(_))) {
            self.insn(span, Instruction::RefAsNonNull);
        }
    }

    /// Emits the instruction that defines a value the allocation pass left out.
    ///
    /// The pass leaves a value out when it is read once, in the block that defines it, or when
    /// it is a constant, which is written again wherever it is read.
    fn inlined(&mut self, value: ValueId) {
        let (block, at) = self.definitions[value.index()]
            .expect("a value emitted where it is read to be defined by an instruction");
        let inst = &self.lir.blocks[block].insts[at];

        debug_assert!(
            block == self.current || matches!(inst.op, Op::I32Const(_)),
            "a value emitted in another block to be a constant",
        );

        self.emit_inst(inst);
    }

    /// The local the dispatch form keeps its program counter in.
    fn pc(&self) -> u32 {
        self.lir
            .locals
            .pc
            .expect("a dispatched body to have a program counter local")
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

    /// What every value that lives in a local is, and where it lives.
    fn debug(&self) -> Vec<ValueDebug> {
        let lir = self.lir;
        let mut names: Vec<Option<Name>> = vec![None; lir.values.len()];

        for (index, value) in lir.params.iter().enumerate() {
            names[value.index()] = self.ctx.param_names.get(index).cloned().unwrap_or_default();
        }

        lir.values
            .iter()
            .filter_map(|(value, data)| {
                Some(ValueDebug {
                    value,
                    ty: data.ty,
                    local: lir.locals.values[value.index()]?,
                    name: names[value.index()].clone(),
                })
            })
            .collect()
    }
}

/// The heap type of a reference type of the LIR.
fn heap_type(ty: RefTy) -> HeapType {
    match ty {
        RefTy::I31 => HeapType::I31,
        RefTy::Type(index) => HeapType::Concrete(index),
        // A cast to a nullable reference is not a value of the LIR: the verifier rejects it,
        // and this is the encoder not being asked to write one.
        RefTy::Eq => unreachable!("a cast to a nullable reference"),
    }
}
