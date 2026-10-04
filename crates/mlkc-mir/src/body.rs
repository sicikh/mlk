//! The MIR of one body: a uniform control-flow graph over words.
//!
//! One set of types serves the two forms of [ADR-0019][adr-0019]: the CFG form, where an
//! assignment writes a slot and a join is a slot the predecessors assigned, and the SSA form,
//! where every value is defined once and a join is a block parameter. What tells them apart is
//! their invariants ([`crate::verify`]), never their types.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md

use std::sync::Arc;

use mlkc_hir_def::{BodyEntityLoc, LocalFunctionId, ModuleId, Name};
use mlkc_hir_ty::Ty;
use mlkc_intern::Interned;
use mlkc_la_arena::{Arena, Idx};
use mlkc_span::Span;

/// The id of a block inside one body.
pub type BlockId = Idx<Block>;

/// The id of a value inside one body.
pub type ValueId = Idx<ValueData>;

/// The id of a slot inside one body.
pub type LocalId = Idx<LocalData>;

/// The id of a lambda inside the HIR body that wrote it.
///
/// A lambda has no name and no identity that outlives the body it is written in: the number is
/// its place in the order the lowering wrote it, and the HIR body is the rest of its name
/// ([ADR-0003][adr-0003]).
///
/// [adr-0003]: ../../docs/adr/0003-id-based-ir.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LambdaId(u32);

impl LambdaId {
    /// The lambda with this place in the order the lowering wrote it.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The place of the lambda in the order the lowering wrote it.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// What a lifted function is inside the HIR body that declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LiftedId {
    /// A function declared in a `local`, by its place in the arena of the HIR body.
    Local(LocalFunctionId),
    /// A lambda written in the HIR body.
    Lambda(LambdaId),
}

/// The location of a function of the project, as MIR addresses it.
///
/// Every function of MIR is a function of its module: the body of an entity, or a function
/// lifted out of one --- a function declared in a `local`, or a lambda. The HIR keeps those
/// inside the body that declares them, because a body is the unit of incrementality
/// ([ADR-0003][adr-0003]); MIR lifts them into functions of their own, so that every stage after
/// it handles a lifted function the way it handles one of the top level.
///
/// [adr-0003]: ../../docs/adr/0003-id-based-ir.md
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionLoc {
    /// The body of an entity of the project.
    Entity(BodyEntityLoc),
    /// A function lifted out of the body of an entity.
    Lifted {
        /// The entity whose body declared the function.
        origin: BodyEntityLoc,
        /// What the function is in that body.
        id: LiftedId,
    },
}

impl FunctionLoc {
    /// The entity whose body declared the function.
    pub fn origin(&self) -> &BodyEntityLoc {
        match self {
            Self::Entity(owner) => owner,
            Self::Lifted { origin, .. } => origin,
        }
    }

    /// The module the function belongs to.
    pub fn module(&self) -> ModuleId {
        self.origin().module()
    }

    /// The name of the function inside its module: the name of the entity it is, or the name of
    /// that entity and what the function is under it --- `fib`, `fib::aux`,
    /// `fib::<mlkc@lambda-0>`.
    ///
    /// `root` is the name of the entity whose body declared the function, and `declared` is the
    /// name the function was declared under, for a function declared in a `local`.
    pub fn name(&self, root: &str, declared: Option<&Name>) -> String {
        match self {
            Self::Entity(_) => root.to_owned(),
            Self::Lifted {
                id: LiftedId::Local(_),
                ..
            } => {
                let declared =
                    declared.expect("a function declared in a `local` to have a name of its own");

                format!("{root}::{declared}")
            },
            Self::Lifted {
                id: LiftedId::Lambda(lambda),
                ..
            } => format!("{root}::<mlkc@lambda-{}>", lambda.index()),
        }
    }

    /// Whether the function is a lambda.
    pub fn is_lambda(&self) -> bool {
        matches!(self, Self::Lifted {
            id: LiftedId::Lambda(_),
            ..
        })
    }

    /// The lambda the function is, if it is one.
    pub fn lambda(&self) -> Option<LambdaId> {
        match self {
            Self::Lifted {
                id: LiftedId::Lambda(lambda),
                ..
            } => Some(*lambda),
            _ => None,
        }
    }
}

