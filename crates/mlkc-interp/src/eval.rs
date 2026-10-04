//! Running one body: the evaluation loop of the interpreter.
//!
//! A call makes a frame and runs the entry block; a block runs its statements and then its
//! terminator; an edge evaluates its arguments before it stores any of them, the way the WASM
//! back end does ([ADR-0020][adr-0020]). Nothing of the loop depends on which form of MIR it
//! reads ([ADR-0019][adr-0019]): a value and a slot are two cases of the same operand.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md

use std::{fmt, sync::Arc};

use mlkc_hir_def::{EntityLoc, FunctionLoc, ItemLocLike};
use mlkc_mir::{
    BlockTarget, Body, Callee, Const, Operand, Place, PrimOp, Rvalue, Stmt, StmtKind, Terminator,
};
use mlkc_span::Span;

use crate::{Extern, Program, Value};

/// The functions of the program that are not written in it.
///
/// A host is what the interpreter asks for an extern function: the tests collect what a program
/// prints, and a real host --- a process, a browser --- does what the name says.
pub trait Host {
    /// Calls `function` with `args`, in the order they are passed, and reads the word back.
    ///
    /// `function` is the canonical name of [ADR-0021][adr-0021], so a host that implements
    /// `print-int` of `std::runtime` sees the whole name and cannot confuse it with the
    /// `print-int` of another module.
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    fn call(&mut self, function: &Extern, args: &[Value]) -> Result<Value, Trap>;
}

/// What stopped a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trap {
    /// The program divided an immediate by zero.
    DivideByZero {
        /// Where the operator is written.
        span: Span,
    },
    /// The program reached a terminator that is not meant to be reached.
    Unreachable {
        /// Where the terminator is written.
        span: Span,
    },
    /// The program read a value or a slot that was not written on the way to it.
    ///
    /// A body that does this breaks the invariants of MIR, so this is a bug of the stages before
    /// the interpreter and not a state a program reaches.
    Uninitialized {
        /// Where the read is written.
        span: Span,
    },
    /// The program called a function that is neither written in it nor declared external.
    MissingBody {
        /// The function.
        entity: EntityLoc<FunctionLoc>,
    },
    /// The interpreter does not run the construct yet.
    Unsupported {
        /// What the construct is, as a phrase a person reads: "a string constant".
        what: &'static str,
        /// Where it is written.
        span: Span,
    },
    /// The host refused the call of an external function.
    Host {
        /// What the host reported.
        message: String,
    },
}

impl Trap {
    /// A mistake the host reports about the program.
    pub fn host(message: impl Into<String>) -> Self {
        Self::Host {
            message: message.into(),
        }
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivideByZero { .. } => f.write_str("the program divided by zero"),
            Self::Unreachable { .. } => {
                f.write_str("the program reached a terminator it cannot reach")
            },
            Self::Uninitialized { .. } => {
                f.write_str("the program read a word that was not written")
            },
            Self::MissingBody { entity } => {
                let name = entity
                    .item
                    .name()
                    .map_or_else(|| format!("{:?}", entity.item), ToString::to_string);

                write!(
                    f,
                    "the program called `{name}`, which has no body and no host"
                )
            },
            Self::Unsupported { what, .. } => {
                write!(f, "the interpreter does not run {what} yet")
            },
            Self::Host { message } => write!(f, "the host refused a call: {message}"),
        }
    }
}

impl std::error::Error for Trap {}

/// Runs `function` with `args`, and gives back the word it returns.
///
/// The body is looked up in `program`, and a function the program does not write is asked of
/// `host` ([`Host`]).
///
/// # Panics
///
/// Panics when the number of arguments is not the number of parameters of the body, or when an
/// edge passes a number of arguments its target does not take: a body whose meaning this is
/// breaks the invariants of MIR, and the stages before the interpreter promise them.
pub fn run(
    program: &Program,
    function: &EntityLoc<FunctionLoc>,
    args: &[Value],
    host: &mut dyn Host,
) -> Result<Value, Trap> {
    Interpreter { program, host }.call(function, args)
}

/// One run: the program it reads, and the host it asks.
struct Interpreter<'a> {
    program: &'a Program,
    host: &'a mut dyn Host,
}

/// The words of one call: the values of the SSA form, and the slots of the CFG form.
///
/// A read of a word that was not written is a trap and not a panic: the interpreter checks the
/// invariant the same way the verifier states it, and reports the place it met.
struct Frame {
    values: Vec<Option<Value>>,
    locals: Vec<Option<Value>>,
}

impl<'a> Interpreter<'a> {
    /// Calls a function of the program, or the host of an extern one.
    fn call(&mut self, function: &EntityLoc<FunctionLoc>, args: &[Value]) -> Result<Value, Trap> {
        let Some(body) = self.program.body(function) else {
            if let Some(external) = self.program.external(function) {
                return self.host.call(external, args);
            }

            return Err(Trap::MissingBody {
                entity: function.clone(),
            });
        };

        let body = Arc::clone(body);

        self.eval_body(&body, args)
    }

