//! The verifier of MIR: the invariants that tell the CFG form from the SSA form.
//!
//! The two forms of [ADR-0019][adr-0019] share their types and differ in what they promise.
//! Both promise that an edge matches the parameters of its target and that the ids an
//! instruction writes down are ids of the body; beyond that:
//!
//! - the CFG form promises that every read of a slot follows an assignment of it on every path,
//!   that no block has parameters, and that a value read is a parameter of the body or the
//!   definition of a statement;
//! - the SSA form promises that every value is defined exactly once, that no slot is read or
//!   written at all, and that every use is dominated by the definition it reads.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md

use std::{collections::VecDeque, fmt};

use crate::{
    BlockId, Body, CaptureData, CodeRef, LocalId, Operand, Place, Rvalue, StmtKind, Terminator,
    ValueId,
    cfg::{Cfg, targets},
};

/// Which form a body is asked to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// The form the lowering produces: slots, no block parameters, and no arguments on an edge.
    Cfg,
    /// The form the SSA construction produces: one definition per value, and block parameters.
    Ssa,
}

/// What a body that does not hold the invariant of its form is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invalid {
    /// The body is entered at a block it does not have.
    MissingEntry {
        /// The block the body enters.
        entry: BlockId,
    },
    /// An edge goes to a block the body does not have.
    MissingBlock {
        /// The block the edge goes to.
        block: BlockId,
    },
    /// An instruction reads a value the body does not have.
    MissingValue {
        /// The value.
        value: ValueId,
    },
    /// An instruction reads a slot the body does not have.
    MissingLocal {
        /// The slot.
        local: LocalId,
    },
    /// An edge passes a number of arguments its target does not take.
    EdgeArity {
        /// The block the edge leaves.
        block: BlockId,
        /// The block the edge goes to.
        target: BlockId,
        /// How many arguments the target takes.
        expected: usize,
        /// How many the edge passes.
        found: usize,
    },
    /// A block of the CFG form has parameters.
    CfgBlockParams {
        /// The block.
        block: BlockId,
    },
    /// A body of the SSA form holds slots.
    SsaLocals,
    /// A body of the SSA form reads or writes a slot.
    SsaLocal {
        /// The block the read or the write is in.
        block: BlockId,
    },
    /// A value is defined more than once.
    ValueDefinedTwice {
        /// The value.
        value: ValueId,
    },
    /// A value is read, and nothing defines it.
    ValueNotDefined {
        /// The value.
        value: ValueId,
    },
    /// A slot is read on a path that does not assign it.
    LocalNotAssigned {
        /// The slot.
        local: LocalId,
    },
    /// A use of a value is not dominated by its definition.
    NotDominated {
        /// The value.
        value: ValueId,
        /// The block the use is in.
        block: BlockId,
    },
    /// A value is defined after a use of it in the same block.
    DefinedAfterUse {
        /// The value.
        value: ValueId,
        /// The block.
        block: BlockId,
    },
    /// A capture is read outside a lambda body.
    CaptureOutsideLambda,
    /// A capture index is not one of the lambda's captures.
    CaptureOutOfRange {
        /// The index that is read.
        index: u32,
        /// How many captures the lambda has.
        captures: usize,
    },
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry { entry } => {
                write!(
                    f,
                    "the body is entered at {}, which is not a block of it",
                    block_text(*entry),
                )
            },
            Self::MissingBlock { block } => {
                write!(
                    f,
                    "an edge goes to {}, which is not a block of the body",
                    block_text(*block),
                )
            },
            Self::MissingValue { value } => {
                write!(
                    f,
                    "{} is read, and the body has no such value",
                    value_text(*value),
                )
            },
            Self::MissingLocal { local } => {
                write!(
                    f,
                    "{} is read, and the body has no such slot",
                    local_text(*local),
                )
            },
            Self::EdgeArity {
                block,
                target,
                expected,
                found,
            } => {
                write!(
                    f,
                    "an edge of {} to {} passes {found} arguments, and the target takes {expected}",
                    block_text(*block),
                    block_text(*target),
                )
            },
            Self::CfgBlockParams { block } => {
                write!(
                    f,
                    "{} has parameters, and a body of the CFG form has none",
                    block_text(*block),
                )
            },
            Self::SsaLocals => f.write_str("a body of the SSA form holds slots"),
            Self::SsaLocal { block } => {
                write!(
                    f,
                    "{} reads or writes a slot, and the SSA form has none",
                    block_text(*block),
                )
            },
            Self::ValueDefinedTwice { value } => {
                write!(f, "{} is defined more than once", value_text(*value))
            },
            Self::ValueNotDefined { value } => {
                write!(f, "{} is read, and nothing defines it", value_text(*value))
            },
            Self::LocalNotAssigned { local } => {
                write!(f, "{} is read where no path assigns it", local_text(*local))
            },
            Self::NotDominated { value, block } => {
                write!(
                    f,
                    "a use of {} in {} is not dominated by its definition",
                    value_text(*value),
                    block_text(*block),
                )
            },
            Self::DefinedAfterUse { value, block } => {
                write!(
                    f,
                    "{} is defined after a use of it in {}",
                    value_text(*value),
                    block_text(*block),
                )
            },
            Self::CaptureOutsideLambda => {
                f.write_str("a capture is read, and the body it is read in is not a lambda")
            },
            Self::CaptureOutOfRange { index, captures } => {
                write!(
                    f,
                    "capture {index} is read, and the lambda captured {captures}",
                )
            },
        }
    }
}

