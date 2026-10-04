//! A reading of MIR, for a person and for a diff.
//!
//! The dump is one line per statement and one line per block, and it names every id the way
//! the verifier does: a block as `b0`, a value as `v0`, and a slot as `l0`. It is the reading
//! the tests snapshot ([ADR-0006][adr-0006]) and the reading a person debugs with.
//!
//! [adr-0006]: ../../docs/adr/0006-snapshot-testing.md

use std::fmt::Write as _;

use mlkc_hir_def::ItemLocLike;

use crate::{
    BlockId, BlockTarget, Body, Callee, CaptureData, CodeRef, Const, LocalId, Operand, Place,
    Rvalue, Stmt, StmtKind, Terminator, ValueId,
};

/// Reads a body as a person reads it: the code of the body, and the code of every lambda it
/// wrote.
pub fn body(body: &Body) -> String {
    let mut out = String::new();
    let mut header = format!("fun {}", owner_text(body));

    if let Some(local) = body.local {
        let _ = write!(header, " ({local:?})");
    }

    code_text(&mut out, body.code(), &header);

    for (id, lambda) in body.lambdas.iter() {
        let captures = labels(lambda.captures.iter().map(capture_text));
        let header = format!(
            "lambda #{}: {} captures ({captures})",
            id.index(),
            lambda.ty
        );

        let _ = writeln!(out);

        code_text(&mut out, lambda.code(), &header);
    }

    out
}

/// Reads one piece of code --- a body's or a lambda's --- under its header line.
fn code_text(out: &mut String, code: CodeRef<'_>, header: &str) {
    let _ = writeln!(out, "{header} (entry {})", block_label(code.entry));

    if !code.params.is_empty() {
        let params = labels(
            code.params
                .iter()
                .map(|value| format!("{}: {}", value_label(*value), code.values[*value].ty)),
        );

        let _ = writeln!(out, "  params: {params}");
    }

    for (id, block) in code.blocks.iter() {
        let _ = write!(out, "  {}", block_label(id));

        if !block.params.is_empty() {
            let _ = write!(
                out,
                "({})",
                labels(block.params.iter().copied().map(value_label)),
            );
        }

        let _ = writeln!(out, ":");

        for stmt in &block.stmts {
            let _ = writeln!(out, "    {}", stmt_text(code, stmt));
        }

        let _ = writeln!(out, "    {}", terminator_text(&block.term));
    }
}

/// The name of the owner of a body.
fn owner_text(body: &Body) -> String {
    body.owner
        .item
        .name()
        .map_or_else(|| format!("{:?}", body.owner.item), ToString::to_string)
}

/// One capture of a lambda, as it is read.
fn capture_text(capture: &CaptureData) -> String {
    match &capture.name {
        Some(name) => format!("{name}: {}", capture.ty),
        None => capture.ty.to_string(),
    }
}

/// What an assignment writes.
fn place_text(code: CodeRef<'_>, place: &Place) -> String {
    match place {
        // A slot is named where it is written: a read of it is a read of the slot.
        Place::Local(local) => {
            match &code.locals[*local].name {
                Some(name) => format!("{}({name})", local_label(*local)),
                None => local_label(*local),
            }
        },
        Place::Value(value) => value_label(*value),
    }
}

/// What an operand reads.
fn operand_text(operand: &Operand) -> String {
    match operand {
        Operand::Value(value) => value_label(*value),
        Operand::Local(local) => local_label(*local),
        Operand::Const(constant) => const_text(constant),
    }
}

/// What a statement writes and computes, as a line.
pub fn stmt_text(code: CodeRef<'_>, stmt: &Stmt) -> String {
    let StmtKind::Assign { place, rvalue } = &stmt.kind;

    format!("{} = {}", place_text(code, place), rvalue_text(rvalue))
}

/// What a statement computes.
fn rvalue_text(rvalue: &Rvalue) -> String {
    match rvalue {
        Rvalue::Use(operand) => format!("use {}", operand_text(operand)),
        Rvalue::Const(constant) => format!("const {}", const_text(constant)),
        Rvalue::Call { callee, args } => {
            format!("call {}({})", callee_text(callee), arguments_text(args),)
        },
        Rvalue::Closure { lambda, captures } => {
            format!(
                "closure lambda#{} ({})",
                lambda.index(),
                arguments_text(captures)
            )
        },
        Rvalue::Capture { index } => format!("capture #{index}"),
        Rvalue::Prim { op, args } => {
            format!("prim {}({})", op.as_str(), arguments_text(args))
        },
    }
}

