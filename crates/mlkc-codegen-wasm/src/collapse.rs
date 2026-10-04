//! Collapsing what selection wrote where the target has nothing to do ([ADR-0022]).
//!
//! Selection emits what the types say, and what the types say is sometimes more than the
//! machine needs:
//!
//! - a box that is opened at once --- `i31.get_s` of `ref.i31` --- is the number inside it;
//! - a cast of a value that is already of the type it is cast to is nothing at all.
//!
//! The pass is a rewrite by definition and use, and not a second way of emitting: it replaces
//! what a value reads, and then drops the definitions nothing reads any more. Dropping is what
//! makes the rewrite complete: a box whose only reader is gone is not a value of the body.
//!
//! What is dropped is only what cannot be observed. An instruction that may trap or that calls
//! something is kept even where nothing reads it, and so is a string constant, which is a
//! construct the backend reports rather than emits: a body that mentions one is diagnosed
//! whether or not the value is read.
//!
//! The shape of the pass --- a rewrite by definition and use, and the removal of what is left
//! with no reader --- is the one Waffle's `reducify` has over its WASM-level SSA
//! (<https://github.com/bytecodealliance/waffle>, Apache-2.0 WITH LLVM-exception, which the
//! Apache-2.0 half of this crate's licence is compatible with); no code is taken from it.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_lir_wasm::{Block, Body, BodyBuilder, Inst, Op, RefTy, Ty, ValueData, ValueId};

/// Rewrites a body by what its values read, and drops what nothing reads any more.
pub(crate) fn run(body: &Body) -> Body {
    let mut substitutions = substitutions(body);

    resolve_all(&mut substitutions);

    let live = liveness(body, &substitutions);

    rebuild(body, &substitutions, &live)
}

/// The value every instruction is replaced by, where what it computes is something the target
/// already did.
fn substitutions(body: &Body) -> Vec<Option<ValueId>> {
    let definitions = body.definitions();
    let mut substitutions: Vec<Option<ValueId>> = vec![None; body.values.len()];

    for (_, block) in body.blocks.iter() {
        for inst in &block.insts {
            let substituted = match &inst.op {
                // The box of a number, opened at once: the number is what the value is.
                Op::I31GetS(operand) => {
                    match definitions[operand.index()]
                        .map(|(block, at)| (&body.blocks[block].insts[at].op, block))
                    {
                        Some((Op::RefI31(number), _)) => Some(*number),
                        _ => None,
                    }
                },
                // A cast of a value that is already what it is cast to: the value is what the
                // cast is.
                Op::RefCast(RefTy::I31, operand) if body.values[*operand].ty == Ty::I31 => {
                    Some(*operand)
                },
                _ => None,
            };

            if let Some(substituted) = substituted {
                substitutions[inst.value.index()] = Some(substituted);
            }
        }
    }

    substitutions
}

/// Resolves every substitution to a value that is not substituted itself.
fn resolve_all(substitutions: &mut [Option<ValueId>]) {
    for index in 0..substitutions.len() {
        let mut root = substitutions[index];

        while let Some(next) = root.and_then(|value| substitutions[value.index()]) {
            root = Some(next);
        }

        if let Some(root) = root {
            substitutions[index] = Some(root);
        }
    }
}

/// The values the body cannot do without: what it is entered with, what an edge passes, what a
/// terminator reads, what an instruction computes although nothing reads it, and what a
/// substitution reads.
fn liveness(body: &Body, substitutions: &[Option<ValueId>]) -> Vec<bool> {
    let definitions = body.definitions();
    let mut live = vec![false; body.values.len()];
    let mut queue: Vec<ValueId> = Vec::new();

    let mark = |value: ValueId, live: &mut Vec<bool>, queue: &mut Vec<ValueId>| {
        if !live[value.index()] {
            live[value.index()] = true;
            queue.push(value);
        }
    };

    for param in &body.params {
        mark(resolve(*param, substitutions), &mut live, &mut queue);
    }

    for (_, block) in body.blocks.iter() {
        for param in &block.params {
            mark(resolve(*param, substitutions), &mut live, &mut queue);
        }

        for operand in block.term.operands() {
            mark(resolve(operand, substitutions), &mut live, &mut queue);
        }
    }

    for substitution in substitutions.iter().flatten() {
        mark(*substitution, &mut live, &mut queue);
    }

    for (_, block) in body.blocks.iter() {
        for inst in &block.insts {
            if is_observed(&inst.op) && substitutions[inst.value.index()].is_none() {
                mark(inst.value, &mut live, &mut queue);
            }
        }
    }

    while let Some(value) = queue.pop() {
        let Some((block, at)) = definitions[value.index()] else {
            continue;
        };

        for operand in body.blocks[block].insts[at].op.operands() {
            mark(resolve(operand, substitutions), &mut live, &mut queue);
        }
    }

    live
}

/// Whether an instruction computes something that is observed even where no value reads it.
///
/// A call prints, writes, or traps; a division traps on a zero divisor; a cast traps on a value
/// of another type. Everything else the LIR computes is a pure value, and a pure value nothing
/// reads is a value the body does not need.
pub(crate) fn is_observed(op: &Op) -> bool {
    matches!(
        op,
        Op::I32DivS(..)
            | Op::RefCast(..)
            | Op::StructGet { .. }
            | Op::Call { .. }
            | Op::CallRef { .. }
            | Op::String(_),
    )
}

