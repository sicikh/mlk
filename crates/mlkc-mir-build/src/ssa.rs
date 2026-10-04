//! The construction of the SSA form, in one walk of the graph.
//!
//! The construction is the on-the-fly one of Braun et al. ([ADR-0019][adr-0019]): the blocks
//! are walked in reverse postorder, a current definition of every slot is kept per block, and
//! a read that finds no definition is answered through the predecessors --- by the one
//! predecessor if there is one, and by a parameter of the block if there are several. A
//! parameter whose arguments all turn out to be one value is dropped, and its uses read that
//! value instead: that is where the copies the CFG form is made of disappear.
//!
//! A block is *sealed* once every predecessor of it has been walked: only then do the
//! arguments of its parameters have definitions to read. A read at a block that is not sealed
//! yet --- a read in a loop header, on the path of the back edge --- makes an incomplete
//! parameter, and the arguments are added when the block is sealed. The CFG is complete
//! before the walk, so sealing is not a question about a graph that is still growing: it is
//! exactly "every predecessor was walked".
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md

use mlkc_la_arena::Arena;
use mlkc_mir::{
    Block, BlockId, BlockTarget, Body, Callee, Code, CodeRef, LambdaData, LocalId, Operand, Place,
    Rvalue, Stmt, StmtKind, Terminator, ValueData, ValueId, cfg::Cfg,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Builds the SSA form of a body that is in the CFG form, and of every lambda it wrote.
///
/// # Panics
///
/// A body that does not hold the CFG invariant is a bug of the stage that built it, and the
/// construction is not a way to recover from one: the precondition is checked in debug builds.
pub fn construct_ssa(body: &Body) -> Body {
    debug_assert!(
        body.validate_cfg().is_ok(),
        "the SSA construction walks a body in the CFG form",
    );

    let mut lambdas = Arena::default();

    for (id, lambda) in body.lambdas.iter() {
        let _ = id;
        lambdas.alloc(construct_ssa_lambda(lambda));
    }

    let code = Ssa::new(body.code()).run();

    Body {
        owner: body.owner.clone(),
        local: body.local.clone(),
        params: code.params,
        entry: code.entry,
        blocks: code.blocks,
        values: code.values,
        locals: code.locals,
        lambdas,
        local_functions: body
            .local_functions
            .iter()
            .map(construct_ssa_local)
            .collect(),
    }
}

/// Builds the SSA form of one function declared inside a body.
///
/// The lambdas of the owner are the owner's, not the function's: a lambda written inside
/// a function declared in a `local` is an entry of the arena of the body that declares the
/// function, and the closure that creates it reads it by the same id.
fn construct_ssa_local(local: &Body) -> Body {
    let code = Ssa::new(local.code()).run();

    Body {
        owner: local.owner.clone(),
        local: local.local.clone(),
        params: code.params,
        entry: code.entry,
        blocks: code.blocks,
        values: code.values,
        locals: code.locals,
        lambdas: Arena::default(),
        local_functions: Vec::new(),
    }
}

/// Builds the SSA form of one lambda of a body.
fn construct_ssa_lambda(lambda: &LambdaData) -> LambdaData {
    let code = Ssa::new(lambda.code()).run();

    LambdaData {
        ty: lambda.ty.clone(),
        captures: lambda.captures.clone(),
        params: code.params,
        param_names: lambda.param_names.clone(),
        entry: code.entry,
        blocks: code.blocks,
        values: code.values,
        locals: code.locals,
    }
}

/// The construction of the SSA form of one body.
struct Ssa<'a> {
    /// The code the construction reads.
    source: CodeRef<'a>,
    /// The shape of the body's graph.
    cfg: Cfg,
    /// The id of every block, by position.
    ids: Vec<BlockId>,
    /// The blocks reached from the entry, in reverse postorder, and then the rest.
    order: Vec<usize>,
    /// Whether a block was walked.
    walked: Vec<bool>,
    /// Whether every predecessor of a block was walked: the arguments of its parameters may
    /// only be read then.
    sealed: Vec<bool>,
    /// The current definition of a slot, per block; a slot no definition of which was met is
    /// absent.
    current: Vec<FxHashMap<LocalId, ValueId>>,
    /// The parameters whose arguments are not known yet, by block and slot.
    incomplete: FxHashMap<(usize, LocalId), ValueId>,
    /// The reads the recursion is inside of, by block and slot: a read that comes back to one
    /// of them is a cycle of blocks with one predecessor each, and it gets a parameter of its
    /// own rather than recursing forever.
    pending: FxHashSet<(usize, LocalId)>,
    /// The parameters of every block, by position.
    params: Vec<Vec<ValueId>>,
    /// The arguments of every parameter, one per predecessor, in the order of the
    /// predecessors.
    phi_args: FxHashMap<ValueId, Vec<ValueId>>,
    /// The values that carried one value, by the value they carry: a parameter whose arguments
    /// all turned out to be one value, and a statement that only copies one. They are not
    /// definitions, and every use of one reads the value it carried.
    trivial: FxHashMap<ValueId, ValueId>,
    /// The values: the ones of the source, and the definition of every statement.
    values: Arena<ValueData>,
    /// The statements of every block, rewritten, by position.
    stmts: Vec<Vec<Stmt>>,
    /// The terminator of every block, rewritten, by position; the arguments of its targets are
    /// filled once every parameter is known.
    terms: Vec<Option<Terminator>>,
}