/// The MIR of one HIR body: every function it declares, each a body of its own.
///
/// The list is flat: the body of the entity first, and then the functions lifted out of it ---
/// the functions declared in a `local`, and the lambdas --- in the order the lowering wrote
/// them. The HIR body is the unit of incrementality ([ADR-0003][adr-0003]), so one value holds
/// all of them; nothing after MIR reads the nesting, because there is none.
///
/// [adr-0003]: ../../docs/adr/0003-id-based-ir.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bodies {
    /// The bodies, the body of the entity itself first.
    pub bodies: Vec<Arc<Body>>,
}

impl Bodies {
    /// The body of the entity itself.
    pub fn root(&self) -> &Arc<Body> {
        self.bodies
            .first()
            .expect("a set of bodies to hold its root")
    }

    /// The body of a function, if the set holds it.
    pub fn get(&self, function: &FunctionLoc) -> Option<&Arc<Body>> {
        self.bodies.iter().find(|body| &body.function == function)
    }

    /// The bodies, the body of the entity first.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<Body>> {
        self.bodies.iter()
    }

    /// How many bodies the set holds.
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    /// Whether the set holds no body at all.
    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }
}

/// The MIR of one function: a control-flow graph over words, and what the function is.
///
/// The body of a function is a graph of its own --- its parameters, its blocks, its values, and
/// its slots are its own, because a function becomes a function of the machine and every stage
/// that works on code is stated per function ([ADR-0026][adr-0026]). A lambda is a function like
/// any other, entered with its environment at run time and with what it captured in
/// [`Body::captures`].
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    /// The function the body is.
    pub function: FunctionLoc,
    /// The name the function was declared under: the name of an entity, and of a function
    /// declared in a `local`; `None` for a lambda, which has no name of its own.
    pub name: Option<Name>,
    /// The type the checker gave the function: what a call of it takes and gives back.
    pub ty: Ty,
    /// The names of the parameters, in order; `None` where the source binds no name. A lambda is
    /// entered with its environment first at run time, which no name binds ([ADR-0026]).
    ///
    /// [adr-0026]: ../../docs/adr/0026-closure-representation.md
    pub param_names: Vec<Option<Name>>,
    /// What a lambda captured, in the free-variable order of the HIR; empty for any other
    /// function ([ADR-0026][adr-0026]).
    ///
    /// [adr-0026]: ../../docs/adr/0026-closure-representation.md
    pub captures: Vec<CaptureData>,
    /// The values the function is entered with: one per parameter, in the order they are declared.
    ///
    /// A parameter is a value and not a slot: the first statements of the entry block bind the
    /// parameter slots to it, and no form moves the list.
    pub params: Vec<ValueId>,
    /// The block a body enters.
    pub entry: BlockId,
    /// The blocks.
    pub blocks: Arena<Block>,
    /// The values: the parameters of the body, and the definitions of the statements.
    pub values: Arena<ValueData>,
    /// The slots of the CFG form; empty once the SSA pass has run.
    pub locals: Arena<LocalData>,
}

/// One binding a lambda captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureData {
    /// The name the binding was written under, if it was.
    pub name: Option<Name>,
    /// The type the checker gave the binding.
    pub ty: Ty,
    /// Where the binding is written, for the debug tables.
    pub span: Span,
}

/// What a pass reads of a piece of code: the code of a [`Body`].
///
/// A pass that works on code and not on the identity a body carries --- the SSA construction,
/// the verifier, the dumps, the selection of the back end --- reads this.
#[derive(Debug, Clone, Copy)]
pub struct CodeRef<'a> {
    /// The values the code is entered with, in the order they are declared.
    pub params: &'a [ValueId],
    /// The block the code enters.
    pub entry: BlockId,
    /// The blocks.
    pub blocks: &'a Arena<Block>,
    /// The values.
    pub values: &'a Arena<ValueData>,
    /// The slots of the CFG form.
    pub locals: &'a Arena<LocalData>,
}