/// What a call calls.
fn callee_text(callee: &Callee) -> String {
    match callee {
        Callee::Entity(entity) => {
            match entity.item.name() {
                Some(name) => format!("fun {name}"),
                None => format!("fun {:?}", entity.item),
            }
        },
        Callee::Local(local) => format!("{local:?}"),
        Callee::Indirect(operand) => operand_text(operand),
    }
}

/// Where a block ends, as a line.
pub fn terminator_text(term: &Terminator) -> String {
    match term {
        Terminator::Goto { target, .. } => format!("goto {}", target_text(target)),
        Terminator::Branch {
            cond, then_, else_, ..
        } => {
            format!(
                "branch {} -> {}, {}",
                operand_text(cond),
                target_text(then_),
                target_text(else_),
            )
        },
        Terminator::Switch {
            scrutinee,
            arms,
            otherwise,
            ..
        } => {
            let mut out = format!("switch {} [", operand_text(scrutinee));

            for (index, (constant, target)) in arms.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }

                let _ = write!(out, "{} -> {}", const_text(constant), target_text(target));
            }

            let _ = write!(out, "] else {}", target_text(otherwise));

            out
        },
        Terminator::Return { value, .. } => format!("return {}", operand_text(value)),
        Terminator::Unreachable { .. } => "unreachable".to_owned(),
    }
}

/// A block an edge goes to, and the arguments it passes.
fn target_text(target: &BlockTarget) -> String {
    if target.args.is_empty() {
        return block_label(target.block);
    }

    format!(
        "{}({})",
        block_label(target.block),
        arguments_text(&target.args),
    )
}

/// The arguments of a call or of an edge.
fn arguments_text(args: &[Operand]) -> String {
    labels(args.iter().map(operand_text))
}

/// A constant as it is read.
fn const_text(constant: &Const) -> String {
    match constant {
        Const::Int(value) => value.to_string(),
        Const::Bool(value) => value.to_string(),
        Const::Unit => "unit".to_owned(),
        Const::Str(value) => format!("{value:?}"),
    }
}

/// The label of a block.
pub fn block_label(block: BlockId) -> String {
    format!("b{}", block.index())
}

/// The label of a value.
pub fn value_label(value: ValueId) -> String {
    format!("v{}", value.index())
}

/// The label of a slot.
pub fn local_label(local: LocalId) -> String {
    format!("l{}", local.index())
}

/// A list of labels, separated by commas.
fn labels(labels: impl Iterator<Item = String>) -> String {
    labels.collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use mlkc_hir_ty::Ty;
    use mlkc_span::Span;

    use super::body;
    use crate::{
        Block, BlockTarget, BodyBuilder, Const, LocalData, Operand, Place, PrimOp, Rvalue, Stmt,
        StmtKind, Terminator, ValueData,
    };

    #[test]
    fn a_body_reads_as_its_blocks() {
        let mut builder = BodyBuilder::new(crate::test_support::owner());
        let value = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        });
        let join = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        });
        let slot = builder.local(LocalData {
            span: Span::dummy(),
            name: Some(mlkc_hir_def::Name::new("answer")),
            ty: Ty::Error,
        });
        let negated = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let done = builder.block(Block {
            params: vec![join],
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Value(negated),
                    rvalue: Rvalue::Prim {
                        op: PrimOp::IntNeg,
                        args: vec![Operand::Value(value)],
                    },
                },
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: Operand::Value(negated),
                span: Span::dummy(),
            },
        });

        *builder.block_mut(entry) = Block {
            params: Vec::new(),
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Local(slot),
                    rvalue: Rvalue::Const(Const::Int(1)),
                },
                span: Span::dummy(),
            }],
            term: Terminator::Goto {
                target: BlockTarget {
                    block: done,
                    args: vec![Operand::Local(slot)],
                },
                span: Span::dummy(),
            },
        };

        let mir = builder.finish(entry);

        assert_eq!(
            body(&mir),
            "fun main (entry b0)\n  params: v0: {error}, v1: {error}\n  b0:\n    l0(answer) = const 1\n    goto b1(l0)\n  b1(v1):\n    v2 = prim int-neg(v0)\n    return v2\n",
        );
    }
}