    /// Runs one body: a frame holds its words, and the loop walks its blocks.
    fn eval_body(&mut self, body: &Body, args: &[Value]) -> Result<Value, Trap> {
        assert_eq!(
            args.len(),
            body.params.len(),
            "a call passes one word per parameter of the body",
        );

        let mut frame = Frame {
            values: vec![None; body.values.len()],
            locals: vec![None; body.locals.len()],
        };

        for (parameter, argument) in body.params.iter().zip(args) {
            frame.values[parameter.index()] = Some(argument.clone());
        }

        let mut block = body.entry;

        loop {
            let data = &body.blocks[block];

            for stmt in &data.stmts {
                self.stmt(&mut frame, stmt)?;
            }

            match &data.term {
                Terminator::Goto { target, span } => {
                    self.edge(body, &mut frame, target, *span)?;
                    block = target.block;
                },
                Terminator::Branch {
                    cond,
                    then_,
                    else_,
                    span,
                } => {
                    let cond = self.operand(&frame, cond, *span)?;
                    let target = if cond.truth() { then_ } else { else_ };

                    self.edge(body, &mut frame, target, *span)?;
                    block = target.block;
                },
                Terminator::Switch {
                    scrutinee,
                    arms,
                    otherwise,
                    span,
                } => {
                    let scrutinee = self.operand(&frame, scrutinee, *span)?;
                    let payload = immediate(&scrutinee, *span)?;
                    let mut target = otherwise;

                    for (constant, arm) in arms {
                        if constant_payload(constant, *span)? == payload {
                            target = arm;
                            break;
                        }
                    }

                    self.edge(body, &mut frame, target, *span)?;
                    block = target.block;
                },
                Terminator::Return { value, span } => {
                    return self.operand(&frame, value, *span);
                },
                Terminator::Unreachable { span } => {
                    return Err(Trap::Unreachable { span: *span });
                },
            }
        }
    }

    /// Runs one statement: computes what it reads, and writes it where it says.
    fn stmt(&mut self, frame: &mut Frame, stmt: &Stmt) -> Result<(), Trap> {
        let StmtKind::Assign { place, rvalue } = &stmt.kind;

        let word = match rvalue {
            Rvalue::Use(operand) => self.operand(frame, operand, stmt.span)?,
            Rvalue::Const(constant) => constant_value(constant, stmt.span)?,
            Rvalue::Prim { op, args } => {
                let args = self.operands(frame, args, stmt.span)?;

                self.prim(*op, &args, stmt.span)?
            },
            Rvalue::Call { callee, args } => {
                let args = self.operands(frame, args, stmt.span)?;

                self.callee(callee, &args, stmt.span)?
            },
            // A closure is code plus the words it captured, and `Capture` reads one of them
            // ([ADR-0026][adr-0026]); the value model of the interpreter is its own, and the
            // lowering of both waits for a decision that is not MIR's.
            //
            // [adr-0026]: ../../docs/adr/0026-closure-representation.md
            Rvalue::Closure { .. } => {
                return Err(Trap::Unsupported {
                    what: "a closure",
                    span: stmt.span,
                });
            },
            Rvalue::Capture { .. } => {
                return Err(Trap::Unsupported {
                    what: "a capture",
                    span: stmt.span,
                });
            },
        };

        match place {
            Place::Value(value) => frame.values[value.index()] = Some(word),
            Place::Local(local) => frame.locals[local.index()] = Some(word),
        }

        Ok(())
    }

    /// Runs one operator over the words its operands give.
    fn prim(&self, op: PrimOp, args: &[Value], span: Span) -> Result<Value, Trap> {
        use PrimOp::*;

        match op {
            IntAdd | IntSub | IntMul | IntDiv | IntEq | IntNe | IntLt | IntLe | IntGt | IntGe => {
                let [left, right] = args else {
                    return Err(Trap::Unsupported {
                        what: "a binary operator with not two operands",
                        span,
                    });
                };

                let left = immediate(left, span)?;
                let right = immediate(right, span)?;

                match op {
                    IntAdd => Ok(Value::Int(wrap(left.wrapping_add(right)))),
                    IntSub => Ok(Value::Int(wrap(left.wrapping_sub(right)))),
                    IntMul => Ok(Value::Int(wrap(left.wrapping_mul(right)))),
                    IntDiv if right == 0 => Err(Trap::DivideByZero { span }),
                    IntDiv => Ok(Value::Int(wrap(left / right))),
                    IntEq => Ok(Value::Bool(left == right)),
                    IntNe => Ok(Value::Bool(left != right)),
                    IntLt => Ok(Value::Bool(left < right)),
                    IntLe => Ok(Value::Bool(left <= right)),
                    IntGt => Ok(Value::Bool(left > right)),
                    IntGe => Ok(Value::Bool(left >= right)),
                    _ => unreachable!("the arm above fixes the operators"),
                }
            },
            IntNeg | BoolNot => {
                let [operand] = args else {
                    return Err(Trap::Unsupported {
                        what: "a unary operator with not one operand",
                        span,
                    });
                };

                let operand = immediate(operand, span)?;

                if op == IntNeg {
                    Ok(Value::Int(wrap(operand.wrapping_neg())))
                } else {
                    Ok(Value::Bool(operand == 0))
                }
            },
            BoolAnd | BoolOr | BoolEq | BoolNe => {
                let [left, right] = args else {
                    return Err(Trap::Unsupported {
                        what: "a binary operator with not two operands",
                        span,
                    });
                };

                let left = immediate(left, span)? != 0;
                let right = immediate(right, span)? != 0;

                match op {
                    BoolAnd => Ok(Value::Bool(left && right)),
                    BoolOr => Ok(Value::Bool(left || right)),
                    BoolEq => Ok(Value::Bool(left == right)),
                    BoolNe => Ok(Value::Bool(left != right)),
                    _ => unreachable!("the arm above fixes the operators"),
                }
            },
            RefEq => {
                let [left, right] = args else {
                    return Err(Trap::Unsupported {
                        what: "a binary operator with not two operands",
                        span,
                    });
                };

                // Two immediates are the same word when their payloads are: `ref.eq` of the WASM
                // back end compares references, and an immediate is its payload.
                let left = immediate(left, span)?;
                let right = immediate(right, span)?;

                Ok(Value::Bool(left == right))
            },
        }
    }

