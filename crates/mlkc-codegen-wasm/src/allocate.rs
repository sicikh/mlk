//! Where the values of a body live, and which of them are emitted where they are read
//! ([ADR-0022]).
//!
//! A value that is read once, in the block that defines it, is not given a local: the
//! instruction that defines it is emitted where it is used, which is one instruction on the
//! stack where a `local.get` and a `local.set` would have been. A statement between the
//! definition and the use may only be passed when it cannot be observed --- a call, a division
//! that may trap, or a cast that may trap pins the value to a local, because computing it after
//! such a statement would be observable.
//!
//! Everything else keeps a local. A value read twice or more has to live somewhere; a value
//! read outside the block that defines it cannot be computed where it is read, because the
//! dispatch form has no fall-through between blocks; and a parameter of the body or of a block
//! is defined by the ABI or by an edge rather than by an instruction.
//!
//! The pass also decides the locals no value lives in: the program counter of a body that is
//! dispatched, and the scratch local a `switch` compares its scrutinee in.
//!
//! The decision the pass makes is what Waffle's `localify` makes over its own IR --- which
//! values an instruction reads where they are defined, and which need storage
//! (<https://github.com/bytecodealliance/waffle>, Apache-2.0 WITH LLVM-exception, which the
//! Apache-2.0 half of this crate's licence is compatible with); no code is taken from it.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use mlkc_lir_wasm::{BlockId, Body, Locals, Terminator, Ty, ValueId};

use crate::{collapse::is_observed, emit::is_direct};

/// One read of a value: the block it is in, and where in the block it stands.
///
/// A position that is the number of instructions of the block is the terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Use {
    block: BlockId,
    at: usize,
}

/// Fills the table of a body: which values live in a WASM local, and which are emitted where
/// they are read.
pub(crate) fn run(body: &mut Body) {
    let uses = uses(body);
    let definitions = body.definitions();
    let direct = is_direct(body);
    let parameters = body.params.len() as u32;
    let mut locals: Vec<Ty> = Vec::new();
    let mut allocation: Vec<Option<u32>> = vec![None; body.values.len()];

    for (value, data) in body.values.iter() {
        let local = if let Some(position) = body.params.iter().position(|param| *param == value) {
            // A parameter is the local the ABI declares for it.
            Some(position as u32)
        } else if is_block_param(body, value) {
            Some(fresh(&mut locals, data.ty, parameters))
        } else if inlines(body, &definitions, &uses, value) {
            None
        } else {
            Some(fresh(&mut locals, data.ty, parameters))
        };

        allocation[value.index()] = local;
    }

    let pc = (!direct).then(|| fresh(&mut locals, Ty::I32, parameters));
    let scratch = body
        .blocks
        .iter()
        .any(|(_, block)| matches!(block.term, Terminator::Switch { .. }))
        .then(|| fresh(&mut locals, Ty::I32, parameters));

    body.locals = Locals {
        locals,
        values: allocation,
        pc,
        scratch,
    };
}

/// Whether a value is a parameter of a block.
fn is_block_param(body: &Body, value: ValueId) -> bool {
    body.blocks
        .iter()
        .any(|(_, block)| block.params.contains(&value))
}

/// Whether the definition of a value may be emitted where it is read.
///
/// The rule: the value is read exactly once, in the block that defines it, and nothing
/// observable stands between the definition and the read.
fn inlines(
    body: &Body,
    definitions: &[Option<(BlockId, usize)>],
    uses: &[Vec<Use>],
    value: ValueId,
) -> bool {
    let Some((block, at)) = definitions[value.index()] else {
        return false;
    };
    let [used] = uses[value.index()].as_slice() else {
        return false;
    };

    if used.block != block {
        return false;
    }

    !body.blocks[block].insts[at + 1..used.at]
        .iter()
        .any(|inst| is_observed(&inst.op))
}

/// Every read of every value, by the value's place in its arena.
fn uses(body: &Body) -> Vec<Vec<Use>> {
    let mut uses = vec![Vec::new(); body.values.len()];

    for (id, block) in body.blocks.iter() {
        for (at, inst) in block.insts.iter().enumerate() {
            for operand in inst.op.operands() {
                uses[operand.index()].push(Use { block: id, at });
            }
        }

        let at = block.insts.len();

        for operand in block.term.operands() {
            uses[operand.index()].push(Use { block: id, at });
        }
    }

    uses
}

/// The local a value of this type is given, and the declaration of it.
fn fresh(locals: &mut Vec<Ty>, ty: Ty, parameters: u32) -> u32 {
    let local = parameters + locals.len() as u32;

    locals.push(ty);

    local
}

