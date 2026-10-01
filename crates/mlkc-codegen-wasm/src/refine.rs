//! What a word refines to: the kind the emitter knows a value has, when it knows one.
//!
//! A word is one uniform value ([ADR-0018][adr-0018]), and a MIR body says what an expression
//! *means* and not what it is. The emitter recovers a kind by a forward dataflow over the SSA
//! body: a parameter whose signature says `Int` is an immediate, the result of `int-add` is an
//! immediate, the result of a call is what the callee gives back, and where two paths meet the
//! refinement is the least upper bound of theirs.
//!
//! The refinement is a lattice ordered by precision:
//!
//! ```text
//! Word              -- any word; no knowledge at all
//! Int | Bool | Unit -- known to be an immediate
//! Never             -- unreachable; the refinement of a value with no definition
//! ```
//!
//! `Int ⊔ Bool` is `Word`, because a WASM local is one type on every path; `Never ⊔ x` is `x`,
//! because an edge that never runs does not constrain what the join holds. `String` and the
//! structures join in as their constructs are lowered: until a module can name their GC types,
//! everything that is not an immediate is a word.
//!
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md

use mlkc_hir_ty::{Builtins, Ty};
use mlkc_mir::{
    BlockTarget, Body, Callee, Const, Operand, PrimOp, Rvalue, StmtKind, Terminator, ValueId,
};

use crate::module::ModuleLayout;

/// What the emitter knows a word to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refinement {
    /// No definition reaches the value, or its block is not reachable; nothing is known.
    Never,
    /// A signed 31-bit integer: an `i31ref`.
    Int,
    /// A boolean: an `i31ref` of `0` or `1`.
    Bool,
    /// The unit value: an `i31ref` of `0`.
    Unit,
    /// A word, and nothing more than that: an `eqref`.
    Word,
}

impl Refinement {
    /// Whether the refinement is of an immediate: a value represented by an `i31ref`.
    pub fn is_immediate(self) -> bool {
        matches!(self, Self::Int | Self::Bool | Self::Unit)
    }

    /// The least upper bound of two refinements: the most precise kind both are known to be.
    ///
    /// Two immediate kinds of different meaning have no kind in common but a word, because a
    /// WASM local has one type on every path; so the join of `Int` and `Bool` is `Word`.
    /// A value nothing defines constrains no join, so `Never` joined with anything is it.
    pub fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Never, other) => other,
            (this, Self::Never) => this,
            (this, other) if this == other => this,
            _ => Self::Word,
        }
    }
}

/// The refinement of a type the checker resolved.
///
/// A class that is one of the classes of the language ([`Builtins`]) refines to its
/// representation; every other type is a word until the constructs that give it a shape are
/// lowered.
pub fn of_ty(ty: &Ty, builtins: &Builtins) -> Refinement {
    let Ty::Class { class, args } = ty else {
        return Refinement::Word;
    };

    if !args.is_empty() {
        return Refinement::Word;
    }

    if class == builtins.int() {
        Refinement::Int
    } else if class == builtins.boolean() {
        Refinement::Bool
    } else if class == builtins.unit() {
        Refinement::Unit
    } else {
        Refinement::Word
    }
}

/// The refinement of a constant.
pub fn of_const(constant: &Const) -> Refinement {
    match constant {
        Const::Int(_) => Refinement::Int,
        Const::Bool(_) => Refinement::Bool,
        Const::Unit => Refinement::Unit,
        Const::Str(_) => Refinement::Word,
    }
}

/// The refinement of what a primitive computes.
///
/// A primitive is named by its semantics and not by an instruction ([ADR-0019][adr-0019]), so
/// its operator fixes the kind of its result: arithmetic gives an immediate, and a comparison
/// gives a boolean.
///
/// [adr-0019]: ../../docs/adr/0019-mir.md
pub fn of_prim(op: PrimOp) -> Refinement {
    match op {
        PrimOp::IntAdd | PrimOp::IntSub | PrimOp::IntMul | PrimOp::IntDiv | PrimOp::IntNeg => {
            Refinement::Int
        },
        PrimOp::IntEq
        | PrimOp::IntNe
        | PrimOp::IntLt
        | PrimOp::IntLe
        | PrimOp::IntGt
        | PrimOp::IntGe
        | PrimOp::BoolAnd
        | PrimOp::BoolOr
        | PrimOp::BoolNot
        | PrimOp::BoolEq
        | PrimOp::BoolNe
        | PrimOp::RefEq => Refinement::Bool,
    }
}