impl<'a> Ssa<'a> {
    /// The construction over `source`.
    fn new(source: CodeRef<'a>) -> Self {
        let ids: Vec<BlockId> = source.blocks.iter().map(|(id, _)| id).collect();
        let count = ids.len();
        let cfg = Cfg::of(source);

        let mut order = cfg.reverse_postorder().to_vec();
        let mut seen = vec![false; count];

        for &block in &order {
            seen[block] = true;
        }

        // A block no path from the entry reaches is walked after the ones that are, so that a
        // definition it reads through the graph is built where it stands.
        for (block, walked) in seen.iter().enumerate() {
            if !walked {
                order.push(block);
            }
        }

        // A block with no predecessor has nothing to wait for: the entry, and a block no path
        // reaches.
        let sealed = (0..count)
            .map(|block| cfg.predecessors(block).is_empty())
            .collect();

        Self {
            source,
            cfg,
            ids,
            order,
            walked: vec![false; count],
            sealed,
            current: (0..count).map(|_| FxHashMap::default()).collect(),
            incomplete: FxHashMap::default(),
            pending: FxHashSet::default(),
            params: vec![Vec::new(); count],
            phi_args: FxHashMap::default(),
            trivial: FxHashMap::default(),
            values: source.values.clone(),
            stmts: vec![Vec::new(); count],
            terms: (0..count).map(|_| None).collect(),
        }
    }

    /// Walks the graph and builds the form.
    fn run(mut self) -> Code {
        for block in 0..self.order.len() {
            let block = self.order[block];
            self.block(block);
        }

        self.arguments();

        self.finish()
    }

    /// Rewrites one block: its statements, the operands of its terminator, and the sealing of
    /// the successors whose predecessors are walked by now.
    fn block(&mut self, at: usize) {
        let source = self.source.blocks[self.ids[at]].clone();
        let mut stmts = Vec::with_capacity(source.stmts.len());

        for stmt in &source.stmts {
            let StmtKind::Assign { place, rvalue } = &stmt.kind;
            let rvalue = self.rvalue(rvalue, at);

            // A copy of a value defines nothing of its own: a slot reads the value it copied,
            // and a definition of the CFG form that copies one is dropped like a parameter
            // that carries one value. Constants, calls, and primitives stay definitions.
            if let Rvalue::Use(Operand::Value(value)) = &rvalue {
                let value = resolve(&self.trivial, *value);

                match place {
                    Place::Local(local) => {
                        self.current[at].insert(*local, value);
                    },
                    Place::Value(copied) => {
                        self.trivial.insert(*copied, value);
                    },
                }

                continue;
            }

            let place = match place {
                // A slot is written: the statement is the definition of a value of its own,
                // and the slot reads it from here on.
                Place::Local(local) => {
                    let data = self.source.locals[*local].clone();
                    let value = self.values.alloc(ValueData {
                        span: stmt.span,
                        ty: data.ty,
                    });

                    self.current[at].insert(*local, value);

                    Place::Value(value)
                },
                // A value is written: it is a definition of the CFG form, and the SSA form
                // keeps it.
                Place::Value(value) => Place::Value(*value),
            };

            stmts.push(Stmt {
                kind: StmtKind::Assign { place, rvalue },
                span: stmt.span,
            });
        }

        self.stmts[at] = stmts;
        let term = self.terminator(&source.term, at);
        self.terms[at] = Some(term);

        // A successor whose every predecessor is walked may read its parameters.
        self.walked[at] = true;

        for successor in self.cfg.successors(at).to_vec() {
            if self
                .cfg
                .predecessors(successor)
                .iter()
                .all(|predecessor| self.walked[*predecessor])
            {
                self.seal(successor);
            }
        }
    }