/// The code a pass built, before it is given the identity of what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code {
    /// The values the code is entered with, in the order they are declared.
    pub params: Vec<ValueId>,
    /// The block the code enters.
    pub entry: BlockId,
    /// The blocks.
    pub blocks: Arena<Block>,
    /// The values.
    pub values: Arena<ValueData>,
    /// The slots of the CFG form.
    pub locals: Arena<LocalData>,
}

impl Code {
    /// What a pass reads of the code.
    pub fn as_ref(&self) -> CodeRef<'_> {
        CodeRef {
            params: &self.params,
            entry: self.entry,
            blocks: &self.blocks,
            values: &self.values,
            locals: &self.locals,
        }
    }
}

impl Body {
    /// What a pass reads of the body's code, whatever the body owns.
    pub fn code(&self) -> CodeRef<'_> {
        CodeRef {
            params: &self.params,
            entry: self.entry,
            blocks: &self.blocks,
            values: &self.values,
            locals: &self.locals,
        }
    }

    /// The entity whose body declared the function.
    pub fn origin(&self) -> &BodyEntityLoc {
        self.function.origin()
    }

    /// The module the function belongs to.
    pub fn module(&self) -> ModuleId {
        self.function.module()
    }

    /// Whether the body is the code of a lambda.
    pub fn is_lambda(&self) -> bool {
        self.function.is_lambda()
    }
}

/// One block: its parameters, its statements, and the terminator it ends in.
///
/// A block is entered through its parameters and ends in exactly one terminator;
/// there are no fall-throughs and no implicit joins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The block parameters: where a value coming from several predecessors is born.
    ///
    /// The CFG form has none; the SSA form has one per value born at a join.
    pub params: Vec<ValueId>,
    /// The statements, in the order they run.
    pub stmts: Vec<Stmt>,
    /// The terminator: where control goes.
    pub term: Terminator,
}

/// What a value is, for a dump, a verifier, and a debugger.
///
/// A value carries what a stage computes and nothing else: the span is where it was read, and
/// the type is what the checker gave the expression it is a value of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueData {
    /// Where the value is written.
    pub span: Span,
    /// The source type the checker gave the expression this value is.
    pub ty: Ty,
}

/// What a slot is, for a dump, a verifier, and a debugger.
///
/// A slot is a name of the CFG form: it is assigned any number of times, and it disappears
/// when the SSA form gives every value a definition of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalData {
    /// Where the slot is bound.
    pub span: Span,
    /// The name the slot was bound under, if it was bound under one.
    pub name: Option<Name>,
    /// The source type of what the slot holds.
    pub ty: Ty,
}

/// One statement, and where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    /// What the statement does.
    pub kind: StmtKind,
    /// Where the statement is written.
    pub span: Span,
}

/// What a statement does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    /// `place = rvalue`.
    Assign {
        /// What is written.
        place: Place,
        /// What is computed.
        rvalue: Rvalue,
    },
}

/// What an assignment writes: a slot of the CFG form, or a value of the SSA form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// A slot of the CFG form; it may be assigned many times.
    Local(LocalId),
    /// An SSA value; it is defined exactly once, by its statement.
    Value(ValueId),
}

/// What an operand reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    /// A value: what an SSA body passes and reads.
    Value(ValueId),
    /// A read of a slot; only in the CFG form.
    Local(LocalId),
    /// A constant.
    Const(Const),
}

/// What a statement computes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rvalue {
    /// A copy of an operand.
    Use(Operand),
    /// A constant.
    Const(Const),
    /// A call.
    Call {
        /// What is called.
        callee: Callee,
        /// The arguments, in the order they are passed; each is a word.
        args: Vec<Operand>,
    },
    /// A closure: the lambda that is its code, and the words it captures
    /// ([ADR-0026][adr-0026]).
    ///
    /// [adr-0026]: ../../docs/adr/0026-closure-representation.md
    Closure {
        /// The code.
        lambda: LambdaId,
        /// The captured words, one per capture of the lambda, in the same order.
        captures: Vec<Operand>,
    },
    /// The value of one binding the enclosing lambda captured; only in a lambda body.
    Capture {
        /// Which capture, by its place in the lambda's `captures` list.
        index: u32,
    },
    /// A word-level primitive; the operator says which.
    Prim {
        /// The operator.
        op: PrimOp,
        /// The operands, in the order the operator takes them.
        args: Vec<Operand>,
    },
}

