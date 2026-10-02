//! What the emitter keeps in a local, and what it emits where the value is used.
//!
//! A value that is read once, in the block that defines it, is not given a local: the rvalue of
//! its definition is emitted where it is used, which is one value on the stack where a
//! `local.get` would have been. A statement between the definition and the use may only be
//! passed when it has no effect --- a call, or a division that may trap, pins the value to a
//! local, because computing it after such a statement would be observable.
//!
//! Everything else keeps a local. A value read twice or more has to live somewhere; a value
//! read outside the block that defines it cannot be computed where it is read, because the
//! dispatch form has no fall-through between blocks; and a parameter of the body or of a block
//! is defined by the ABI or by an edge rather than by a statement.

use mlkc_mir::{
    Block, BlockTarget, Body, Callee, Operand, Place, PrimOp, Rvalue, Stmt, StmtKind, Terminator,
    ValueId,
};

/// The values the emitter inlines, by the value's place in its arena.
pub(crate) struct Inlining<'a> {
    /// The statement that defines each inlined value; nothing for a value that keeps a local.
    definitions: Vec<Option<&'a Stmt>>,
}

impl<'a> Inlining<'a> {
    /// Plans the inlining of `body`.
    pub(crate) fn of(body: &'a Body) -> Self {
        let mut uses = vec![0u32; body.values.len()];
        let mut definitions: Vec<Option<&'a Stmt>> = vec![None; body.values.len()];

        for (_, block) in body.blocks.iter() {
            for stmt in &block.stmts {
                operands_of_stmt(stmt, &mut |operand| count(operand, &mut uses));
            }

            operands_of_terminator(&block.term, &mut |operand| {
                count(operand, &mut uses);
            });
        }

        for (_, block) in body.blocks.iter() {
            for (position, stmt) in block.stmts.iter().enumerate() {
                let StmtKind::Assign {
                    place: Place::Value(value),
                    ..
                } = &stmt.kind
                else {
                    continue;
                };

                // A value read more than once has to live somewhere, and a local is where.
                if uses[value.index()] != 1 {
                    continue;
                }

                if inlines(block, position, *value) {
                    definitions[value.index()] = Some(stmt);
                }
            }
        }

        Self { definitions }
    }

    /// Whether a value is emitted where it is used rather than kept in a local.
    pub(crate) fn is_inlined(&self, value: ValueId) -> bool {
        self.definitions[value.index()].is_some()
    }

    /// The statement an inlined value is defined by.
    pub(crate) fn definition(&self, value: ValueId) -> Option<&'a Stmt> {
        self.definitions[value.index()]
    }
}

/// Whether the value a statement at `position` defines may be emitted at its single use.
///
/// The use is looked for in the rest of the block and in its terminator: a value read outside
/// the block cannot be computed in it. A statement that is not the use is passed only when it
/// has no effect, since the definition would otherwise be computed after it.
fn inlines(block: &Block, position: usize, value: ValueId) -> bool {
    for stmt in &block.stmts[position + 1..] {
        let mut reads = false;

        operands_of_stmt(stmt, &mut |operand| reads |= reads_value(operand, value));

        if reads {
            return true;
        }

        if !is_pure(stmt) {
            return false;
        }
    }

    let mut reads = false;

    operands_of_terminator(&block.term, &mut |operand| {
        reads |= reads_value(operand, value);
    });

    reads
}

/// Whether a statement computes a value and nothing a later statement can observe.
///
/// The operators of the language are total except a division, which traps on a zero divisor,
/// and a call, which may print or trap. Everything else --- a constant, a read, an arithmetic
/// operator that wraps --- is what makes a value safe to compute later.
fn is_pure(stmt: &Stmt) -> bool {
    let StmtKind::Assign { rvalue, .. } = &stmt.kind;

    match rvalue {
        Rvalue::Use(_) | Rvalue::Const(_) => true,
        Rvalue::Prim { op, .. } => !matches!(op, PrimOp::IntDiv),
        Rvalue::Call { .. } => false,
    }
}

/// Counts one use of a value.
fn count(operand: &Operand, uses: &mut [u32]) {
    if let Operand::Value(value) = operand {
        uses[value.index()] += 1;
    }
}

/// Whether an operand is a read of `value`.
fn reads_value(operand: &Operand, value: ValueId) -> bool {
    matches!(operand, Operand::Value(it) if *it == value)
}

/// Calls `visit` with every operand a statement reads.
fn operands_of_stmt(stmt: &Stmt, visit: &mut impl FnMut(&Operand)) {
    let StmtKind::Assign { rvalue, .. } = &stmt.kind;

    match rvalue {
        Rvalue::Use(operand) => visit(operand),
        Rvalue::Const(_) => {},
        Rvalue::Prim { args, .. } => {
            for arg in args {
                visit(arg);
            }
        },
        Rvalue::Call { callee, args } => {
            if let Callee::Indirect(operand) = callee {
                visit(operand);
            }

            for arg in args {
                visit(arg);
            }
        },
    }
}

/// Calls `visit` with every operand a terminator reads.
fn operands_of_terminator(term: &Terminator, visit: &mut impl FnMut(&Operand)) {
    match term {
        Terminator::Goto { target, .. } => operands_of_target(target, visit),
        Terminator::Branch {
            cond, then_, else_, ..
        } => {
            visit(cond);
            operands_of_target(then_, visit);
            operands_of_target(else_, visit);
        },
        Terminator::Switch {
            scrutinee,
            arms,
            otherwise,
            ..
        } => {
            visit(scrutinee);

            for (_, target) in arms {
                operands_of_target(target, visit);
            }

            operands_of_target(otherwise, visit);
        },
        Terminator::Return { value, .. } => visit(value),
        Terminator::Unreachable { .. } => {},
    }
}

/// Calls `visit` with every operand an edge passes.
fn operands_of_target(target: &BlockTarget, visit: &mut impl FnMut(&Operand)) {
    for arg in &target.args {
        visit(arg);
    }
}