impl Body {
    /// Checks that the body holds the invariant of the CFG form.
    pub fn validate_cfg(&self) -> Result<(), Invalid> {
        self.validate(Form::Cfg)
    }

    /// Checks that the body holds the invariant of the SSA form.
    pub fn validate_ssa(&self) -> Result<(), Invalid> {
        self.validate(Form::Ssa)
    }

    /// Checks that the body holds the invariant of `form` ([ADR-0019][adr-0019]).
    ///
    /// A capture may be read only where the body is a lambda's, and only one the lambda took:
    /// the captures of the body say which those are.
    ///
    /// [adr-0019]: ../../docs/adr/0019-mir.md
    pub fn validate(&self, form: Form) -> Result<(), Invalid> {
        let captures = self.is_lambda().then_some(self.captures.as_slice());

        self.code().validate(form, captures)
    }
}

impl CodeRef<'_> {
    /// Checks that the code holds the invariant of `form`.
    ///
    /// `captures` is what the code's lambda captured, for the code of a lambda; `None` for the
    /// body of an entity, where a capture is nothing.
    fn validate(&self, form: Form, captures: Option<&[CaptureData]>) -> Result<(), Invalid> {
        self.check_ids(captures)?;
        self.check_edges()?;

        match form {
            Form::Cfg => self.check_cfg(),
            Form::Ssa => self.check_ssa(),
        }
    }

    /// Checks that every id an instruction writes down is an id of the body.
    fn check_ids(&self, captures: Option<&[CaptureData]>) -> Result<(), Invalid> {
        if self.entry.index() >= self.blocks.len() {
            return Err(Invalid::MissingEntry { entry: self.entry });
        }

        for value in self.params {
            self.check_value(*value)?;
        }

        for block in self.blocks.values() {
            for param in &block.params {
                self.check_value(*param)?;
            }

            for stmt in &block.stmts {
                let StmtKind::Assign { place, rvalue } = &stmt.kind;

                match place {
                    Place::Local(local) => self.check_local(*local)?,
                    Place::Value(value) => self.check_value(*value)?,
                }

                self.check_rvalue(rvalue, captures)?;
            }

            self.check_terminator(&block.term)?;
        }

        Ok(())
    }

    /// Checks that every edge matches the parameters of its target.
    fn check_edges(&self) -> Result<(), Invalid> {
        for (id, block) in self.blocks.iter() {
            for target in targets(&block.term) {
                let expected = self.blocks[target.block].params.len();

                if target.args.len() != expected {
                    return Err(Invalid::EdgeArity {
                        block: id,
                        target: target.block,
                        expected,
                        found: target.args.len(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Checks the invariant of the CFG form.
    fn check_cfg(&self) -> Result<(), Invalid> {
        for (id, block) in self.blocks.iter() {
            if !block.params.is_empty() {
                return Err(Invalid::CfgBlockParams { block: id });
            }
        }

        let mut defined = vec![false; self.values.len()];
        let mut parameter = vec![false; self.values.len()];

        for value in self.params {
            parameter[value.index()] = true;
        }

        for block in self.blocks.values() {
            for stmt in &block.stmts {
                let StmtKind::Assign {
                    place: Place::Value(value),
                    ..
                } = &stmt.kind
                else {
                    continue;
                };

                let at = value.index();

                if defined[at] || parameter[at] {
                    return Err(Invalid::ValueDefinedTwice { value: *value });
                }

                defined[at] = true;
            }
        }

        for block in self.blocks.values() {
            for stmt in &block.stmts {
                let StmtKind::Assign { rvalue, .. } = &stmt.kind;

                for operand in rvalue_operands(rvalue) {
                    if let Operand::Value(value) = operand {
                        let at = value.index();

                        if !parameter[at] && !defined[at] {
                            return Err(Invalid::ValueNotDefined { value: *value });
                        }
                    }
                }
            }

            for operand in terminator_operands(&block.term) {
                if let Operand::Value(value) = operand {
                    let at = value.index();

                    if !parameter[at] && !defined[at] {
                        return Err(Invalid::ValueNotDefined { value: *value });
                    }
                }
            }
        }

        self.check_definite_assignment()
    }

    /// Checks that every read of a slot follows an assignment of it on every path.
    ///
    /// A block no path from the entry reaches is not checked: there is no path to it for the
    /// promise to be about.
    fn check_definite_assignment(&self) -> Result<(), Invalid> {
        let blocks: Vec<BlockId> = self.blocks.iter().map(|(id, _)| id).collect();
        let cfg = Cfg::of(*self);
        let entry = self.entry.index();

        let mut assigned: Vec<Option<Vec<bool>>> = vec![None; self.blocks.len()];
        assigned[entry] = Some(vec![false; self.locals.len()]);
        let mut queue = VecDeque::from([entry]);

        while let Some(block) = queue.pop_front() {
            let mut state = assigned[block]
                .clone()
                .expect("a block in the queue holds an assignment state");

            for stmt in &self.blocks[blocks[block]].stmts {
                let StmtKind::Assign { place, rvalue } = &stmt.kind;

                for operand in rvalue_operands(rvalue) {
                    if let Operand::Local(local) = operand
                        && !state[local.index()]
                    {
                        return Err(Invalid::LocalNotAssigned { local: *local });
                    }
                }

                if let Place::Local(local) = place {
                    state[local.index()] = true;
                }
            }

            for operand in terminator_operands(&self.blocks[blocks[block]].term) {
                if let Operand::Local(local) = operand
                    && !state[local.index()]
                {
                    return Err(Invalid::LocalNotAssigned { local: *local });
                }
            }

            for &succ in cfg.successors(block) {
                let merged = match &assigned[succ] {
                    None => Some(state.clone()),
                    Some(old) => {
                        let mut merged = old.clone();

                        for (slot, assigned) in merged.iter_mut().zip(&state) {
                            *slot &= *assigned;
                        }

                        if merged == *old { None } else { Some(merged) }
                    },
                };

                if let Some(merged) = merged {
                    assigned[succ] = Some(merged);
                    queue.push_back(succ);
                }
            }
        }

        Ok(())
    }

    /// Checks the invariant of the SSA form.
    fn check_ssa(&self) -> Result<(), Invalid> {
        if !self.locals.is_empty() {
            return Err(Invalid::SsaLocals);
        }

        for (id, block) in self.blocks.iter() {
            let reads_a_slot = block.stmts.iter().any(|stmt| {
                let StmtKind::Assign { place, rvalue } = &stmt.kind;

                matches!(place, Place::Local(_)) || rvalue_operands(rvalue).iter().any(is_local)
            });

            if reads_a_slot || terminator_operands(&block.term).iter().any(is_local) {
                return Err(Invalid::SsaLocal { block: id });
            }
        }

        // Every value is defined once: by a parameter of the body, by a parameter of a block,
        // or by a statement.
        let entry = self.entry.index();
        let mut defs: Vec<Option<Def>> = vec![None; self.values.len()];

        for value in self.params {
            let at = value.index();

            if defs[at].is_some() {
                return Err(Invalid::ValueDefinedTwice { value: *value });
            }

            defs[at] = Some(Def::Block(entry));
        }

        for (id, block) in self.blocks.iter() {
            let at = id.index();

            for param in &block.params {
                let value = param.index();

                if defs[value].is_some() {
                    return Err(Invalid::ValueDefinedTwice { value: *param });
                }

                defs[value] = Some(Def::Block(at));
            }

            for (index, stmt) in block.stmts.iter().enumerate() {
                let StmtKind::Assign {
                    place: Place::Value(value),
                    ..
                } = &stmt.kind
                else {
                    continue;
                };

                let defined = value.index();

                if defs[defined].is_some() {
                    return Err(Invalid::ValueDefinedTwice { value: *value });
                }

                defs[defined] = Some(Def::Stmt(at, index));
            }
        }

        for (id, _) in self.values.iter() {
            if defs[id.index()].is_none() {
                return Err(Invalid::ValueNotDefined { value: id });
            }
        }

        // Every use is dominated by its definition. A use in a block no path from the entry
        // reaches is not checked: there is no path to it for the promise to be about.
        let cfg = Cfg::of(*self);
        let rpo = cfg.reverse_postorder();
        let mut number = vec![usize::MAX; self.blocks.len()];

        for (index, block) in rpo.iter().enumerate() {
            number[*block] = index;
        }

        let idom = dominators(entry, rpo, &cfg, &number);

        for (id, block) in self.blocks.iter() {
            let at = id.index();

            if number[at] == usize::MAX {
                continue;
            }

            for (index, stmt) in block.stmts.iter().enumerate() {
                let StmtKind::Assign { rvalue, .. } = &stmt.kind;

                for operand in rvalue_operands(rvalue) {
                    self.check_definition(operand, id, index, &defs, &idom)?;
                }
            }

            let term = block.stmts.len();

            for operand in terminator_operands(&block.term) {
                self.check_definition(operand, id, term, &defs, &idom)?;
            }

            for target in targets(&block.term) {
                for operand in &target.args {
                    self.check_definition(operand, id, term, &defs, &idom)?;
                }
            }
        }

        Ok(())
    }

    /// Checks that a use of a value at `at` in `block` reads a definition that dominates it.
    fn check_definition(
        &self,
        operand: &Operand,
        block: BlockId,
        at: usize,
        defs: &[Option<Def>],
        idom: &[Option<usize>],
    ) -> Result<(), Invalid> {
        let Operand::Value(value) = operand else {
            return Ok(());
        };

        match defs[value.index()].expect("every value is defined") {
            Def::Block(defined) => {
                if !dominates(defined, block.index(), idom) {
                    return Err(Invalid::NotDominated {
                        value: *value,
                        block,
                    });
                }
            },
            Def::Stmt(defined, index) => {
                if defined == block.index() {
                    if index >= at {
                        return Err(Invalid::DefinedAfterUse {
                            value: *value,
                            block,
                        });
                    }
                } else if !dominates(defined, block.index(), idom) {
                    return Err(Invalid::NotDominated {
                        value: *value,
                        block,
                    });
                }
            },
        }

        Ok(())
    }

    /// Checks that a value id is an id of the body.
    fn check_value(&self, value: ValueId) -> Result<(), Invalid> {
        if value.index() < self.values.len() {
            Ok(())
        } else {
            Err(Invalid::MissingValue { value })
        }
    }

    /// Checks that a slot id is an id of the body.
    fn check_local(&self, local: LocalId) -> Result<(), Invalid> {
        if local.index() < self.locals.len() {
            Ok(())
        } else {
            Err(Invalid::MissingLocal { local })
        }
    }

    /// Checks the ids of an rvalue.
    fn check_rvalue(
        &self,
        rvalue: &Rvalue,
        captures: Option<&[CaptureData]>,
    ) -> Result<(), Invalid> {
        for operand in rvalue_operands(rvalue) {
            match operand {
                Operand::Value(value) => self.check_value(*value)?,
                Operand::Local(local) => self.check_local(*local)?,
                Operand::Const(_) => {},
            }
        }

        if let Rvalue::Capture { index } = rvalue {
            let Some(captures) = captures else {
                return Err(Invalid::CaptureOutsideLambda);
            };

            if *index as usize >= captures.len() {
                return Err(Invalid::CaptureOutOfRange {
                    index: *index,
                    captures: captures.len(),
                });
            }
        }

        Ok(())
    }

    /// Checks the ids of a terminator.
    fn check_terminator(&self, term: &Terminator) -> Result<(), Invalid> {
        for operand in terminator_operands(term) {
            match operand {
                Operand::Value(value) => self.check_value(*value)?,
                Operand::Local(local) => self.check_local(*local)?,
                Operand::Const(_) => {},
            }
        }

        for target in targets(term) {
            if target.block.index() >= self.blocks.len() {
                return Err(Invalid::MissingBlock {
                    block: target.block,
                });
            }

            for operand in &target.args {
                match operand {
                    Operand::Value(value) => self.check_value(*value)?,
                    Operand::Local(local) => self.check_local(*local)?,
                    Operand::Const(_) => {},
                }
            }
        }

        Ok(())
    }
}

/// Where a value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Def {
    /// A parameter of a body or of a block, defined where the block is entered.
    Block(usize),
    /// A statement, defined where the statement runs.
    Stmt(usize, usize),
}

/// The operands of an rvalue.
fn rvalue_operands(rvalue: &Rvalue) -> Vec<&Operand> {
    match rvalue {
        Rvalue::Use(operand) => vec![operand],
        Rvalue::Const(_) => Vec::new(),
        Rvalue::Call { callee, args } => {
            let mut operands: Vec<&Operand> = args.iter().collect();

            if let crate::Callee::Indirect(operand) = callee {
                operands.push(operand);
            }

            operands
        },
        Rvalue::Closure { captures, .. } => captures.iter().collect(),
        Rvalue::Capture { .. } => Vec::new(),
        Rvalue::Prim { args, .. } => args.iter().collect(),
    }
}

/// The operands of a terminator, the arguments of its targets included.
fn terminator_operands(term: &Terminator) -> Vec<&Operand> {
    let mut operands: Vec<&Operand> = Vec::new();

    match term {
        Terminator::Goto { .. } => {},
        Terminator::Branch { cond, .. } => operands.push(cond),
        Terminator::Switch { scrutinee, .. } => operands.push(scrutinee),
        Terminator::Return { value, .. } => operands.push(value),
        Terminator::Unreachable { .. } => {},
    }

    for target in targets(term) {
        operands.extend(&target.args);
    }

    operands
}

/// Whether an operand reads a slot.
fn is_local(operand: &&Operand) -> bool {
    matches!(operand, Operand::Local(_))
}

/// The immediate dominator of every reachable block, in the style of Cooper, Harvey, and
/// Kennedy: iterate the reverse postorder until nothing changes.
fn dominators(entry: usize, rpo: &[usize], cfg: &Cfg, number: &[usize]) -> Vec<Option<usize>> {
    let mut idom = vec![None; number.len()];
    idom[entry] = Some(entry);

    loop {
        let mut changed = false;

        for &block in rpo {
            if block == entry {
                continue;
            }

            let mut new = None;

            for &pred in cfg.predecessors(block) {
                if idom[pred].is_none() {
                    continue;
                }

                new = Some(match new {
                    None => pred,
                    Some(current) => intersect(current, pred, &idom, number),
                });
            }

            if new != idom[block] {
                idom[block] = new;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    idom
}

/// The common dominator of two blocks, in the style of Cooper, Harvey, and Kennedy.
fn intersect(a: usize, b: usize, idom: &[Option<usize>], number: &[usize]) -> usize {
    let mut a = a;
    let mut b = b;

    while a != b {
        while number[a] > number[b] {
            a = idom[a].expect("a reachable block has a dominator");
        }

        while number[b] > number[a] {
            b = idom[b].expect("a reachable block has a dominator");
        }
    }

    a
}

/// Whether the block `defined` dominates the block `used`.
fn dominates(defined: usize, used: usize, idom: &[Option<usize>]) -> bool {
    let mut block = used;

    loop {
        if block == defined {
            return true;
        }

        match idom[block] {
            Some(next) if next != block => block = next,
            _ => return false,
        }
    }
}

/// The label of a block, for a message.
fn block_text(block: BlockId) -> String {
    format!("b{}", block.index())
}

/// The label of a value, for a message.
fn value_text(value: ValueId) -> String {
    format!("v{}", value.index())
}

/// The label of a slot, for a message.
fn local_text(local: LocalId) -> String {
    format!("l{}", local.index())
}

#[cfg(test)]
mod tests {
    use mlkc_hir_ty::Ty;
    use mlkc_span::Span;

    use super::*;
    use crate::{
        Block, BlockTarget, BodyBuilder, Const, FunctionLoc, LocalData, PrimOp, Stmt, ValueData,
        test_support,
    };

    /// A builder of the body of a test function.
    fn builder() -> BodyBuilder {
        BodyBuilder::new(FunctionLoc::Entity(test_support::owner()), Ty::Error)
    }

    fn value(builder: &mut BodyBuilder) -> ValueId {
        builder.value(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        })
    }

    fn slot(builder: &mut BodyBuilder) -> LocalId {
        builder.local(LocalData {
            span: Span::dummy(),
            name: None,
            ty: Ty::Error,
        })
    }

    fn parameter(builder: &mut BodyBuilder) -> ValueId {
        builder.param(ValueData {
            span: Span::dummy(),
            ty: Ty::Error,
        })
    }

    /// A CFG body of two blocks: the entry assigns a slot and branches to a block that reads it.
    fn cfg_body() -> Body {
        let mut builder = builder();
        let int = parameter(&mut builder);
        let cond = parameter(&mut builder);
        let slot = slot(&mut builder);
        let done = builder.block(Block {
            params: Vec::new(),
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Local(slot),
                    rvalue: Rvalue::Prim {
                        op: PrimOp::IntNeg,
                        args: vec![Operand::Value(int)],
                    },
                },
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: Operand::Local(slot),
                span: Span::dummy(),
            },
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: done,
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: done,
                    args: Vec::new(),
                },
                span: Span::dummy(),
            },
        });

        builder.finish(entry)
    }

    /// An SSA body of two blocks: the entry defines a value, and the join takes it as a
    /// parameter.
    fn ssa_body() -> Body {
        let mut builder = builder();
        let int = parameter(&mut builder);
        let cond = parameter(&mut builder);
        let negated = value(&mut builder);
        let join = value(&mut builder);
        let done = builder.block(Block {
            params: vec![join],
            stmts: Vec::new(),
            term: Terminator::Return {
                value: Operand::Value(join),
                span: Span::dummy(),
            },
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Value(negated),
                    rvalue: Rvalue::Prim {
                        op: PrimOp::IntNeg,
                        args: vec![Operand::Value(int)],
                    },
                },
                span: Span::dummy(),
            }],
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: done,
                    args: vec![Operand::Value(negated)],
                },
                else_: BlockTarget {
                    block: done,
                    args: vec![Operand::Value(negated)],
                },
                span: Span::dummy(),
            },
        });

        builder.finish(entry)
    }

    #[test]
    fn a_cfg_body_holds_its_invariant() {
        let body = cfg_body();

        assert_eq!(body.validate_cfg(), Ok(()));
        assert_eq!(
            body.validate_ssa(),
            Err(Invalid::SsaLocals),
            "a body with slots is not in the SSA form",
        );
    }

    #[test]
    fn a_slot_read_where_no_path_assigns_it_is_a_mistake() {
        let mut builder = builder();
        let slot = slot(&mut builder);
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Return {
                value: Operand::Local(slot),
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(
            body.validate_cfg(),
            Err(Invalid::LocalNotAssigned { local: slot }),
        );
    }

    #[test]
    fn a_body_of_one_statement_holds_the_invariant() {
        let mut builder = builder();
        let slot = slot(&mut builder);
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Local(slot),
                    rvalue: Rvalue::Const(Const::Unit),
                },
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: Operand::Local(slot),
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(body.validate_cfg(), Ok(()));
    }

    #[test]
    fn an_ssa_body_holds_its_invariant() {
        let body = ssa_body();
        let join = body
            .blocks
            .iter()
            .find(|(_, block)| !block.params.is_empty())
            .expect("a join")
            .0;

        assert_eq!(body.validate_ssa(), Ok(()));
        assert_eq!(
            body.validate_cfg(),
            Err(Invalid::CfgBlockParams { block: join }),
            "a block parameter is not of the CFG form",
        );
    }

    #[test]
    fn a_use_that_its_definition_does_not_dominate_is_a_mistake() {
        // The entry passes a value the join defines: the definition comes after the use.
        let mut builder = builder();
        let cond = parameter(&mut builder);
        let defined = value(&mut builder);
        let join = value(&mut builder);
        let done = builder.block(Block {
            params: vec![join],
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Value(defined),
                    rvalue: Rvalue::Const(Const::Unit),
                },
                span: Span::dummy(),
            }],
            term: Terminator::Return {
                value: Operand::Value(defined),
                span: Span::dummy(),
            },
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: done,
                    args: vec![Operand::Value(defined)],
                },
                else_: BlockTarget {
                    block: done,
                    args: vec![Operand::Value(defined)],
                },
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(
            body.validate_ssa(),
            Err(Invalid::NotDominated {
                value: defined,
                block: entry,
            }),
        );
    }

    #[test]
    fn a_use_before_the_definition_in_one_block_is_a_mistake() {
        let mut builder = builder();
        let value = value(&mut builder);
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: vec![
                Stmt {
                    kind: StmtKind::Assign {
                        place: Place::Value(value),
                        rvalue: Rvalue::Use(Operand::Value(value)),
                    },
                    span: Span::dummy(),
                },
                Stmt {
                    kind: StmtKind::Assign {
                        place: Place::Value(value),
                        rvalue: Rvalue::Const(Const::Unit),
                    },
                    span: Span::dummy(),
                },
            ],
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(
            body.validate_ssa(),
            Err(Invalid::ValueDefinedTwice { value }),
        );
    }

    #[test]
    fn an_edge_with_the_wrong_number_of_arguments_is_a_mistake() {
        let mut body = ssa_body();
        let entry = body.entry;

        if let Terminator::Branch { then_, .. } = &mut body.blocks[entry].term {
            then_.args.clear();
        }

        assert!(matches!(
            body.validate_ssa(),
            Err(Invalid::EdgeArity { .. })
        ));
    }

    #[test]
    fn a_value_of_a_body_that_nothing_defines_is_a_mistake() {
        let mut builder = builder();
        let undefined = value(&mut builder);
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable {
                span: Span::dummy(),
            },
        });
        let body = builder.finish(entry);

        assert_eq!(
            body.validate_ssa(),
            Err(Invalid::ValueNotDefined { value: undefined }),
        );
    }
}