/// What a call calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callee {
    /// A function of the project, in this module or in another: the body of an entity, a
    /// function declared in a `local`, or a lambda.
    Direct(FunctionLoc),
    /// A function held in an operand: a closure.
    Indirect(Operand),
}

/// A constant of the language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Const {
    /// A signed 31-bit integer; the range invariant of [ADR-0018][adr-0018].
    ///
    /// [adr-0018]: ../../docs/adr/0018-values-as-words.md
    Int(i32),
    /// A boolean.
    Bool(bool),
    /// The unit value.
    Unit,
    /// A string, which is a reference to a GC array.
    Str(Interned<str>),
}

/// A word-level primitive, named by its semantics and not by an instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimOp {
    /// The wrapping addition of two immediates.
    IntAdd,
    /// The wrapping subtraction of two immediates.
    IntSub,
    /// The wrapping multiplication of two immediates.
    IntMul,
    /// The division of two immediates; a zero divisor traps, and the one case that leaves the
    /// range wraps.
    IntDiv,
    /// The negation of an immediate; the one case that leaves the range wraps.
    IntNeg,
    /// Whether two immediates are equal.
    IntEq,
    /// Whether two immediates are not equal.
    IntNe,
    /// Whether the first immediate is less than the second.
    IntLt,
    /// Whether the first immediate is less than or equal to the second.
    IntLe,
    /// Whether the first immediate is greater than the second.
    IntGt,
    /// Whether the first immediate is greater than or equal to the second.
    IntGe,
    /// Whether both booleans are true.
    BoolAnd,
    /// Whether either boolean is true.
    BoolOr,
    /// The negation of a boolean.
    BoolNot,
    /// Whether two booleans are equal.
    BoolEq,
    /// Whether two booleans are not equal.
    BoolNe,
    /// The identity of two references.
    RefEq,
}

impl PrimOp {
    /// The operator as a dump reads it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IntAdd => "int-add",
            Self::IntSub => "int-sub",
            Self::IntMul => "int-mul",
            Self::IntDiv => "int-div",
            Self::IntNeg => "int-neg",
            Self::IntEq => "int-eq",
            Self::IntNe => "int-ne",
            Self::IntLt => "int-lt",
            Self::IntLe => "int-le",
            Self::IntGt => "int-gt",
            Self::IntGe => "int-ge",
            Self::BoolAnd => "bool-and",
            Self::BoolOr => "bool-or",
            Self::BoolNot => "bool-not",
            Self::BoolEq => "bool-eq",
            Self::BoolNe => "bool-ne",
            Self::RefEq => "ref-eq",
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
        /// The condition: a word the language reads as a boolean.
        cond: Operand,
        /// Where control goes when the condition holds.
        then_: BlockTarget,
        /// Where control goes when it does not.
        else_: BlockTarget,
        /// Where the terminator is written.
        span: Span,
    },
    /// Control goes to one of several blocks, by what a word is.
    Switch {
        /// The word that decides.
        scrutinee: Operand,
        /// The constants and where control goes for each of them.
        arms: Vec<(Const, BlockTarget)>,
        /// Where control goes when no arm matches.
        otherwise: BlockTarget,
        /// Where the terminator is written.
        span: Span,
    },
    /// The body gives back a word.
    Return {
        /// The word.
        value: Operand,
        /// Where the terminator is written.
        span: Span,
    },
    /// Control that is not meant to be reached.
    Unreachable {
        /// Where the terminator is written.
        span: Span,
    },
}

/// A block an edge goes to, and the arguments the edge passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTarget {
    /// The block.
    pub block: BlockId,
    /// The arguments: one per parameter of the block, in the order of the parameters.
    ///
    /// The CFG form passes none, because no block of it has parameters.
    pub args: Vec<Operand>,
}

