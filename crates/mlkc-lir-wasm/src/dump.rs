//! A reading of the LIR, for a person and for a diff.
//!
//! The dump is one line per instruction and one line per block, and it names every id the way
//! the verifier does: a block as `b0` and a value as `v0`. A value that lives in a local is
//! marked with it, so that a reading says what encoding will write: `v1(local 3) = i32.const 1`
//! is a value the emitter stores, and `v1 = i32.const 1` is one it emits where it is read.
//!
//! It is the reading the tests snapshot ([ADR-0006][adr-0006]) and the reading a person debugs
//! with.
//!
//! [adr-0006]: ../../docs/adr/0006-snapshot-testing.md

use std::fmt::Write as _;

use crate::{
    BlockId, BlockTarget, Body, Inst, Node, Op, Structure, Terminator, ValueId,
    ty::{RefTy, Ty},
};

/// Reads a body as a person reads it.
pub fn body(body: &Body) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        "lir (entry {}) ret {}",
        block_label(body.entry),
        body.ret,
    );

    if !body.params.is_empty() {
        let params = labels(
            body.params
                .iter()
                .map(|value| format!("{}: {}", value_label(*value), body.values[*value].ty)),
        );

        let _ = writeln!(out, "  params: {params}");
    }

    if !body.locals.is_unallocated() {
        let _ = writeln!(out, "  {}", locals_text(body));
    }

    for (id, block) in body.blocks.iter() {
        let _ = write!(out, "  {}", block_label(id));

        if !block.params.is_empty() {
            let params =
                labels(block.params.iter().map(|param| {
                    format!("{}: {}", value_text(body, *param), body.values[*param].ty,)
                }));

            let _ = write!(out, "({params})");
        }

        let _ = writeln!(out, ":");

        for inst in &block.insts {
            let _ = writeln!(out, "    {}", inst_text(body, inst));
        }

        let _ = writeln!(out, "    {}", terminator_text(&block.term));
    }

    out
}

/// What one instruction defines, and what it computes, as a line.
pub fn inst_text(body: &Body, inst: &Inst) -> String {
    format!("{} = {}", value_text(body, inst.value), op_text(&inst.op))
}

/// The label of a value, with the local it lives in where it is not a parameter.
///
/// A parameter is the local the ABI declared for it, and the parameters are read from the
/// header; every other value that lives in a local says which one, so that a reading says what
/// encoding will write.
fn value_text(body: &Body, value: ValueId) -> String {
    let label = value_label(value);

    match body.locals.values.get(value.index()).copied().flatten() {
        Some(local) if local as usize >= body.params.len() => format!("{label}(local {local})"),
        _ => label,
    }
}

/// Reads the structured control flow of a body as a person reads it.
///
/// The dump is a nested list of frames, one per line, in the order encoding writes them: a
/// `block` or a `loop` holds what stands between it and its `end`, an `if` holds both arms, and
/// a `leaf` is where the instructions of one block are emitted.
pub fn structure(structure: &Structure) -> String {
    let mut out = String::new();

    nodes_text(&structure.nodes, 0, &mut out);

    out
}

/// Writes one list of nodes, indented by the frames around them.
fn nodes_text(nodes: &[Node], depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth + 1);

    for node in nodes {
        match node {
            Node::Block { out: block, body } => {
                let _ = writeln!(out, "{indent}block {}:", block_label(*block));
                nodes_text(body, depth + 1, out);
            },
            Node::Loop { header, body } => {
                let _ = writeln!(out, "{indent}loop {}:", block_label(*header));
                nodes_text(body, depth + 1, out);
            },
            Node::If {
                cond, then_, else_, ..
            } => {
                let _ = writeln!(out, "{indent}if {}:", value_label(*cond));
                nodes_text(then_, depth + 1, out);
                let _ = writeln!(out, "{indent}else:");
                nodes_text(else_, depth + 1, out);
            },
            Node::Leaf { block } => {
                let _ = writeln!(out, "{indent}leaf {}", block_label(*block));
            },
            Node::Params { target, args, .. } => {
                let _ = writeln!(
                    out,
                    "{indent}params {}({})",
                    block_label(*target),
                    labels(args.iter().copied().map(value_label)),
                );
            },
            Node::Br { depth, target, .. } => {
                let _ = writeln!(out, "{indent}br {depth} -> {}", block_label(*target),);
            },
            Node::Return { value, .. } => {
                let _ = writeln!(out, "{indent}return {}", value_label(*value));
            },
            Node::Unreachable { .. } => {
                let _ = writeln!(out, "{indent}unreachable");
            },
        }
    }
}