    /// Seals a block: the arguments of its incomplete parameters are read now.
    fn seal(&mut self, at: usize) {
        if self.sealed[at] {
            return;
        }

        self.sealed[at] = true;

        let incomplete: Vec<(LocalId, ValueId)> = self
            .incomplete
            .iter()
            .filter(|((block, _), _)| *block == at)
            .map(|((_, local), phi)| (*local, *phi))
            .collect();

        for (local, phi) in incomplete {
            self.incomplete.remove(&(at, local));

            let predecessors = self.cfg.predecessors(at).to_vec();
            let mut args = Vec::with_capacity(predecessors.len());

            for predecessor in &predecessors {
                args.push(self.read_local(*predecessor, local));
            }

            if let Some(first) = args.first().copied()
                && args.iter().all(|arg| *arg == first)
            {
                self.trivial.insert(phi, first);
                self.params[at].retain(|param| *param != phi);
                self.current[at].insert(local, first);
            } else {
                self.phi_args.insert(phi, args);
            }
        }
    }

    /// Rewrites an rvalue, reading every slot as a value.
    fn rvalue(&mut self, rvalue: &Rvalue, at: usize) -> Rvalue {
        match rvalue {
            Rvalue::Use(operand) => Rvalue::Use(self.operand(operand, at)),
            Rvalue::Const(constant) => Rvalue::Const(constant.clone()),
            Rvalue::Call { callee, args } => {
                Rvalue::Call {
                    callee: match callee {
                        Callee::Entity(entity) => Callee::Entity(entity.clone()),
                        Callee::Local(local) => Callee::Local(*local),
                        Callee::Indirect(operand) => Callee::Indirect(self.operand(operand, at)),
                    },
                    args: args.iter().map(|arg| self.operand(arg, at)).collect(),
                }
            },
            Rvalue::Closure { lambda, captures } => {
                Rvalue::Closure {
                    lambda: *lambda,
                    captures: captures
                        .iter()
                        .map(|capture| self.operand(capture, at))
                        .collect(),
                }
            },
            Rvalue::Capture { index } => Rvalue::Capture { index: *index },
            Rvalue::Prim { op, args } => {
                Rvalue::Prim {
                    op: *op,
                    args: args.iter().map(|arg| self.operand(arg, at)).collect(),
                }
            },
        }
    }

    /// Rewrites an operand, reading a slot as the value of its current definition.
    fn operand(&mut self, operand: &Operand, at: usize) -> Operand {
        match operand {
            Operand::Value(value) => Operand::Value(*value),
            Operand::Const(constant) => Operand::Const(constant.clone()),
            Operand::Local(local) => Operand::Value(self.read_local(at, *local)),
        }
    }

    /// Rewrites a terminator: the operands it reads, and nothing of its targets yet.
    fn terminator(&mut self, term: &Terminator, at: usize) -> Terminator {
        match term {
            Terminator::Goto { target, span } => {
                Terminator::Goto {
                    target: self.target(target),
                    span: *span,
                }
            },
            Terminator::Branch {
                cond,
                then_,
                else_,
                span,
            } => {
                Terminator::Branch {
                    cond: self.operand(cond, at),
                    then_: self.target(then_),
                    else_: self.target(else_),
                    span: *span,
                }
            },
            Terminator::Switch {
                scrutinee,
                arms,
                otherwise,
                span,
            } => {
                Terminator::Switch {
                    scrutinee: self.operand(scrutinee, at),
                    arms: arms
                        .iter()
                        .map(|(constant, target)| (constant.clone(), self.target(target)))
                        .collect(),
                    otherwise: self.target(otherwise),
                    span: *span,
                }
            },
            Terminator::Return { value, span } => {
                Terminator::Return {
                    value: self.operand(value, at),
                    span: *span,
                }
            },
            Terminator::Unreachable { span } => Terminator::Unreachable { span: *span },
        }
    }