/// Builds one body.
///
/// The builder owns the arenas, so that a stage allocates blocks, values, and slots in the
/// order it meets them, and refers to a block it has not filled yet by its id.
#[derive(Debug)]
pub struct BodyBuilder {
    function: FunctionLoc,
    name: Option<Name>,
    ty: Ty,
    param_names: Vec<Option<Name>>,
    captures: Vec<CaptureData>,
    params: Vec<ValueId>,
    blocks: Arena<Block>,
    values: Arena<ValueData>,
    locals: Arena<LocalData>,
}

impl BodyBuilder {
    /// A builder of the body of `function`, whose type the checker gave as `ty`.
    pub fn new(function: FunctionLoc, ty: Ty) -> Self {
        Self {
            function,
            name: None,
            ty,
            param_names: Vec::new(),
            captures: Vec::new(),
            params: Vec::new(),
            blocks: Arena::default(),
            values: Arena::default(),
            locals: Arena::default(),
        }
    }

    /// Sets the name the function was declared under.
    pub fn name(mut self, name: Option<Name>) -> Self {
        self.name = name;
        self
    }

    /// Sets the names of the parameters, in the order they are declared.
    pub fn param_names(mut self, names: Vec<Option<Name>>) -> Self {
        self.param_names = names;
        self
    }

    /// Sets what a lambda captured, in the free-variable order of the HIR.
    pub fn captures(mut self, captures: Vec<CaptureData>) -> Self {
        self.captures = captures;
        self
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

    /// Allocates a slot.
    pub fn local(&mut self, data: LocalData) -> LocalId {
        self.locals.alloc(data)
    }

    /// Allocates a block.
    pub fn block(&mut self, block: Block) -> BlockId {
        self.blocks.alloc(block)
    }

    /// The block with this id, to fill it after it was referred to.
    pub fn block_mut(&mut self, id: BlockId) -> &mut Block {
        &mut self.blocks[id]
    }

    /// Finishes the code, entered at `entry`.
    ///
    /// What the code is and what a lambda of it captured is the identity around it and not the
    /// code itself, so a caller that builds code and not a body of the language gets the code
    /// ([`Code`]) and not a body.
    pub fn code(self, entry: BlockId) -> Code {
        Code {
            params: self.params,
            entry,
            blocks: self.blocks,
            values: self.values,
            locals: self.locals,
        }
    }

    /// Finishes the body, entered at `entry`.
    pub fn finish(self, entry: BlockId) -> Body {
        Body {
            function: self.function,
            name: self.name,
            ty: self.ty,
            param_names: self.param_names,
            captures: self.captures,
            params: self.params,
            entry,
            blocks: self.blocks,
            values: self.values,
            locals: self.locals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_reads_back_what_the_builder_allocated() {
        let owner = crate::test_support::owner();
        let function = FunctionLoc::Entity(owner.clone());
        let mut builder = BodyBuilder::new(function.clone(), Ty::Error);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        });
        let slot = builder.local(LocalData {
            span: Span::dummy(),
            name: Some(Name::new("value")),
            ty: Ty::Error,
        });
        let value = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        });
        let block = builder.block(Block {
            params: Vec::new(),
            stmts: vec![
                Stmt {
                    kind: StmtKind::Assign {
                        place: Place::Local(slot),
                        rvalue: Rvalue::Use(Operand::Value(param)),
                    },
                    span: Span::dummy(),
                },
                Stmt {
                    kind: StmtKind::Assign {
                        place: Place::Value(value),
                        rvalue: Rvalue::Prim {
                            op: PrimOp::IntNeg,
                            args: vec![Operand::Local(slot)],
                        },
                    },
                    span: Span::dummy(),
                },
            ],
            term: Terminator::Return {
                value: Operand::Value(value),
                span: Span::dummy(),
            },
        });
        let body = builder.finish(block);

        assert_eq!(body.function, function);
        assert_eq!(body.origin(), &owner);
        assert_eq!(body.params, [param]);
        assert_eq!(body.entry, block);
        assert_eq!(body.blocks[block].stmts.len(), 2);
        assert_eq!(body.locals[slot].name, Some(Name::new("value")));
        assert_eq!(body.blocks[block].term, Terminator::Return {
            value: Operand::Value(value),
            span: Span::dummy(),
        },);
    }
}
