//! The lowering of a checked body into the CFG form of MIR.
//!
//! The lowering is one walk of the HIR body in post-order, one slot per expression: the
//! operands of an expression are read from the slots of its children, and the result is
//! assigned to the slot of the expression ([ADR-0019][adr-0019]). What the walk reads of the
//! checker is the type of every node --- which is what picks a `PrimOp` and what a `ValueData`
//! remembers --- and what it reads of the resolution is a callee: the path of a call is
//! resolved the way the check resolved it, and never by walking a module table here.
//!
//! A body of one block is what the walk makes of an expression with no choice in it. An `if`
//! is lowered into blocks of its own: every condition is evaluated in a block that branches,
//! every arm is a block that writes the slot of the expression and goes to the block the arms
//! meet in, and what is written after the `if` is written in that block. The blocks are
//! allocated as the walk meets them, and a block is filled when control reaches its end, so
//! a terminator names blocks that are filled later.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md
//!
//! A body whose check reported a mistake is not lowered: a `Ty::Error` expression has no
//! meaning to lower, and the driver is what asks for MIR only when the body checks clean. The
//! walk is total over such a body: what it cannot lower is a mistake the check should have
//! reported, so it is an internal compiler exception and not a diagnostic ([adr-0019]).

use mlkc_diagnostics::ice;
use mlkc_hir_def::{
    BinaryOp, Body, BodyEntityLoc, ClassLoc, EntityLoc, Expr, ExprId, FunctionLoc, ItemTree,
    Literal, LocalDefId, ModuleId, Name, Namespace, Pat, PatId, PathAnchor, PathData, UnaryOp,
};
use mlkc_hir_ty::{CheckedBody, INT_MAX, INT_MIN, Ty};
use mlkc_la_arena::ArenaMap;
use mlkc_lower::BodySourceMap;
use mlkc_mir::{
    Block, BlockId, BlockTarget, Body as MirBody, BodyBuilder, Callee, Const, LocalData, LocalId,
    Operand, Place, PrimOp, Rvalue, Stmt, StmtKind, Terminator, ValueData,
};
use mlkc_resolve::{Resolution, Walk};
use mlkc_span::Span;
use mlkc_typeck::CheckDeps;

/// Lowers the checked body of an entity into the CFG form of MIR.
///
/// `tree` is the surface of the module the body is of: the lowering checks that the entity and
/// the body are of one module, and reads nothing else of it. `source_map` is where the nodes of
/// the HIR body are written ([`BodySourceMap`]), `checked` is what the check of the body left
/// behind, and `resolution` and `deps` are what the check read: a path of a call is resolved
/// with the same walk the check used ([`Walk::entity_of`]), so the two read it the same way
/// ([ADR-0019][adr-0019]).
///
/// The pass reports nothing: semantics is checked in the checker, and every node of a body the
/// check accepted has a meaning this walk can lower. A node it cannot lower is a gap of the
/// check, and the pass says so by stopping with an internal compiler exception --- it never
/// manufactures a body out of a construct the language does not have.
///
/// [adr-0019]: ../../docs/adr/0019-mir.md
pub fn lower_body(
    owner: BodyEntityLoc,
    tree: &ItemTree,
    body: &Body,
    source_map: &BodySourceMap,
    checked: &CheckedBody,
    resolution: &Resolution,
    deps: &CheckDeps,
) -> MirBody {
    debug_assert_eq!(
        owner.module(),
        tree.module(),
        "a body is of an entity of the tree it is lowered against",
    );

    let module = owner.module();
    let walk = Walk::of(deps.graph(), deps.closure());
    let mut builder = BodyBuilder::new(owner.clone());
    let entry = open_block(&mut builder);

    let lowerer = Lowerer {
        module,
        body,
        source_map,
        checked,
        deps,
        resolution,
        walk,
        builder,
        slots: ArenaMap::default(),
        pat_slots: ArenaMap::default(),
        current: entry,
        stmts: Vec::new(),
    };

    lowerer.run(entry)
}

/// Opens a block: a placeholder the lowering fills when control reaches its end.
///
/// A block is opened before it is filled: a terminator names the blocks control goes to, and
/// a block that is not filled yet has no terminator to be named by.
fn open_block(builder: &mut BodyBuilder) -> BlockId {
    builder.block(Block {
        params: Vec::new(),
        stmts: Vec::new(),
        term: Terminator::Unreachable {
            span: Span::dummy(),
        },
    })
}