    /// A target with its arguments cleared: they are filled once every parameter is known.
    fn target(&self, target: &BlockTarget) -> BlockTarget {
        BlockTarget {
            block: target.block,
            args: Vec::new(),
        }
    }

    /// The value a read of a slot is answered with in `at`.
    fn read_local(&mut self, at: usize, local: LocalId) -> ValueId {
        if let Some(value) = self.current[at].get(&local) {
            return *value;
        }

        self.read_local_recursive(at, local)
    }

    /// The value a read of a slot is answered with, resolved through the predecessors.
    ///
    /// A block that is not sealed yet gets an incomplete parameter: a read in a loop header
    /// comes back to it before the back edge has a definition to pass. A sealed block with one
    /// predecessor reads from it; a sealed block with several gets a parameter whose arguments
    /// are read then and there. A cycle of sealed blocks with one predecessor each --- a block
    /// no path reaches, or a loop to the entry --- is answered with a parameter too, rather
    /// than recursing forever.
    fn read_local_recursive(&mut self, at: usize, local: LocalId) -> ValueId {
        if !self.sealed[at] {
            return self.place_incomplete(at, local);
        }

        let predecessors = self.cfg.predecessors(at).to_vec();

        if let [only] = predecessors.as_slice()
            && !self.pending.contains(&(at, local))
        {
            self.pending.insert((at, local));
            let value = self.read_local(*only, local);
            self.pending.remove(&(at, local));
            self.current[at].insert(local, value);

            return value;
        }

        self.place_phi(at, local, &predecessors)
    }

    /// Places a parameter of `at` whose arguments are not known yet.
    fn place_incomplete(&mut self, at: usize, local: LocalId) -> ValueId {
        let data = self.source.locals[local].clone();
        let phi = self.values.alloc(ValueData {
            span: data.span,
            ty: data.ty,
        });

        self.params[at].push(phi);
        self.current[at].insert(local, phi);
        self.incomplete.insert((at, local), phi);

        phi
    }

    /// Places a parameter of `at` for the reads of `local`, and answers with it --- or with
    /// the one value its arguments all turned out to be.
    fn place_phi(&mut self, at: usize, local: LocalId, predecessors: &[usize]) -> ValueId {
        let data = self.source.locals[local].clone();
        let phi = self.values.alloc(ValueData {
            span: data.span,
            ty: data.ty,
        });

        self.params[at].push(phi);
        self.current[at].insert(local, phi);

        let mut args = Vec::with_capacity(predecessors.len());

        for predecessor in predecessors {
            args.push(self.read_local(*predecessor, local));
        }

        // Braun's trivial-parameter elimination: a parameter whose arguments are one value is
        // that value, and the uses the construction made read it directly.
        if let Some(first) = args.first().copied()
            && args.iter().all(|arg| *arg == first)
        {
            let popped = self.params[at].pop();
            debug_assert_eq!(popped, Some(phi));

            self.trivial.insert(phi, first);
            self.current[at].insert(local, first);

            return first;
        }

        self.phi_args.insert(phi, args);

        phi
    }

    /// Fills the arguments of every edge, once every parameter of every block is known.
    fn arguments(&mut self) {
        for at in 0..self.ids.len() {
            let Some(term) = self.terms[at].take() else {
                continue;
            };

            self.terms[at] = Some(self.terminator_arguments(&term, at));
        }
    }

    /// The terminator of a block, with the arguments of its targets filled.
    fn terminator_arguments(&self, term: &Terminator, from: usize) -> Terminator {
        match term {
            Terminator::Goto { target, span } => {
                Terminator::Goto {
                    target: self.arguments_of(target, from),
                    span: *span,
                }
            },
            Terminator::Branch {
                cond,
                then_,
                else_,
                span,
            } => {
                Terminator::Branch {
                    cond: cond.clone(),
                    then_: self.arguments_of(then_, from),
                    else_: self.arguments_of(else_, from),
                    span: *span,
                }
            },
            Terminator::Switch {
                scrutinee,
                arms,
                otherwise,
                span,
            } => {
                Terminator::Switch {
                    scrutinee: scrutinee.clone(),
                    arms: arms
                        .iter()
                        .map(|(constant, target)| {
                            (constant.clone(), self.arguments_of(target, from))
                        })
                        .collect(),
                    otherwise: self.arguments_of(otherwise, from),
                    span: *span,
                }
            },
            Terminator::Return { value, span } => {
                Terminator::Return {
                    value: value.clone(),
                    span: *span,
                }
            },
            Terminator::Unreachable { span } => Terminator::Unreachable { span: *span },
        }
    }

