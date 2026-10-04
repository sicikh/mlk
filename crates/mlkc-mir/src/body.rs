//! The MIR of one body: a uniform control-flow graph over words.
//!
//! One set of types serves the two forms of [ADR-0019][adr-0019]: the CFG form, where an
//! assignment writes a slot and a join is a slot the predecessors assigned, and the SSA form,
//! where every value is defined once and a join is a block parameter. What tells them apart is
//! their invariants ([`crate::verify`]), never their types.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md

use mlkc_hir_def::{BodyEntityLoc, EntityLoc, FunctionLoc, LocalFunctionId, Name};
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

/// The id of a lambda in the arena of one body.
pub type LambdaId = Idx<LambdaData>;

/// The MIR of one body.
///
/// The body of an entity that owns one is addressed by the entity's name; a function declared
/// inside a body has [`Body::local`] set, and its owner is the entity that declares it. The
/// lambdas written in the body --- and in its lambdas, all of them --- are [ADR-0026]'s
/// [`Body::lambdas`].
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    /// The entity whose body this is.
    pub owner: BodyEntityLoc,
    /// The function declared inside a body this body is, if it is one.
    pub local: Option<LocalFunction>,
    /// The values the owner is entered with: one per parameter, in the order they are declared.
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
    /// The lambdas written in the body, its own lambdas' lambdas among them, in the order the
    /// lowering made them ([ADR-0026][adr-0026]).
    ///
    /// The arena is flat: a lambda of a lambda is an entry of the same arena, and which
    /// lambda wrote it is read off the [`Rvalue::Closure`] that creates it.
    ///
    /// [adr-0026]: ../../docs/adr/0026-closure-representation.md
    pub lambdas: Arena<LambdaData>,
    /// The functions declared inside this body, in the order the body declares them
    /// ([ADR-0019][adr-0019]).
    ///
    /// The list is flat: a function declared in a `local` written inside another one is an entry
    /// of the same list, the way the arena of the HIR body is flat. A body of one is a body of
    /// its own --- its parameters, its blocks and its slots are its own --- and the rest of what
    /// the function is, the name it is declared under and the type the checker gave it, is
    /// [`LocalFunction`].
    ///
    /// A function declared inside a body captures nothing: it is given its own parameters only,
    /// and what it reads of the module it reaches without an environment. Its code is lifted
    /// into a function of the module the back end numbers like any other ([ADR-0020]).
    ///
    /// [adr-0019]: ../../docs/adr/0019-mir.md
    /// [adr-0020]: ../../docs/adr/0020-wasm-backend.md
    pub local_functions: Vec<Body>,
}

/// A function declared inside a body, as the body it is ([ADR-0019][adr-0019]).
///
/// What a function declared inside a body is made of is the body itself; this is the identity
/// around it: the place it holds in the arena of the body that declares it, the name it is
/// declared under, the type the checker gave it, and the names of its parameters.
///
/// [adr-0019]: ../../docs/adr/0019-mir.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFunction {
    /// The function, by its place in the arena of the body that declares it.
    pub id: LocalFunctionId,
    /// The name the function is declared under.
    pub name: Name,
    /// The type the checker gave it: what a call of it takes and gives back.
    pub ty: Ty,
    /// The name every parameter was declared under, in order; `None` for a pattern with no name.
    pub param_names: Vec<Option<Name>>,
}