/// The locals of a body, as the header reads them.
fn locals_text(body: &Body) -> String {
    let parameters = body.params.len() as u32;
    let mut locals: Vec<String> = body
        .locals
        .locals
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{}: {ty}", parameters + index as u32))
        .collect();

    // The special locals are declared with the others; the header says which they are.
    for (named, at) in [("pc", body.locals.pc), ("scratch", body.locals.scratch)] {
        if let Some(at) = at
            && let Some(at) = at.checked_sub(parameters)
            && let Some(text) = locals.get_mut(at as usize)
        {
            let _ = write!(text, " ({named})");
        }
    }

    format!("locals: {}", locals.join(", "))
}

/// What an instruction computes.
pub fn op_text(op: &Op) -> String {
    use Op::*;

    match op {
        I32Const(value) => format!("i32.const {value}"),
        I32Eqz(operand) => format!("i32.eqz {}", value_label(*operand)),
        I32Eq(lhs, rhs) => binary("i32.eq", *lhs, *rhs),
        I32Ne(lhs, rhs) => binary("i32.ne", *lhs, *rhs),
        I32LtS(lhs, rhs) => binary("i32.lt_s", *lhs, *rhs),
        I32LeS(lhs, rhs) => binary("i32.le_s", *lhs, *rhs),
        I32GtS(lhs, rhs) => binary("i32.gt_s", *lhs, *rhs),
        I32GeS(lhs, rhs) => binary("i32.ge_s", *lhs, *rhs),
        I32Add(lhs, rhs) => binary("i32.add", *lhs, *rhs),
        I32Sub(lhs, rhs) => binary("i32.sub", *lhs, *rhs),
        I32Mul(lhs, rhs) => binary("i32.mul", *lhs, *rhs),
        I32DivS(lhs, rhs) => binary("i32.div_s", *lhs, *rhs),
        I32And(lhs, rhs) => binary("i32.and", *lhs, *rhs),
        I32Or(lhs, rhs) => binary("i32.or", *lhs, *rhs),
        RefI31(operand) => format!("ref.i31 {}", value_label(*operand)),
        I31GetS(operand) => format!("i31.get_s {}", value_label(*operand)),
        RefCast(ty, operand) => format!("ref.cast {} {}", ref_text(*ty), value_label(*operand)),
        RefEq(lhs, rhs) => binary("ref.eq", *lhs, *rhs),
        Call { function, args } => {
            format!("call ${function}({})", value_labels(args))
        },
        CallLocal { function, args } => {
            format!("call-local {function:?}({})", value_labels(args))
        },
        CallIndirect { callee, args } => {
            format!(
                "call-indirect {}({})",
                value_label(*callee),
                value_labels(args)
            )
        },
        String(value) => format!("str {value:?}"),
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
                value_label(*cond),
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
            let mut out = format!("switch {} [", value_label(*scrutinee));

            for (index, (value, target)) in arms.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }

                let _ = write!(out, "{value} -> {}", target_text(target));
            }

            let _ = write!(out, "] else {}", target_text(otherwise));

            out
        },
        Terminator::Return { value, .. } => format!("return {}", value_label(*value)),
        Terminator::Unreachable { .. } => "unreachable".to_owned(),
    }
}

/// A block an edge goes to, and the arguments it passes.
fn target_text(target: &BlockTarget) -> String {
    if target.args.is_empty() {
        return block_label(target.block);
    }

    let args = value_labels(&target.args);

    format!("{}({args})", block_label(target.block))
}

/// A binary instruction, as a line.
fn binary(op: &str, lhs: ValueId, rhs: ValueId) -> String {
    format!("{op} {}, {}", value_label(lhs), value_label(rhs))
}

/// The values of an instruction, separated by commas.
fn value_labels(values: &[ValueId]) -> String {
    labels(values.iter().copied().map(value_label))
}

/// The label of a block.
pub fn block_label(block: BlockId) -> String {
    format!("b{}", block.index())
}

/// The label of a value.
pub fn value_label(value: ValueId) -> String {
    format!("v{}", value.index())
}

/// A reference type, as the target writes it.
pub fn ref_text(ty: RefTy) -> String {
    Ty::Ref(ty).to_string()
}

/// A list of labels, separated by commas.
fn labels(labels: impl Iterator<Item = String>) -> String {
    labels.collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use mlkc_span::Span;

    use super::body;
    use crate::{Block, BodyBuilder, Inst, Op, RefTy, Terminator, Ty, ValueData};

    #[test]
    fn a_body_reads_as_its_blocks() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let value = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::EQREF,
        });
        let cast = builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: cast,
                op: Op::RefCast(RefTy::I31, value),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: cast,
                span: Span::dummy(),
            },
        });
        let built = builder.finish(entry);

        assert_eq!(
            body(&built),
            "lir (entry b0) ret (ref i31)\n  params: v0: eqref\n  b0:\n    v1 = ref.cast (ref i31) v0\n    return v1\n",
        );
    }
}