/// The refinement of every value of one body, to a fixpoint.
#[derive(Debug, Clone)]
pub(crate) struct Refinements {
    values: Vec<Refinement>,
}

impl Refinements {
    /// Computes the refinement of every value of `body`.
    ///
    /// A value a statement defines refines to what its right-hand side computes, which is
    /// tightened as the operands it reads are; a block parameter refines to the join of the
    /// arguments its edges pass, and an entry parameter starts at what its signature says,
    /// since no edge defines it.
    pub(crate) fn of(body: &Body, builtins: &Builtins, layout: &ModuleLayout) -> Self {
        let mut values = vec![Refinement::Never; body.values.len()];

        for param in &body.params {
            values[param.index()] = of_ty(&body.values[*param].ty, builtins);
        }

        loop {
            let mut changed = false;

            for (_, block) in body.blocks.iter() {
                for stmt in &block.stmts {
                    let StmtKind::Assign {
                        place: mlkc_mir::Place::Value(value),
                        rvalue,
                    } = &stmt.kind
                    else {
                        continue;
                    };

                    let refined = rvalue_refinement(rvalue, &values, layout);

                    if values[value.index()] != refined {
                        values[value.index()] = refined;
                        changed = true;
                    }
                }

                for (target, _) in edges(&block.term) {
                    let destination = &body.blocks[target.block];

                    for (param, argument) in destination.params.iter().zip(&target.args) {
                        let argument = operand_refinement(argument, &values);
                        let joined = values[param.index()].join(argument);

                        if values[param.index()] != joined {
                            values[param.index()] = joined;
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                return Self { values };
            }
        }
    }

    /// The refinement of a value.
    pub(crate) fn get(&self, value: ValueId) -> Refinement {
        self.values[value.index()]
    }

    /// The refinement of an operand.
    pub(crate) fn operand(&self, operand: &Operand) -> Refinement {
        match operand {
            Operand::Value(value) => self.get(*value),
            Operand::Local(_) => Refinement::Word,
            Operand::Const(constant) => of_const(constant),
        }
    }
}

/// The refinement a right-hand side computes.
fn rvalue_refinement(rvalue: &Rvalue, values: &[Refinement], layout: &ModuleLayout) -> Refinement {
    match rvalue {
        Rvalue::Use(operand) => operand_refinement(operand, values),
        Rvalue::Const(constant) => of_const(constant),
        Rvalue::Prim { op, .. } => of_prim(*op),
        Rvalue::Call { callee, .. } => {
            match callee {
                Callee::Entity(entity) => {
                    layout.signature_of(entity).map_or(Refinement::Word, |sig| {
                        // A call gives back an `eqref`; what it refines to is the type of the result,
                        // which only the shape of a class can make narrower than a word.
                        of_ty(&sig.ret, layout.builtins())
                    })
                },
                Callee::Local(_) | Callee::Indirect(_) => Refinement::Word,
            }
        },
    }
}

/// The refinement of an operand.
fn operand_refinement(operand: &Operand, values: &[Refinement]) -> Refinement {
    match operand {
        Operand::Value(value) => values[value.index()],
        Operand::Local(_) => Refinement::Word,
        Operand::Const(constant) => of_const(constant),
    }
}

/// The edges a terminator leaves by, and the operand the edge is decided with.
fn edges(term: &Terminator) -> Vec<(&BlockTarget, Option<&Operand>)> {
    match term {
        Terminator::Goto { target, .. } => vec![(target, None)],
        Terminator::Branch {
            cond, then_, else_, ..
        } => vec![(then_, Some(cond)), (else_, Some(cond))],
        Terminator::Switch {
            scrutinee,
            arms,
            otherwise,
            ..
        } => {
            arms.iter()
                .map(|(_, target)| (target, Some(scrutinee)))
                .chain(std::iter::once((otherwise, Some(scrutinee))))
                .collect()
        },
        Terminator::Return { .. } | Terminator::Unreachable { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::Refinement;

    #[test]
    fn a_join_widens_to_the_kind_two_refinements_share() {
        assert_eq!(Refinement::Int.join(Refinement::Int), Refinement::Int);
        assert_eq!(Refinement::Int.join(Refinement::Bool), Refinement::Word);
        assert_eq!(Refinement::Bool.join(Refinement::Word), Refinement::Word);
    }

    #[test]
    fn never_is_the_join_identity() {
        assert_eq!(Refinement::Never.join(Refinement::Int), Refinement::Int);
        assert_eq!(Refinement::Bool.join(Refinement::Never), Refinement::Bool);
        assert_eq!(Refinement::Never.join(Refinement::Never), Refinement::Never);
    }
}