    /// The arguments an edge of `from` passes to a target.
    fn arguments_of(&self, target: &BlockTarget, from: usize) -> BlockTarget {
        let to = target.block.index();
        let position = self
            .cfg
            .predecessors(to)
            .iter()
            .position(|pred| *pred == from)
            .expect("an edge of a block that is a predecessor of its target");

        let args = self.params[to]
            .iter()
            .map(|param| {
                let args = &self.phi_args[param];
                Operand::Value(args[position])
            })
            .collect();

        BlockTarget {
            block: target.block,
            args,
        }
    }

    /// Builds the body: the parameters of every block, the compacted values, and no slots.
    fn finish(mut self) -> Code {
        // A value that carried one value is not a definition of anything: the arena is rebuilt
        // without it, so that every value of the SSA form has a definition.
        let mut remap: Vec<Option<ValueId>> = vec![None; self.values.len()];
        let mut values = Arena::default();

        for (id, data) in self.values.iter() {
            if self.trivial.contains_key(&id) {
                continue;
            }

            remap[id.index()] = Some(values.alloc(data.clone()));
        }

        let map = |value: ValueId| {
            let value = resolve(&self.trivial, value);

            remap[value.index()].expect("a value that was not dropped")
        };

        let mut blocks = Arena::default();

        for at in 0..self.ids.len() {
            let params = self.params[at].iter().copied().map(map).collect();
            let stmts = std::mem::take(&mut self.stmts[at])
                .into_iter()
                .map(|stmt| remap_stmt(stmt, &map))
                .collect();
            let term = self.terms[at]
                .take()
                .expect("every block of the body was walked");
            let term = remap_terminator(term, &map);

            blocks.alloc(Block {
                params,
                stmts,
                term,
            });
        }

        debug_assert_eq!(
            blocks.len(),
            self.ids.len(),
            "the walk adds no block and drops none, so a block keeps its id",
        );

        Code {
            params: self.source.params.iter().copied().map(map).collect(),
            entry: self.source.entry,
            blocks,
            values,
            locals: Arena::default(),
        }
    }
}

/// The value a use of a dropped value reads.
fn resolve(trivial: &FxHashMap<ValueId, ValueId>, value: ValueId) -> ValueId {
    let mut value = value;

    // A value that carried one value is a chain; the chain ends at a value that was not
    // dropped. A cycle would be a graph no program has, and the bound is what keeps a
    // malformed one from hanging the construction.
    for _ in 0..=trivial.len() {
        match trivial.get(&value) {
            Some(next) => value = *next,
            None => break,
        }
    }

    value
}

/// Rewrites a statement over the compacted values.
fn remap_stmt(stmt: Stmt, map: &impl Fn(ValueId) -> ValueId) -> Stmt {
    let StmtKind::Assign { place, rvalue } = stmt.kind;

    Stmt {
        kind: StmtKind::Assign {
            place: match place {
                Place::Local(local) => Place::Local(local),
                Place::Value(value) => Place::Value(map(value)),
            },
            rvalue: remap_rvalue(rvalue, map),
        },
        span: stmt.span,
    }
}

/// Rewrites an rvalue over the compacted values.
fn remap_rvalue(rvalue: Rvalue, map: &impl Fn(ValueId) -> ValueId) -> Rvalue {
    match rvalue {
        Rvalue::Use(operand) => Rvalue::Use(remap_operand(operand, map)),
        Rvalue::Const(constant) => Rvalue::Const(constant),
        Rvalue::Call { callee, args } => {
            Rvalue::Call {
                callee: match callee {
                    Callee::Entity(entity) => Callee::Entity(entity),
                    Callee::Local(local) => Callee::Local(local),
                    Callee::Indirect(operand) => Callee::Indirect(remap_operand(operand, map)),
                },
                args: args
                    .into_iter()
                    .map(|operand| remap_operand(operand, map))
                    .collect(),
            }
        },
        Rvalue::Closure { lambda, captures } => {
            Rvalue::Closure {
                lambda,
                captures: captures
                    .into_iter()
                    .map(|capture| remap_operand(capture, map))
                    .collect(),
            }
        },
        Rvalue::Capture { index } => Rvalue::Capture { index },
        Rvalue::Prim { op, args } => {
            Rvalue::Prim {
                op,
                args: args
                    .into_iter()
                    .map(|operand| remap_operand(operand, map))
                    .collect(),
            }
        },
    }
}