#[cfg(test)]
mod tests {
    use mlkc_lir_wasm::{
        Block, BlockTarget, Body, BodyBuilder, Inst, Op, Terminator, Ty, ValueData, ValueId,
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

    /// A body of one block: `insts`, and a return of `ret`.
    fn straight(mut builder: BodyBuilder, ret: ValueId, insts: Vec<Inst>) -> Body {
        let entry = builder.block(Block {
            params: Vec::new(),
            insts,
            term: Terminator::Return {
                value: ret,
                span: Span::dummy(),
            },
        });

        builder.finish(entry)
    }

    #[test]
    fn a_value_read_once_in_its_block_is_emitted_where_it_is_read() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let param = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let number = value(&mut builder, Ty::I32);
        let one = value(&mut builder, Ty::I32);
        let sum = value(&mut builder, Ty::I32);
        let result = value(&mut builder, Ty::I31);
        let body = straight(builder, result, vec![
            Inst {
                value: number,
                op: Op::I31GetS(param),
                span: Span::dummy(),
            },
            Inst {
                value: one,
                op: Op::I32Const(1),
                span: Span::dummy(),
            },
            Inst {
                value: sum,
                op: Op::I32Add(number, one),
                span: Span::dummy(),
            },
            Inst {
                value: result,
                op: Op::RefI31(sum),
                span: Span::dummy(),
            },
        ]);
        let mut body = body;

        run(&mut body);

        assert_eq!(body.locals.values[param.index()], Some(0));
        assert!(
            body.locals.values[1..].iter().all(Option::is_none),
            "every value read once to live nowhere: {:?}",
            body.locals.values,
        );
        assert!(body.locals.locals.is_empty());
        assert_eq!(body.locals.pc, None);
        assert_eq!(body.locals.scratch, None);
    }

    #[test]
    fn a_value_read_twice_lives_in_a_local() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let number = value(&mut builder, Ty::I32);
        let sum = value(&mut builder, Ty::I32);
        let result = value(&mut builder, Ty::I31);
        let mut body = straight(builder, result, vec![
            Inst {
                value: number,
                op: Op::I32Const(2),
                span: Span::dummy(),
            },
            Inst {
                value: sum,
                op: Op::I32Add(number, number),
                span: Span::dummy(),
            },
            Inst {
                value: result,
                op: Op::RefI31(sum),
                span: Span::dummy(),
            },
        ]);

        run(&mut body);

        // A value read twice has to live somewhere, and an `i32` lives in an `i32` local.
        assert_eq!(body.locals.values[number.index()], Some(0));
        assert_eq!(body.locals.locals, [Ty::I32]);
        assert_eq!(body.locals.values[sum.index()], None);
        assert_eq!(body.locals.values[result.index()], None);
    }

    #[test]
    fn a_value_read_in_another_block_lives_in_a_local() {
        let mut builder = BodyBuilder::new(Ty::I32);
        let number = value(&mut builder, Ty::I32);
        let tested = value(&mut builder, Ty::I32);
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: number,
                op: Op::I32Const(7),
                span: Span::dummy(),
            }],
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let other = builder.block(Block {
            params: Vec::new(),
            insts: vec![Inst {
                value: tested,
                op: Op::I32Eqz(number),
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: tested,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Goto {
            target: BlockTarget {
                block: other,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };

        let mut body = builder.finish(entry);

        run(&mut body);

        // A value read in a block that is not the one that defines it cannot be computed where
        // it is read: the dispatch form has no fall-through between blocks. The body is
        // dispatched, so it keeps a program counter as well.
        assert_eq!(body.locals.values[number.index()], Some(0));
        assert_eq!(body.locals.values[tested.index()], None);
        assert_eq!(body.locals.locals, [Ty::I32, Ty::I32]);
        assert_eq!(body.locals.pc, Some(1));
        assert_eq!(body.locals.scratch, None);
    }

    #[test]
    fn a_switch_leaves_a_scratch_local() {
        let mut builder = BodyBuilder::new(Ty::I31);
        let scrutinee = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I32,
        });
        let value_ = builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::I31,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let one = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: value_,
                span: Span::dummy(),
            },
        });
        let otherwise = builder.block(Block {
            params: Vec::new(),
            insts: Vec::new(),
            term: Terminator::Return {
                value: value_,
                span: Span::dummy(),
            },
        });

        builder.block_mut(entry).term = Terminator::Switch {
            scrutinee,
            arms: vec![(1, BlockTarget {
                block: one,
                args: Vec::new(),
            })],
            otherwise: BlockTarget {
                block: otherwise,
                args: Vec::new(),
            },
            span: Span::dummy(),
        };

        let mut body = builder.finish(entry);

        run(&mut body);

        assert_eq!(body.locals.pc, Some(2));
        assert_eq!(body.locals.scratch, Some(3));
        assert_eq!(body.locals.locals, [Ty::I32, Ty::I32]);
    }
}