/// The lowering of one body.
struct Lowerer<'a> {
    /// The module the body is of.
    module: ModuleId,
    /// The HIR body.
    body: &'a Body,
    /// Where the nodes of the HIR body are written.
    source_map: &'a BodySourceMap,
    /// The types the check recorded.
    checked: &'a CheckedBody,
    /// What the check read: the projects, the closure, the surfaces, and the classes of the
    /// language.
    deps: &'a CheckDeps,
    /// What the resolution of the module left: what each import resolved to.
    resolution: &'a Resolution,
    /// The walk over the closure the check was handed: what resolves the path of a call.
    walk: Walk<'a>,
    /// The MIR body under construction.
    builder: BodyBuilder,
    /// The slot of every expression, once it is lowered.
    slots: ArenaMap<ExprId, LocalId>,
    /// The slot of every pattern, once it is bound.
    pat_slots: ArenaMap<PatId, LocalId>,
    /// The block being built: the statements collected so far are its, and filling it gives it
    /// the terminator control goes on with.
    current: BlockId,
    /// The statements of the block being built, in the order they were lowered.
    stmts: Vec<Stmt>,
}

impl Lowerer<'_> {
    /// Lowers the body and finishes it.
    fn run(mut self, entry: BlockId) -> MirBody {
        for pat in self.body.params() {
            self.parameter(*pat);
        }

        let root = self.expr(self.body.root());
        let span = self.span_expr(self.body.root());
        self.seal(Terminator::Return {
            value: Operand::Local(root),
            span,
        });

        self.builder.finish(entry)
    }

    /// Lowers one expression into a slot of its own, and answers that slot.
    fn expr(&mut self, expr: ExprId) -> LocalId {
        if let Some(slot) = self.slots.get(expr) {
            return *slot;
        }

        let span = self.span_expr(expr);
        let ty = self.checked.expr_type(expr).cloned().unwrap_or(Ty::Error);
        let slot = self.slot(span, None, ty);
        let node = self.body[expr].clone();

        // An `if` writes its slot in every arm of it: the slot is the value the arms agree on,
        // and the statements of an arm are the statements of a block of its own.
        if let Expr::If { .. } = &node {
            self.if_expr(slot, &node, span);
            self.slots.insert(expr, slot);

            return slot;
        }

        let rvalue = self.rvalue(&node);

        self.assign(Place::Local(slot), rvalue, span);
        self.slots.insert(expr, slot);

        slot
    }

    /// Lowers one node of the HIR.
    ///
    /// Every arm is a node the check accepted: a node the language does not have is one the
    /// check reported, and a body it reported about is not lowered.
    fn rvalue(&mut self, node: &Expr) -> Rvalue {
        match node {
            Expr::Missing => {
                ice!("the lowering met an expression that is not there, and the body checks clean")
            },
            Expr::Path(path) => {
                let path = self.body[*path].clone();
                self.value_path(&path)
            },
            Expr::Literal(Literal::Int(value)) => {
                if !(INT_MIN..=INT_MAX).contains(value) {
                    ice!(
                        "the lowering met the `Int` literal `{value}`, which does not fit the \
                         31-bit representation, and the check accepted it"
                    );
                }

                Rvalue::Const(Const::Int(*value as i32))
            },
            Expr::Literal(Literal::Bool(value)) => Rvalue::Const(Const::Bool(*value)),
            Expr::Literal(Literal::Str(value)) => Rvalue::Const(Const::Str(value.clone())),
            Expr::Call { callee, args } => {
                let mut lowered = Vec::with_capacity(args.len());

                for arg in args {
                    lowered.push(Operand::Local(self.expr(*arg)));
                }

                let callee = self.callee(*callee);

                Rvalue::Call {
                    callee,
                    args: lowered,
                }
            },
            Expr::Field { field, .. } => {
                ice!(
                    "the lowering met a read of the field `{field:?}`, and the language has no \
                     fields"
                )
            },
            Expr::Binary { lhs, op, rhs } => self.binary(*lhs, *op, *rhs),
            Expr::Unary { op, operand } => self.unary(*op, *operand),
            Expr::Let { pat, expr, body } => {
                let bound = self.expr(*expr);
                self.bind(*pat, bound);
                let value = self.expr(*body);

                Rvalue::Use(Operand::Local(value))
            },
            // An `if` writes its slot in every arm of it, and the arms are blocks of their own:
            // it is lowered by `Lowerer::if_expr`, which `Lowerer::expr` calls before this.
            Expr::If { .. } => {
                ice!("the lowering met an `if` outside the rule that reads an `if`")
            },
        }
    }

    /// Lowers an `if` into the blocks of it, writing `slot` in every arm.
    ///
    /// Every condition is evaluated in the block control reaches it in and branches to the
    /// block of the arm it selects and to the block of the next condition --- or, for the last
    /// one, to the block of the expression selected when nothing holds. Every arm writes the
    /// slot of the expression and goes to the block the arms meet in, and what is written after
    /// the `if` is written there.
    ///
    /// An `if` without an `else` selects no value: the block control falls into writes the unit
    /// the expression is, and the arms are `Unit` like it. The block is written like any other
    /// arm of the choice, so what is written after the `if` reads one slot however many arms
    /// the choice has.
    fn if_expr(&mut self, slot: LocalId, node: &Expr, span: Span) {
        let Expr::If {
            cond,
            then_,
            arms,
            otherwise,
        } = node
        else {
            ice!("the lowering met an expression that is not an `if` in the rule of an `if`")
        };

        // The conditions and the expressions they select, in the order they are written: the
        // first condition is the one the current block evaluates.
        let mut branches = Vec::with_capacity(1 + arms.len());
        branches.push((*cond, *then_));
        branches.extend(arms.iter().map(|arm| (arm.cond, arm.body)));

        // Every arm is a block, every condition after the first is a block of its own --- the
        // condition is evaluated after the arm before it failed, and the block it is evaluated
        // in is the one that branches --- and the expression selected when nothing holds is a
        // block after them. The block the arms meet in is opened last.
        let mut bodies = Vec::with_capacity(branches.len());
        let mut conditions = Vec::with_capacity(branches.len().saturating_sub(1));

        for index in 0..branches.len() {
            bodies.push(self.open());

            if index + 1 < branches.len() {
                conditions.push(self.open());
            }
        }

        let other = self.open();
        let join = self.open();

        // The first condition is evaluated where control already is, and what stands after it
        // is the block of the next condition, or the last expression when there is none.
        let value = self.expr(branches[0].0);
        let next = conditions.first().copied().unwrap_or(other);

        self.seal(Terminator::Branch {
            cond: Operand::Local(value),
            then_: BlockTarget {
                block: bodies[0],
                args: Vec::new(),
            },
            else_: BlockTarget {
                block: next,
                args: Vec::new(),
            },
            span: self.span_expr(branches[0].0),
        });

        // Every condition after the first is evaluated in the block the one before it failed
        // into.
        for (index, (condition, _)) in branches.iter().enumerate().skip(1) {
            self.current = conditions[index - 1];

            let value = self.expr(*condition);
            let next = conditions.get(index).copied().unwrap_or(other);

            self.seal(Terminator::Branch {
                cond: Operand::Local(value),
                then_: BlockTarget {
                    block: bodies[index],
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: next,
                    args: Vec::new(),
                },
                span: self.span_expr(*condition),
            });
        }

        // Every arm writes the slot of the expression and goes to the block the arms meet in.
        for (index, (_, body)) in branches.iter().enumerate() {
            self.current = bodies[index];

            let span = self.span_expr(*body);
            let value = self.expr(*body);

            self.assign(Place::Local(slot), Rvalue::Use(Operand::Local(value)), span);
            self.seal(Terminator::Goto {
                target: BlockTarget {
                    block: join,
                    args: Vec::new(),
                },
                span,
            });
        }

        // The expression selected when no condition holds: the walk of it, or --- an `if`
        // without an `else` --- the unit the expression is.
        self.current = other;

        let end = match otherwise {
            Some(otherwise) => {
                let span = self.span_expr(*otherwise);
                let value = self.expr(*otherwise);

                self.assign(Place::Local(slot), Rvalue::Use(Operand::Local(value)), span);

                span
            },
            None => {
                self.assign(Place::Local(slot), Rvalue::Const(Const::Unit), span);

                span
            },
        };

        self.seal(Terminator::Goto {
            target: BlockTarget {
                block: join,
                args: Vec::new(),
            },
            span: end,
        });

        self.current = join;
    }

    /// Binds a parameter of the body: a value, and the slot it is bound to.
    fn parameter(&mut self, pat: PatId) {
        let span = self.span_pat(pat);
        let ty = self.checked.pat_type(pat).cloned().unwrap_or(Ty::Error);
        let value = self.builder.param(ValueData {
            span,
            ty: ty.clone(),
        });

        self.binding(pat, Rvalue::Use(Operand::Value(value)), span, ty);
    }

    /// Binds the pattern of a `let` to the slot of the expression it is bound to.
    fn bind(&mut self, pat: PatId, source: LocalId) {
        let span = self.span_pat(pat);
        let ty = self.checked.pat_type(pat).cloned().unwrap_or(Ty::Error);

        self.binding(pat, Rvalue::Use(Operand::Local(source)), span, ty);
    }

    /// Binds a pattern to the value of an rvalue, where the pattern binds a name.
    fn binding(&mut self, pat: PatId, rvalue: Rvalue, span: Span, ty: Ty) {
        match &self.body[pat] {
            Pat::Bind(name) => {
                let slot = self.slot(span, Some(name.clone()), ty);
                self.assign(Place::Local(slot), rvalue, span);
                self.pat_slots.insert(pat, slot);
            },
            Pat::Wildcard => {},
            Pat::Missing => {
                ice!("the lowering met a pattern that is not there, and the body checks clean")
            },
        }
    }

    /// Lowers a path written where a value belongs.
    ///
    /// The check resolves the paths of a body, and a body it reported about is not lowered: a
    /// path here is one whose root and whose imports the check already read, and what the walk
    /// answers is what the check answered.
    fn value_path(&mut self, path: &PathData) -> Rvalue {
        match path.anchor.clone() {
            // A binding of the body: the slot the binding was given.
            PathAnchor::Binding(pat) => {
                match self.pat_slots.get(pat) {
                    Some(slot) => Rvalue::Use(Operand::Local(*slot)),
                    None => {
                        ice!(
                            "the lowering met a read of the binding `{}`, which no pattern of the body \
                     binds",
                            path.name(),
                        )
                    },
                }
            },
            // A type variable and an entity declared inside the body are not values the
            // language has, and a name of another namespace is what the check reported.
            PathAnchor::TypeVar(_) | PathAnchor::Local(_) => {
                ice!(
                    "the lowering met the path `{}` where a value belongs, and the check accepted \
                 it",
                    path.name(),
                )
            },
            // A name of the project: what the check resolved it to, read the same way here.
            PathAnchor::Item(_) | PathAnchor::Use(_) | PathAnchor::Project(_) => {
                let (entity, _) =
                    self.walk
                        .entity_of(self.module, self.resolution, path, Namespace::Value);

                match entity {
                    // A function, a constant, or a class written where a value belongs: the
                    // check rejects a name that denotes no value, and the language has no value
                    // of an entity yet.
                    Some(entity) => {
                        ice!(
                            "the lowering met the path `{}`, which resolves to `{:?}`, where a value \
                         belongs",
                            path.name(),
                            entity.item,
                        )
                    },
                    None => {
                        ice!(
                            "the lowering met the path `{}`, which resolves to nothing, and the \
                         check accepted it",
                            path.name(),
                        )
                    },
                }
            },
            PathAnchor::Unresolved => {
                ice!(
                    "the lowering met the path `{}`, which the lowering anchored to nothing, and the \
                 check accepted it",
                    path.name(),
                )
            },
        }
    }

    /// Resolves the callee of a call.
    fn callee(&mut self, expr: ExprId) -> Callee {
        let Expr::Path(path) = self.body[expr].clone() else {
            // A callee that is not a name is a value that is called: a word held in a slot.
            return Callee::Indirect(Operand::Local(self.expr(expr)));
        };

        let path = self.body[path].clone();

        match path.anchor.clone() {
            PathAnchor::Binding(pat) => {
                match self.pat_slots.get(pat) {
                    Some(slot) => Callee::Indirect(Operand::Local(*slot)),
                    None => {
                        ice!(
                            "the lowering met a call of the binding `{}`, which no pattern of the body \
                     binds",
                            path.name(),
                        )
                    },
                }
            },
            PathAnchor::Local(LocalDefId::Function(local)) => Callee::Local(local),
            PathAnchor::Local(LocalDefId::Const(_)) => {
                ice!(
                    "the lowering met a call of the constant `{}`, and the check accepted it",
                    path.name(),
                )
            },
            _ => {
                let (entity, _) =
                    self.walk
                        .entity_of(self.module, self.resolution, &path, Namespace::Value);

                match entity {
                    Some(entity) => {
                        match FunctionLoc::try_from(entity.item.clone()) {
                            Ok(function) => {
                                Callee::Entity(EntityLoc {
                                    module: entity.module,
                                    item: function,
                                })
                            },
                            Err(_) => {
                                ice!(
                                    "the lowering met a call of `{}`, which is not a function, and the \
                             check accepted it",
                                    path.name(),
                                )
                            },
                        }
                    },
                    None => {
                        ice!(
                            "the lowering met a call of `{}`, which resolves to nothing, and the \
                         check accepted it",
                            path.name(),
                        )
                    },
                }
            },
        }
    }

    /// Lowers a binary operation into a primitive.
    fn binary(&mut self, lhs: ExprId, op: BinaryOp, rhs: ExprId) -> Rvalue {
        let left = self.expr(lhs);
        let right = self.expr(rhs);
        let args = vec![Operand::Local(left), Operand::Local(right)];

        let (Some(lhs_ty), Some(rhs_ty)) = (
            self.checked.expr_type(lhs).cloned(),
            self.checked.expr_type(rhs).cloned(),
        ) else {
            ice!("the lowering met `{op}` on operands the check left no type for")
        };

        Rvalue::Prim {
            op: self.binary_prim(op, &lhs_ty, &rhs_ty),
            args,
        }
    }

    /// The primitive of a binary operator.
    ///
    /// Every operator of the language is a rule of the check for now, and the rule is what
    /// picks the primitive: a pair of operands the check accepted has one, and a pair without
    /// one is a gap of the check.
    fn binary_prim(&mut self, op: BinaryOp, lhs: &Ty, rhs: &Ty) -> PrimOp {
        let int = self.deps.builtins().int().clone();
        let boolean = self.deps.builtins().boolean().clone();
        let ints = is_class(lhs, &int) && is_class(rhs, &int);
        let booleans = is_class(lhs, &boolean) && is_class(rhs, &boolean);

        match op {
            BinaryOp::Add if ints => PrimOp::IntAdd,
            BinaryOp::Sub if ints => PrimOp::IntSub,
            BinaryOp::Mul if ints => PrimOp::IntMul,
            BinaryOp::Div if ints => PrimOp::IntDiv,
            BinaryOp::Lt if ints => PrimOp::IntLt,
            BinaryOp::Le if ints => PrimOp::IntLe,
            BinaryOp::Gt if ints => PrimOp::IntGt,
            BinaryOp::Ge if ints => PrimOp::IntGe,
            BinaryOp::Eq if ints => PrimOp::IntEq,
            BinaryOp::Ne if ints => PrimOp::IntNe,
            BinaryOp::Eq if booleans => PrimOp::BoolEq,
            BinaryOp::Ne if booleans => PrimOp::BoolNe,
            BinaryOp::And if booleans => PrimOp::BoolAnd,
            BinaryOp::Or if booleans => PrimOp::BoolOr,
            op => {
                ice!(
                    "the lowering met `{op}` on `{lhs}` and `{rhs}`, and the language has no \
                 primitive for it",
                )
            },
        }
    }

    /// Lowers a unary operation.
    fn unary(&mut self, op: UnaryOp, operand: ExprId) -> Rvalue {
        let value = self.expr(operand);

        match op {
            // A sign that keeps the value is the value.
            UnaryOp::Pos => Rvalue::Use(Operand::Local(value)),
            UnaryOp::Neg => {
                let ty = self.checked.expr_type(operand).cloned();

                match ty {
                    Some(ty) if is_class(&ty, self.deps.builtins().int()) => {
                        Rvalue::Prim {
                            op: PrimOp::IntNeg,
                            args: vec![Operand::Local(value)],
                        }
                    },
                    Some(ty) => {
                        ice!(
                            "the lowering met `{op}` on `{ty}`, and the language has no primitive \
                         for it",
                        )
                    },
                    None => {
                        ice!(
                            "the lowering met `{op}` on an operand the check left no type \
                                  for"
                        )
                    },
                }
            },
        }
    }

    /// Allocates a slot.
    fn slot(&mut self, span: Span, name: Option<Name>, ty: Ty) -> LocalId {
        self.builder.local(LocalData { span, name, ty })
    }

    /// Opens a block: a placeholder the walk fills when control reaches its end.
    fn open(&mut self) -> BlockId {
        open_block(&mut self.builder)
    }

    /// Fills the current block, and gives it the terminator control goes on with.
    ///
    /// The statements collected so far are the statements of the block, and the walk goes on in
    /// whatever block the caller makes current next.
    fn seal(&mut self, term: Terminator) {
        let stmts = std::mem::take(&mut self.stmts);

        *self.builder.block_mut(self.current) = Block {
            params: Vec::new(),
            stmts,
            term,
        };
    }

    /// Writes one assignment into the current block.
    fn assign(&mut self, place: Place, rvalue: Rvalue, span: Span) {
        self.stmts.push(Stmt {
            kind: StmtKind::Assign { place, rvalue },
            span,
        });
    }

    /// Where an expression is written.
    fn span_expr(&self, expr: ExprId) -> Span {
        self.source_map
            .expr(expr)
            .map_or_else(Span::dummy, |range| Span::new(self.module.0, range))
    }

    /// Where a pattern is written.
    fn span_pat(&self, pat: PatId) -> Span {
        self.source_map
            .pat(pat)
            .map_or_else(Span::dummy, |range| Span::new(self.module.0, range))
    }
}