/// Rewrites an operand over the compacted values.
fn remap_operand(operand: Operand, map: &impl Fn(ValueId) -> ValueId) -> Operand {
    match operand {
        Operand::Value(value) => Operand::Value(map(value)),
        Operand::Local(local) => Operand::Local(local),
        Operand::Const(constant) => Operand::Const(constant),
    }
}

/// Rewrites a terminator over the compacted values.
fn remap_terminator(term: Terminator, map: &impl Fn(ValueId) -> ValueId) -> Terminator {
    match term {
        Terminator::Goto { target, span } => {
            Terminator::Goto {
                target: remap_target(target, map),
                span,
            }
        },
        Terminator::Branch {
            cond,
            then_,
            else_,
            span,
        } => {
            Terminator::Branch {
                cond: remap_operand(cond, map),
                then_: remap_target(then_, map),
                else_: remap_target(else_, map),
                span,
            }
        },
        Terminator::Switch {
            scrutinee,
            arms,
            otherwise,
            span,
        } => {
            Terminator::Switch {
                scrutinee: remap_operand(scrutinee, map),
                arms: arms
                    .into_iter()
                    .map(|(constant, target)| (constant, remap_target(target, map)))
                    .collect(),
                otherwise: remap_target(otherwise, map),
                span,
            }
        },
        Terminator::Return { value, span } => {
            Terminator::Return {
                value: remap_operand(value, map),
                span,
            }
        },
        Terminator::Unreachable { span } => Terminator::Unreachable { span },
    }
}