/// The code of one lambda: a control-flow graph over words, as a body is.
///
/// A lambda is a value whose code the back end lifts into a function of its own
/// ([ADR-0026][adr-0026]): the type the checker gave it says how the code crosses the ABI, the
/// captures say what its environment holds, and the rest is a body of its own --- with its own
/// blocks, values, and slots, because an SSA form and a local space are per function.
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaData {
    /// The type the checker gave the lambda: what a closure of it takes and gives back.
    pub ty: Ty,
    /// The bindings the lambda captured, in the free-variable order of the HIR.
    pub captures: Vec<CaptureData>,
    /// The parameters, in the order they are declared.
    pub params: Vec<ValueId>,
    /// The name every parameter was declared under, in order; `None` for a pattern with no name.
    pub param_names: Vec<Option<Name>>,
    /// The block the lambda enters.
    pub entry: BlockId,
    /// The blocks.
    pub blocks: Arena<Block>,
    /// The values: the parameters of the lambda, and the definitions of the statements.
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

/// A closure a piece of code creates: the lambda it is made of, and where it is written.
///
/// A lambda is created by exactly one `Rvalue::Closure`, in the code that wrote the expression
/// ([ADR-0026][adr-0026]), and this is what a pass that walks the lambdas of a body reads.
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LambdaClosure {
    /// The lambda the closure is made of.
    pub lambda: LambdaId,
    /// Where the expression that creates it is written.
    pub span: Span,
}

/// The lambdas a piece of code creates itself, in the order it creates them.
///
/// The walk reads every block, because a lambda created in a block no path reaches is still
/// a lambda of the code, and the order is the order the blocks and the statements list. A lambda
/// the code creates once is listed once.
pub fn lambda_closures(code: CodeRef<'_>) -> Vec<LambdaClosure> {
    let mut closures: Vec<LambdaClosure> = Vec::new();

    for (_, block) in code.blocks.iter() {
        for stmt in &block.stmts {
            let StmtKind::Assign {
                rvalue: Rvalue::Closure { lambda, .. },
                ..
            } = &stmt.kind
            else {
                continue;
            };

            if !closures.iter().any(|closure| closure.lambda == *lambda) {
                closures.push(LambdaClosure {
                    lambda: *lambda,
                    span: stmt.span,
                });
            }
        }
    }

    closures
}

/// What a pass reads of a piece of code: the code of a [`Body`] or of a [`LambdaData`].
///
/// A pass that works on code and not on the identity a body carries --- the SSA construction,
/// the verifier, the dumps, the selection of the back end --- reads this, and one function
/// serves the body of an entity and the body of a lambda alike ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
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
}

impl LambdaData {
    /// What a pass reads of the lambda's code, whatever the lambda owns.
    pub fn code(&self) -> CodeRef<'_> {
        CodeRef {
            params: &self.params,
            entry: self.entry,
            blocks: &self.blocks,
            values: &self.values,
            locals: &self.locals,
        }
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
    /// A function of the project, in this module or in another.
    Entity(EntityLoc<FunctionLoc>),
    /// A function declared inside the enclosing body.
    Local(LocalFunctionId),
    /// A function held in an operand: later, when functions become values.
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
#[derive(Debug, Default)]
pub struct BodyBuilder {
    owner: Option<BodyEntityLoc>,
    local: Option<LocalFunction>,
    params: Vec<ValueId>,
    blocks: Arena<Block>,
    values: Arena<ValueData>,
    locals: Arena<LocalData>,
}

impl BodyBuilder {
    /// A builder of the body of `owner`.
    pub fn new(owner: BodyEntityLoc) -> Self {
        Self {
            owner: Some(owner),
            ..Self::default()
        }
    }

    /// Sets the function declared inside a body that the body is.
    pub fn local_function(mut self, local: LocalFunction) -> Self {
        self.local = Some(local);
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
    /// The code of a lambda is built the way the code of a body is; only the identity around it
    /// tells the two apart ([`Body`], [`LambdaData`]).
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
    ///
    /// The lambdas of the body are not the builder's: the lowering owns them and puts them in
    /// when the walk is over.
    ///
    /// # Panics
    ///
    /// Panics if the builder was made without an owner, which [`BodyBuilder::new`] does not
    /// let a caller do.
    pub fn finish(self, entry: BlockId) -> Body {
        let owner = self.owner.expect("a body has an owner");

        Body {
            owner,
            local: self.local,
            params: self.params,
            entry,
            blocks: self.blocks,
            values: self.values,
            locals: self.locals,
            lambdas: Arena::default(),
            local_functions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_reads_back_what_the_builder_allocated() {
        let owner = crate::test_support::owner();
        let mut builder = BodyBuilder::new(owner.clone());
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

        assert_eq!(body.owner, owner);
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