/// Whether a type is a class that takes no arguments, and that class.
fn is_class(ty: &Ty, class: &EntityLoc<ClassLoc>) -> bool {
    matches!(ty, Ty::Class { class: found, args } if args.is_empty() && found == class)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use mlkc_diagnostics::Ice;
    use mlkc_hir_def::{
        BodyEntityLoc, ClassLoc, EntityLoc, ItemLocLike, ItemTree, ModuleId, ModuleScope, Name,
        Prelude, ProjectGraph,
    };
    use mlkc_lower::{LoweredBody, lower_body as lower_hir, lower_module};
    use mlkc_parser::parse;
    use mlkc_resolve::{Closure, Resolution};
    use mlkc_syntax::ModuleRoot;
    use mlkc_typeck::{Builtins, CheckDeps, check_body, resolve_module_types};
    use mlkc_vfs::{FileId, RelPathBuf};

    use super::*;
    use crate::construct_ssa;

    /// The classes of the language, as a module of a test declares them.
    const CLASSES: &str = "#[builtin]\ntype Int\n\n#[builtin]\ntype Unit\n\n#[builtin]\ntype String\n\n#[builtin]\ntype Bool\n";

    /// A module of a test: its surface, its bodies, and what the check and the lowering read.
    struct Fixture {
        tree: ItemTree,
        resolution: Resolution,
        deps: CheckDeps,
        bodies: Vec<(BodyEntityLoc, LoweredBody)>,
    }

    /// The module of `source`, lowered, resolved, and checked as far as a surface goes.
    fn fixture(source: &str) -> Fixture {
        let module = ModuleId(FileId::from_raw(0));
        let parsed = parse(source);
        let root = parsed.tree::<ModuleRoot>();
        let relative = RelPathBuf::try_from("main.mlk").expect("a relative path");
        let lowered = lower_module(module, &root, &Prelude::none(), &[], relative.as_path());

        let tree = lowered.item_tree;
        let builtins = Builtins::new(
            class(&tree, "Int"),
            class(&tree, "Unit"),
            class(&tree, "String"),
            class(&tree, "Bool"),
        );
        let resolution = Resolution::new(
            Arc::new(ModuleScope::default()),
            BTreeMap::new(),
            Arc::from(Vec::new()),
        );
        let graph = Arc::new(ProjectGraph::default());
        let closure = Closure::of(module, &tree, &graph, &BTreeMap::new(), &mut |_| None);
        let deps = CheckDeps::new(builtins)
            .with_graph(graph)
            .with_closure(closure);

        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(module, Arc::new(types));

        let bodies = lowered
            .bodies
            .iter()
            .filter_map(|decl| lower_hir(&tree, &decl.decl).map(|body| (decl.owner.clone(), body)))
            .collect();

        Fixture {
            tree,
            resolution,
            deps,
            bodies,
        }
    }

    /// The class of a module under a name.
    fn class(tree: &ItemTree, name: &str) -> EntityLoc<ClassLoc> {
        let (item, _) = tree
            .entities()
            .find(|(item, _)| item.name() == Some(&Name::new(name)))
            .expect("the class to be declared");

        EntityLoc {
            module: tree.module(),
            item: ClassLoc::try_from(item).expect("a class"),
        }
    }

    /// Checks the first body of `source`, lowers it, and answers the MIR and the SSA form of it.
    fn lower(source: &str) -> (MirBody, MirBody) {
        let fixture = fixture(source);
        let (owner, body) = fixture.bodies.first().expect("a body of the fixture");
        let (checked, diagnostics) = check_body(
            owner.clone(),
            &fixture.tree,
            &body.body,
            &fixture.resolution,
            &fixture.deps,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let mir = lower_body(
            owner.clone(),
            &fixture.tree,
            &body.body,
            &body.source_map,
            &checked,
            &fixture.resolution,
            &fixture.deps,
        );
        let ssa = construct_ssa(&mir);

        (mir, ssa)
    }

    #[test]
    fn an_if_may_be_written_in_a_condition() {
        let source = format!(
            "{CLASSES}fun pick(a: Bool, b: Bool, c: Bool): Int = \
             if (if a then b else c) then 1 else 2\n"
        );
        let (mir, ssa) = lower(&source);

        assert_eq!(mir.validate_cfg(), Ok(()));

        // The condition is a choice of its own: the walk of the condition leaves control in the
        // block the arms of it meet in, and the branch that reads it is written there --- the
        // block of the outer condition is the join of the inner one.
        assert_eq!(
            dump_of(&mir),
            "fun pick (entry b0)\n  params: v0: Bool, v1: Bool, v2: Bool\n  b0:\n    l0(a) = use v0\n    l1(b) = use v1\n    l2(c) = use v2\n    l5 = use l0\n    branch l5 -> b4, b5\n  b1:\n    l8 = const 1\n    l3 = use l8\n    goto b3\n  b2:\n    l9 = const 2\n    l3 = use l9\n    goto b3\n  b3:\n    return l3\n  b4:\n    l6 = use l1\n    l4 = use l6\n    goto b6\n  b5:\n    l7 = use l2\n    l4 = use l7\n    goto b6\n  b6:\n    branch l4 -> b1, b2\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun pick (entry b0)\n  params: v0: Bool, v1: Bool, v2: Bool\n  b0:\n    branch v0 -> b4, b5\n  b1:\n    v5 = const 1\n    goto b3(v5)\n  b2:\n    v4 = const 2\n    goto b3(v4)\n  b3(v6):\n    return v6\n  b4:\n    goto b6(v1)\n  b5:\n    goto b6(v2)\n  b6(v3):\n    branch v3 -> b1, b2\n",
        );
    }

    #[test]
    fn an_if_without_an_else_selects_the_unit_it_is() {
        let source = format!("{CLASSES}fun log(flag: Bool): Unit = if flag then log(flag)\n");
        let (mir, ssa) = lower(&source);

        assert_eq!(mir.validate_cfg(), Ok(()));

        // The block control falls into when no condition holds writes the unit the expression
        // is, so that what is written after the `if` reads one slot however many arms the
        // choice has. The block is written like any other arm of the choice.
        assert_eq!(
            dump_of(&mir),
            "fun log (entry b0)\n  params: v0: Bool\n  b0:\n    l0(flag) = use v0\n    l2 = use l0\n    branch l2 -> b1, b2\n  b1:\n    l4 = use l0\n    l3 = call fun log(l4)\n    l1 = use l3\n    goto b3\n  b2:\n    l1 = const unit\n    goto b3\n  b3:\n    return l1\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun log (entry b0)\n  params: v0: Bool\n  b0:\n    branch v0 -> b1, b2\n  b1:\n    v2 = call fun log(v0)\n    goto b3(v2)\n  b2:\n    v1 = const unit\n    goto b3(v1)\n  b3(v3):\n    return v3\n",
        );
    }

    #[test]
    fn an_if_branches_and_writes_its_slot() {
        let source = format!("{CLASSES}fun pick(flag: Bool): Int = if flag then 1 else 2\n");
        let (mir, ssa) = lower(&source);

        assert_eq!(mir.validate_cfg(), Ok(()));

        // The condition is evaluated in the entry block, which branches; every arm writes the
        // slot of the `if` and goes to the block the arms meet in, and what is written after
        // the `if` is written there.
        assert_eq!(
            dump_of(&mir),
            "fun pick (entry b0)\n  params: v0: Bool\n  b0:\n    l0(flag) = use v0\n    l2 = use l0\n    branch l2 -> b1, b2\n  b1:\n    l3 = const 1\n    l1 = use l3\n    goto b3\n  b2:\n    l4 = const 2\n    l1 = use l4\n    goto b3\n  b3:\n    return l1\n",
        );

        // The value the arms agree on is born at the join: the SSA form gives it a parameter of
        // the block the arms meet in, and each arm passes its own value.
        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun pick (entry b0)\n  params: v0: Bool\n  b0:\n    branch v0 -> b1, b2\n  b1:\n    v2 = const 1\n    goto b3(v2)\n  b2:\n    v1 = const 2\n    goto b3(v1)\n  b3(v3):\n    return v3\n",
        );
    }

    #[test]
    fn an_elif_chain_is_a_chain_of_branches_that_meet_at_one_block() {
        let source = format!(
            "{CLASSES}fun pick(low: Bool, high: Bool): Int = \
             if low then 1 elif high then 2 else 3\n"
        );
        let (mir, ssa) = lower(&source);

        assert_eq!(mir.validate_cfg(), Ok(()));

        // The `elif` is a condition of its own: the arm before it fails into the block that
        // evaluates it, and the block branches to the arm the condition selects and to the
        // expression selected when nothing holds.
        assert_eq!(
            dump_of(&mir),
            "fun pick (entry b0)\n  params: v0: Bool, v1: Bool\n  b0:\n    l0(low) = use v0\n    l1(high) = use v1\n    l3 = use l0\n    branch l3 -> b1, b2\n  b1:\n    l5 = const 1\n    l2 = use l5\n    goto b5\n  b2:\n    l4 = use l1\n    branch l4 -> b3, b4\n  b3:\n    l6 = const 2\n    l2 = use l6\n    goto b5\n  b4:\n    l7 = const 3\n    l2 = use l7\n    goto b5\n  b5:\n    return l2\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun pick (entry b0)\n  params: v0: Bool, v1: Bool\n  b0:\n    branch v0 -> b1, b2\n  b1:\n    v4 = const 1\n    goto b5(v4)\n  b2:\n    branch v1 -> b3, b4\n  b3:\n    v3 = const 2\n    goto b5(v3)\n  b4:\n    v2 = const 3\n    goto b5(v2)\n  b5(v5):\n    return v5\n",
        );
    }

    #[test]
    fn an_if_may_be_read_where_a_value_belongs() {
        let source = format!("{CLASSES}fun pick(flag: Bool): Int = (if flag then 1 else 2) + 3\n");
        let (mir, ssa) = lower(&source);

        assert_eq!(mir.validate_cfg(), Ok(()));

        // What is written after the `if` is written in the block the arms meet in: the sum is
        // computed there, over the slot the arms wrote.
        assert_eq!(
            dump_of(&mir),
            "fun pick (entry b0)\n  params: v0: Bool\n  b0:\n    l0(flag) = use v0\n    l3 = use l0\n    branch l3 -> b1, b2\n  b1:\n    l4 = const 1\n    l2 = use l4\n    goto b3\n  b2:\n    l5 = const 2\n    l2 = use l5\n    goto b3\n  b3:\n    l6 = const 3\n    l1 = prim int-add(l2, l6)\n    return l1\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun pick (entry b0)\n  params: v0: Bool\n  b0:\n    branch v0 -> b1, b2\n  b1:\n    v2 = const 1\n    goto b3(v2)\n  b2:\n    v1 = const 2\n    goto b3(v1)\n  b3(v4):\n    v3 = const 3\n    v5 = prim int-add(v4, v3)\n    return v5\n",
        );
    }

    #[test]
    fn an_if_may_be_written_inside_an_arm() {
        let source = format!(
            "{CLASSES}fun pick(flag: Bool): Int =\n    if flag then\n        let x = 1 in\n        x\n    else\n        if flag then 2 else 3\n"
        );
        let (mir, ssa) = lower(&source);

        // The arm holds a `let` and an `if` of its own: what the arm writes is written where
        // the walk of the arm ended, and the join of the arm's own `if` that is.
        assert_eq!(mir.validate_cfg(), Ok(()));
        assert_eq!(ssa.validate_ssa(), Ok(()));

        assert_eq!(
            dump_of(&ssa),
            "fun pick (entry b0)\n  params: v0: Bool\n  b0:\n    branch v0 -> b1, b2\n  b1:\n    v4 = const 1\n    goto b3(v4)\n  b2:\n    branch v0 -> b4, b5\n  b3(v5):\n    return v5\n  b4:\n    v2 = const 2\n    goto b6(v2)\n  b5:\n    v1 = const 3\n    goto b6(v1)\n  b6(v3):\n    goto b3(v3)\n",
        );
    }

    #[test]
    fn a_truth_value_lowers_into_a_constant() {
        let source = format!("{CLASSES}fun both(): Bool = true && false\n");
        let (mir, ssa) = lower(&source);

        // A truth value is a word like any other: it lowers into the constant it is, and the
        // operator over two of them is the word-level one.
        assert_eq!(mir.validate_cfg(), Ok(()));
        assert_eq!(
            dump_of(&mir),
            "fun both (entry b0)\n  b0:\n    l1 = const true\n    l2 = const false\n    l0 = prim bool-and(l1, l2)\n    return l0\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun both (entry b0)\n  b0:\n    v0 = const true\n    v1 = const false\n    v2 = prim bool-and(v0, v1)\n    return v2\n",
        );
    }

    #[test]
    fn a_body_of_one_expression_lowers_into_slots() {
        let source = format!("{CLASSES}fun double(value: Int): Int = value + value\n");
        let (mir, ssa) = lower(&source);

        assert_eq!(mir.validate_cfg(), Ok(()));
        assert_eq!(
            dump_of(&mir),
            "fun double (entry b0)\n  params: v0: Int\n  b0:\n    l0(value) = use v0\n    l2 = use l0\n    l3 = use l0\n    l1 = prim int-add(l2, l3)\n    return l1\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun double (entry b0)\n  params: v0: Int\n  b0:\n    v1 = prim int-add(v0, v0)\n    return v1\n",
        );
    }

    #[test]
    fn a_let_binds_a_slot_for_the_scope_of_its_body() {
        let source = format!("{CLASSES}fun identity(value: Int): Int = let x = value in x\n");
        let (mir, _) = lower(&source);

        assert_eq!(
            dump_of(&mir),
            "fun identity (entry b0)\n  params: v0: Int\n  b0:\n    l0(value) = use v0\n    l2 = use l0\n    l3(x) = use l2\n    l4 = use l3\n    l1 = use l4\n    return l1\n",
        );
    }

    #[test]
    fn a_body_the_check_rejected_is_a_bug_and_not_a_diagnostic() {
        let source = format!("{CLASSES}fun big(): Int = 1099511627776\n");
        let fixture = fixture(&source);
        let (owner, body) = fixture.bodies.first().expect("a body of the fixture");
        let (checked, diagnostics) = check_body(
            owner.clone(),
            &fixture.tree,
            &body.body,
            &fixture.resolution,
            &fixture.deps,
        );
        assert_eq!(diagnostics.len(), 1, "the check reports the literal");

        // The driver is what keeps a body the check rejected away from the lowering; a caller
        // that hands one over breaks the contract, and the pass says so with an internal compiler
        // exception a host can read --- never with a diagnostic of its own.
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            lower_body(
                owner.clone(),
                &fixture.tree,
                &body.body,
                &body.source_map,
                &checked,
                &fixture.resolution,
                &fixture.deps,
            )
        }))
        .expect_err("the lowering to be an exception");

        let ice = payload
            .downcast_ref::<Ice>()
            .expect("the payload to be an exception");

        assert!(ice.message().contains("1099511627776"), "{ice}");
    }

    /// The dump of a body, which the tests read the lowering and the SSA form by.
    fn dump_of(body: &MirBody) -> String {
        mlkc_mir::dump::body(body)
    }
}