/// Rewrites a target over the compacted values.
fn remap_target(target: BlockTarget, map: &impl Fn(ValueId) -> ValueId) -> BlockTarget {
    BlockTarget {
        block: target.block,
        args: target
            .args
            .into_iter()
            .map(|operand| remap_operand(operand, map))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use mlkc_hir_ty::Ty;
    use mlkc_mir::{
        Block, BlockTarget, BodyBuilder, Const, LocalData, Operand, Place, PrimOp, Rvalue, Stmt,
        StmtKind, Terminator, ValueData, dump,
    };
    use mlkc_span::Span;

    use super::*;

    fn parameter(builder: &mut BodyBuilder) -> ValueId {
        builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        })
    }

    fn slot(builder: &mut BodyBuilder, name: &str) -> LocalId {
        builder.local(LocalData {
            span: Span::dummy(),
            name: Some(mlkc_hir_def::Name::new(name)),
            ty: Ty::Error,
        })
    }

    fn assign(place: Place, rvalue: Rvalue) -> Stmt {
        Stmt {
            kind: StmtKind::Assign { place, rvalue },
            span: Span::dummy(),
        }
    }

    fn goto(block: BlockId) -> Terminator {
        Terminator::Goto {
            target: BlockTarget {
                block,
                args: Vec::new(),
            },
            span: Span::dummy(),
        }
    }

    fn ret(value: Operand) -> Terminator {
        Terminator::Return {
            value,
            span: Span::dummy(),
        }
    }

    /// A block that is filled after the blocks that refer to it were allocated.
    fn placeholder(builder: &mut BodyBuilder) -> BlockId {
        builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        })
    }

    #[test]
    fn a_body_of_one_block_loses_its_slots() {
        let mut builder = BodyBuilder::new(crate::test_support::owner());
        let int = parameter(&mut builder);
        let x = slot(&mut builder, "x");
        let y = slot(&mut builder, "y");
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: vec![
                assign(Place::Local(x), Rvalue::Use(Operand::Value(int))),
                assign(Place::Local(y), Rvalue::Prim {
                    op: PrimOp::IntNeg,
                    args: vec![Operand::Local(x)],
                }),
            ],
            term: ret(Operand::Local(y)),
        });
        let body = construct_ssa(&builder.finish(entry));

        assert_eq!(body.validate_ssa(), Ok(()));
        assert_eq!(
            dump::body(&body),
            "fun main (entry b0)\n  params: v0: {error}\n  b0:\n    v1 = prim int-neg(v0)\n    return v1\n",
        );
    }

    #[test]
    fn a_join_takes_a_parameter() {
        let mut builder = BodyBuilder::new(crate::test_support::owner());
        let cond = parameter(&mut builder);
        let x = slot(&mut builder, "x");
        let y = slot(&mut builder, "y");
        let entry = placeholder(&mut builder);
        let left = placeholder(&mut builder);
        let right = placeholder(&mut builder);
        let join = placeholder(&mut builder);

        *builder.block_mut(entry) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(x), Rvalue::Const(Const::Int(1)))],
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: left,
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: right,
                    args: Vec::new(),
                },
                span: Span::dummy(),
            },
        };
        *builder.block_mut(left) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(x), Rvalue::Const(Const::Int(2)))],
            term: goto(join),
        };
        *builder.block_mut(right) = Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: goto(join),
        };
        *builder.block_mut(join) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(y), Rvalue::Use(Operand::Local(x)))],
            term: ret(Operand::Local(y)),
        };

        let body = construct_ssa(&builder.finish(entry));

        assert_eq!(body.validate_ssa(), Ok(()));
        assert_eq!(
            dump::body(&body),
            "fun main (entry b0)\n  params: v0: {error}\n  b0:\n    v1 = const 1\n    branch v0 -> b1, b2\n  b1:\n    v2 = const 2\n    goto b3(v2)\n  b2:\n    goto b3(v1)\n  b3(v3):\n    return v3\n",
        );
    }

    #[test]
    fn a_parameter_that_carries_one_value_is_dropped() {
        let mut builder = BodyBuilder::new(crate::test_support::owner());
        let cond = parameter(&mut builder);
        let x = slot(&mut builder, "x");
        let y = slot(&mut builder, "y");
        let entry = placeholder(&mut builder);
        let left = placeholder(&mut builder);
        let right = placeholder(&mut builder);
        let join = placeholder(&mut builder);

        *builder.block_mut(entry) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(x), Rvalue::Const(Const::Int(1)))],
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: left,
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: right,
                    args: Vec::new(),
                },
                span: Span::dummy(),
            },
        };
        *builder.block_mut(left) = Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: goto(join),
        };
        *builder.block_mut(right) = Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: goto(join),
        };
        *builder.block_mut(join) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(y), Rvalue::Use(Operand::Local(x)))],
            term: ret(Operand::Local(y)),
        };

        let body = construct_ssa(&builder.finish(entry));

        assert_eq!(body.validate_ssa(), Ok(()));
        assert_eq!(
            dump::body(&body),
            "fun main (entry b0)\n  params: v0: {error}\n  b0:\n    v1 = const 1\n    branch v0 -> b1, b2\n  b1:\n    goto b3\n  b2:\n    goto b3\n  b3:\n    return v1\n",
        );
    }

    #[test]
    fn a_loop_carries_its_value_in_a_parameter() {
        let mut builder = BodyBuilder::new(crate::test_support::owner());
        let cond = parameter(&mut builder);
        let x = slot(&mut builder, "x");
        let y = slot(&mut builder, "y");
        let z = slot(&mut builder, "z");
        let entry = placeholder(&mut builder);
        let head = placeholder(&mut builder);
        let body = placeholder(&mut builder);
        let exit = placeholder(&mut builder);

        *builder.block_mut(entry) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(x), Rvalue::Const(Const::Int(0)))],
            term: goto(head),
        };
        *builder.block_mut(head) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(y), Rvalue::Use(Operand::Local(x)))],
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: body,
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: exit,
                    args: Vec::new(),
                },
                span: Span::dummy(),
            },
        };
        *builder.block_mut(body) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(x), Rvalue::Prim {
                op: PrimOp::IntNeg,
                args: vec![Operand::Local(y)],
            })],
            term: goto(head),
        };
        *builder.block_mut(exit) = Block {
            params: Vec::new(),
            stmts: vec![assign(Place::Local(z), Rvalue::Use(Operand::Local(x)))],
            term: ret(Operand::Local(z)),
        };

        let body = construct_ssa(&builder.finish(entry));

        assert_eq!(body.validate_ssa(), Ok(()));
        assert_eq!(
            dump::body(&body),
            "fun main (entry b0)\n  params: v0: {error}\n  b0:\n    v1 = const 0\n    goto b1(v1)\n  b1(v2):\n    branch v0 -> b2, b3\n  b2:\n    v3 = prim int-neg(v2)\n    goto b1(v3)\n  b3:\n    return v2\n",
        );
    }
}