    /// Calls what a callee says, and gives back the word it returns.
    fn callee(&mut self, callee: &Callee, args: &[Value], span: Span) -> Result<Value, Trap> {
        match callee {
            Callee::Entity(function) => self.call(function, args),
            Callee::Local(_) => {
                Err(Trap::Unsupported {
                    what: "a call to a function declared inside a body",
                    span,
                })
            },
            Callee::Indirect(_) => {
                Err(Trap::Unsupported {
                    what: "an indirect call",
                    span,
                })
            },
        }
    }

    /// Emits an edge: every argument first, then the parameters of the target.
    fn edge(
        &mut self,
        body: &Body,
        frame: &mut Frame,
        target: &BlockTarget,
        span: Span,
    ) -> Result<(), Trap> {
        let mut args = Vec::with_capacity(target.args.len());

        for argument in &target.args {
            args.push(self.operand(frame, argument, span)?);
        }

        let params = &body.blocks[target.block].params;

        assert_eq!(
            params.len(),
            args.len(),
            "an edge passes one word per parameter of its target",
        );

        // The arguments were evaluated before this loop, so an edge that permutes the parameters
        // of its target reads every argument before it writes any of them, as the back end does.
        for (param, argument) in params.iter().zip(args) {
            frame.values[param.index()] = Some(argument);
        }

        Ok(())
    }

    /// Reads an operand as the word it is.
    fn operand(&self, frame: &Frame, operand: &Operand, span: Span) -> Result<Value, Trap> {
        match operand {
            Operand::Value(value) => {
                frame.values[value.index()]
                    .clone()
                    .ok_or(Trap::Uninitialized { span })
            },
            Operand::Local(local) => {
                frame.locals[local.index()]
                    .clone()
                    .ok_or(Trap::Uninitialized { span })
            },
            Operand::Const(constant) => constant_value(constant, span),
        }
    }

    /// Reads every operand of a call or an operator, in the order they are passed.
    fn operands(
        &self,
        frame: &Frame,
        operands: &[Operand],
        span: Span,
    ) -> Result<Vec<Value>, Trap> {
        operands
            .iter()
            .map(|operand| self.operand(frame, operand, span))
            .collect()
    }
}

/// The word a constant is.
fn constant_value(constant: &Const, span: Span) -> Result<Value, Trap> {
    match constant {
        Const::Int(value) => Ok(Value::Int(*value)),
        Const::Bool(value) => Ok(Value::Bool(*value)),
        Const::Unit => Ok(Value::Unit),
        Const::Str(_) => {
            Err(Trap::Unsupported {
                what: "a string constant",
                span,
            })
        },
    }
}

/// The payload a constant compares by, the way `i31.get_s` reads it.
fn constant_payload(constant: &Const, span: Span) -> Result<i32, Trap> {
    match constant {
        Const::Int(value) => Ok(*value),
        Const::Bool(value) => Ok(i32::from(*value)),
        Const::Unit => Ok(0),
        Const::Str(_) => {
            Err(Trap::Unsupported {
                what: "a string constant",
                span,
            })
        },
    }
}

/// Reads a word as the payload of the immediate it has to be.
fn immediate(value: &Value, span: Span) -> Result<i32, Trap> {
    value.immediate().ok_or(Trap::Unsupported {
        what: "a reference where an immediate belongs",
        span,
    })
}

/// The payload an `i31` holds: the low 31 bits, sign-extended.
///
/// The interpreter computes the way the back end does: in `i32`, narrowed to the range of the
/// immediate by the same truncation `ref.i31` performs ([ADR-0018][adr-0018]).
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
fn wrap(value: i32) -> i32 {
    (value << 1) >> 1
}