/// Builds the body over the values that are left: their definitions, rewritten by what they
/// read, and everything else dropped.
fn rebuild(body: &Body, substitutions: &[Option<ValueId>], live: &[bool]) -> Body {
    let mut builder = BodyBuilder::new(body.ret);
    let mut map: Vec<Option<ValueId>> = vec![None; body.values.len()];

    for param in &body.params {
        let data = &body.values[*param];

        map[param.index()] = Some(builder.param(ValueData {
            span: data.span,
            ty: data.ty,
        }));
    }

    for (value, data) in body.values.iter() {
        if body.params.contains(&value) || !live[value.index()] {
            continue;
        }

        map[value.index()] = Some(builder.value(ValueData {
            span: data.span,
            ty: data.ty,
        }));
    }

    let mut blocks = Vec::with_capacity(body.blocks.len());

    for (id, block) in body.blocks.iter() {
        let params = block
            .params
            .iter()
            .map(|param| map[param.index()].expect("a parameter of a block to be live"))
            .collect();
        let mut insts = Vec::with_capacity(block.insts.len());

        for inst in &block.insts {
            if !live[inst.value.index()] {
                continue;
            }

            let mut op = inst.op.clone();

            op.map_operands(|operand| remap(operand, substitutions, &map));

            insts.push(Inst {
                value: map[inst.value.index()].expect("a live value to be mapped"),
                op,
                span: inst.span,
            });
        }

        let mut term = block.term.clone();

        term.map_operands(|operand| remap(operand, substitutions, &map));

        let built = builder.block(Block {
            params,
            insts,
            term,
        });

        // The blocks are built in the order of the old arena, so an edge names the same block
        // it named; the assertion is what keeps that true.
        debug_assert_eq!(built.index(), id.index(), "a block to keep its place");

        blocks.push(built);
    }

    let entry = blocks[body.entry.index()];

    builder.finish(entry)
}

/// The value an operand reads, with the substitutions applied.
fn resolve(operand: ValueId, substitutions: &[Option<ValueId>]) -> ValueId {
    let mut value = operand;

    while let Some(next) = substitutions[value.index()] {
        value = next;
    }

    value
}

/// The value an operand reads, with the substitutions applied and the new naming.
fn remap(operand: ValueId, substitutions: &[Option<ValueId>], map: &[Option<ValueId>]) -> ValueId {
    let value = resolve(operand, substitutions);

    map[value.index()].expect("a value something live reads to be live")
}

#[cfg(test)]
mod tests {
    use mlkc_lir_wasm::{
        Block, Body, BodyBuilder, Inst, Op, RefTy, Terminator, Ty, ValueData, ValueId,
    };
    use mlkc_span::Span;

    use super::run;

    /// A value of `ty` in the arena of `builder`.
    fn value(builder: &mut BodyBuilder, ty: Ty) -> ValueId {
        builder.value(ValueData {
            span: Span::dummy(),
            ty,
        })
    }

    /// A body of one block: `inst`, and a return of `ret`.
    fn body_of(mut builder: BodyBuilder, ret: ValueId, inst: Inst) -> Body {
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![inst],
            term: Terminator::Return {
                value: ret,
                span: Span::dummy(),
            },
        });

        builder.finish(entry)
    }

    #[test]
    fn a_box_that_is_opened_at_once_is_the_number_inside() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let number = value(&mut builder, Ty::I32);
        let boxed = value(&mut builder, Ty::I31);
        let opened = value(&mut builder, Ty::I32);
        let again = value(&mut builder, Ty::I31);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![
                Inst {
                    value: number,
                    op: Op::I32Const(2),
                    span: Span::dummy(),
                },
                Inst {
                    value: boxed,
                    op: Op::RefI31(number),
                    span: Span::dummy(),
                },
                Inst {
                    value: opened,
                    op: Op::I31GetS(boxed),
                    span: Span::dummy(),
                },
                Inst {
                    value: again,
                    op: Op::RefI31(opened),
                    span: Span::dummy(),
                },
            ],
            term: Terminator::Return {
                value: again,
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);
        let collapsed = run(&body);

        // The box and the unbox that followed it read the number the first one closed: what is
        // left is the constant and the box the body gives back.
        assert_eq!(
            mlkc_lir_wasm::dump::body(&collapsed),
            "lir (entry b0) ret (ref i31)\n  b0:\n    v0 = i32.const 2\n    v1 = ref.i31 v0\n    return v1\n",
        );
    }

    #[test]
    fn a_cast_of_what_a_value_already_is_is_dropped() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let cast = value(&mut builder, Ty::I31);
        let body = body_of(builder, param, Inst {
            value: cast,
            op: Op::RefCast(RefTy::I31, param),
            span: Span::dummy(),
        });
        let collapsed = run(&body);

        assert_eq!(
            mlkc_lir_wasm::dump::body(&collapsed),
            "lir (entry b0) ret (ref i31)\n  params: v0: (ref i31)\n  b0:\n    return v0\n",
        );
    }

    #[test]
    fn a_call_nothing_reads_is_kept() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let called = value(&mut builder, Ty::EQREF);
        let body = body_of(builder, param, Inst {
            value: called,
            op: Op::Call {
                function: 7,
                args: Vec::new(),
            },
            span: Span::dummy(),
        });
        let collapsed = run(&body);

        // A call is observed whatever reads it, so it stays and the value it defines lives.
        assert_eq!(
            mlkc_lir_wasm::dump::body(&collapsed),
            "lir (entry b0) ret (ref i31)\n  params: v0: (ref i31)\n  b0:\n    v1 = call $7()\n    return v0\n",
        );
    }

    #[test]
    fn a_pure_value_nothing_reads_is_dropped() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let constant = value(&mut builder, Ty::I32);
        let body = body_of(builder, param, Inst {
            value: constant,
            op: Op::I32Const(1),
            span: Span::dummy(),
        });
        let collapsed = run(&body);

        assert_eq!(
            mlkc_lir_wasm::dump::body(&collapsed),
            "lir (entry b0) ret (ref i31)\n  params: v0: (ref i31)\n  b0:\n    return v0\n",
        );
    }
}
