//! The lowering of a checked body into the CFG form of MIR.
//!
//! The lowering is one walk of the HIR body in post-order, one slot per expression: the
//! operands of an expression are read from the slots of its children, and the result is
//! assigned to the slot of the expression ([ADR-0019][adr-0019]). What the walk reads of the
//! checker is the type of every node --- which is what picks a `PrimOp` and what a `ValueData`
//! remembers --- and what it reads of the resolution is a callee: the path of a call is
//! resolved the way the check resolved it, and never by walking a module table here.
//!
//! [adr-0019]: ../../docs/adr/0019-mir.md
//!
//! A body whose check reported a mistake is not lowered: a `Ty::Error` expression has no
//! meaning to lower, and the driver is what asks for MIR only when the body checks clean. The
//! walk is total all the same: a node it cannot read is a [`MirDiag`], not a panic.

use mlkc_hir_def::{
    BinaryOp, Body, BodyEntityLoc, ClassLoc, EntityLoc, Expr, ExprId, FunctionLoc, ItemTree,
    Literal, LocalDefId, ModuleId, Name, Namespace, Pat, PatId, PathAnchor, PathData, UnaryOp,
};
use mlkc_hir_ty::{CheckedBody, Ty};
use mlkc_la_arena::ArenaMap;
use mlkc_lower::BodySourceMap;
use mlkc_mir::{
    Block, Body as MirBody, BodyBuilder, Callee, Const, LocalData, LocalId, Operand, Place, PrimOp,
    Rvalue, Stmt, StmtKind, Terminator, ValueData,
};
use mlkc_resolve::{Resolution, Walk};
use mlkc_span::Span;
use mlkc_typeck::CheckDeps;

use crate::diagnostic::{MirDiag, MirError, Operator};

/// The smallest and the largest `Int`, as [ADR-0018][adr-0018] fixes them.
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
const INT_MIN: i64 = -(1 << 30);
const INT_MAX: i64 = (1 << 30) - 1;

/// Lowers the checked body of an entity into the CFG form of MIR.
///
/// `tree` is the surface of the module the body is of: the lowering checks that the entity and
/// the body are of one module, and reads nothing else of it. `source_map` is where the nodes of
/// the HIR body are written ([`BodySourceMap`]), `checked` is what the check of the body left
/// behind, and `resolution` and `deps` are what the check read: a path of a call is resolved
/// with the same walk the check used ([`Walk::entity_of`]), so the two read it the same way
/// ([ADR-0019][adr-0019]).
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
) -> (MirBody, Vec<MirDiag>) {
    debug_assert_eq!(
        owner.module(),
        tree.module(),
        "a body is of an entity of the tree it is lowered against",
    );

    let module = owner.module();
    let walk = Walk::of(deps.graph(), deps.closure());

    let lowerer = Lowerer {
        module,
        body,
        source_map,
        checked,
        deps,
        resolution,
        walk,
        builder: BodyBuilder::new(owner.clone()),
        slots: ArenaMap::default(),
        pat_slots: ArenaMap::default(),
        stmts: Vec::new(),
        diags: Vec::new(),
    };

    lowerer.run()
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
    /// The statements of the entry block, in the order they were lowered.
    stmts: Vec<Stmt>,
    /// What the lowering found.
    diags: Vec<MirDiag>,
}

impl Lowerer<'_> {
    /// Lowers the body and finishes it.
    fn run(mut self) -> (MirBody, Vec<MirDiag>) {
        for pat in self.body.params() {
            self.parameter(*pat);
        }

        let root = self.expr(self.body.root());
        let span = self.span_expr(self.body.root());
        let stmts = std::mem::take(&mut self.stmts);
        let entry = self.builder.block(Block {
            params: Vec::new(),
            stmts,
            term: Terminator::Return {
                value: Operand::Local(root),
                span,
            },
        });

        let body = self.builder.finish(entry);

        (body, self.diags)
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
        let rvalue = self.rvalue(&node, span);

        self.assign(Place::Local(slot), rvalue, span);
        self.slots.insert(expr, slot);

        slot
    }

    /// Lowers one node of the HIR.
    fn rvalue(&mut self, node: &Expr, span: Span) -> Rvalue {
        match node {
            Expr::Missing => {
                self.report(MirError::Missing, span);
                Rvalue::Const(Const::Unit)
            },
            Expr::Path(path) => {
                let path = self.body[*path].clone();
                self.value_path(&path, span)
            },
            Expr::Literal(Literal::Int(value)) => {
                if (INT_MIN..=INT_MAX).contains(value) {
                    Rvalue::Const(Const::Int(*value as i32))
                } else {
                    self.report(MirError::IntOutOfRange { value: *value }, span);
                    Rvalue::Const(Const::Int(0))
                }
            },
            Expr::Literal(Literal::Str(value)) => Rvalue::Const(Const::Str(value.clone())),
            Expr::Call { callee, args } => {
                let mut lowered = Vec::with_capacity(args.len());

                for arg in args {
                    lowered.push(Operand::Local(self.expr(*arg)));
                }

                let callee = self.callee(*callee, span);

                Rvalue::Call {
                    callee,
                    args: lowered,
                }
            },
            Expr::Field { field, .. } => {
                self.report(
                    MirError::Field {
                        name: field.clone(),
                    },
                    span,
                );

                Rvalue::Const(Const::Unit)
            },
            Expr::Binary { lhs, op, rhs } => self.binary(*lhs, *op, *rhs, span),
            Expr::Unary { op, operand } => self.unary(*op, *operand, span),
            Expr::Let { pat, expr, body } => {
                let bound = self.expr(*expr);
                self.bind(*pat, bound);
                let value = self.expr(*body);

                Rvalue::Use(Operand::Local(value))
            },
        }
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
            Pat::Missing => self.report(MirError::Missing, span),
        }
    }

    /// Lowers a path written where a value belongs.
    fn value_path(&mut self, path: &PathData, span: Span) -> Rvalue {
        match path.anchor.clone() {
            // A binding of the body: the slot the binding was given.
            PathAnchor::Binding(pat) => {
                match self.pat_slots.get(pat) {
                    Some(slot) => Rvalue::Use(Operand::Local(*slot)),
                    None => {
                        self.report(MirError::Unresolved { name: path.name() }, span);

                        Rvalue::Const(Const::Unit)
                    },
                }
            },
            // An entity declared inside the body, and a type variable: not values the language
            // has, and a name of another namespace is what the check reported.
            PathAnchor::TypeVar(_) | PathAnchor::Local(_) => {
                self.report(MirError::NotAValue { name: path.name() }, span);

                Rvalue::Const(Const::Unit)
            },
            // A name of the project: what the check resolved it to, read the same way here.
            PathAnchor::Item(_) | PathAnchor::Use(_) | PathAnchor::Project(_) => {
                let (entity, _) =
                    self.walk
                        .entity_of(self.module, self.resolution, path, Namespace::Value);

                match entity {
                    // A function, a constant, or a class written where a value belongs: the
                    // language has no value of an entity yet.
                    Some(_) => {
                        self.report(MirError::NotAValue { name: path.name() }, span);
                    },
                    None => {
                        self.report(MirError::Unresolved { name: path.name() }, span);
                    },
                }

                Rvalue::Const(Const::Unit)
            },
            PathAnchor::Unresolved => {
                self.report(MirError::Unresolved { name: path.name() }, span);

                Rvalue::Const(Const::Unit)
            },
        }
    }

    /// Resolves the callee of a call.
    fn callee(&mut self, expr: ExprId, span: Span) -> Callee {
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
                        self.report(MirError::Unresolved { name: path.name() }, span);

                        Callee::Indirect(Operand::Const(Const::Unit))
                    },
                }
            },
            PathAnchor::Local(LocalDefId::Function(local)) => Callee::Local(local),
            PathAnchor::Local(LocalDefId::Const(_)) => {
                self.report(MirError::NotCallable { name: path.name() }, span);

                Callee::Indirect(Operand::Const(Const::Unit))
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
                                self.report(MirError::NotCallable { name: path.name() }, span);

                                Callee::Indirect(Operand::Const(Const::Unit))
                            },
                        }
                    },
                    None => {
                        self.report(MirError::Unresolved { name: path.name() }, span);

                        Callee::Indirect(Operand::Const(Const::Unit))
                    },
                }
            },
        }
    }

    /// Lowers a binary operation into a primitive.
    fn binary(&mut self, lhs: ExprId, op: BinaryOp, rhs: ExprId, span: Span) -> Rvalue {
        let left = self.expr(lhs);
        let right = self.expr(rhs);
        let args = vec![Operand::Local(left), Operand::Local(right)];

        let (Some(lhs_ty), Some(rhs_ty)) = (
            self.checked.expr_type(lhs).cloned(),
            self.checked.expr_type(rhs).cloned(),
        ) else {
            self.report(MirError::MissingType, span);
            return Rvalue::Use(Operand::Local(left));
        };

        match self.binary_prim(op, &lhs_ty, &rhs_ty, span) {
            Some(op) => Rvalue::Prim { op, args },
            // A mistake was reported; the copy keeps the body in one piece.
            None => Rvalue::Use(Operand::Local(left)),
        }
    }

    /// The primitive of a binary operator, if the language has one for these operands.
    fn binary_prim(&mut self, op: BinaryOp, lhs: &Ty, rhs: &Ty, span: Span) -> Option<PrimOp> {
        let int = self.deps.builtins().int().clone();
        let boolean = self.deps.builtins().boolean().clone();
        let ints = is_class(lhs, &int) && is_class(rhs, &int);
        let booleans = is_class(lhs, &boolean) && is_class(rhs, &boolean);

        let prim = match op {
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
            BinaryOp::Eq | BinaryOp::Ne => {
                self.report(MirError::Equality { ty: lhs.clone() }, span);
                return None;
            },
            op => {
                self.report(
                    MirError::Operator {
                        op: Operator::Binary(op),
                    },
                    span,
                );

                return None;
            },
        };

        Some(prim)
    }

    /// Lowers a unary operation.
    fn unary(&mut self, op: UnaryOp, operand: ExprId, span: Span) -> Rvalue {
        let value = self.expr(operand);

        match op {
            // A sign that keeps the value is the value.
            UnaryOp::Pos => Rvalue::Use(Operand::Local(value)),
            UnaryOp::Neg => {
                let Some(ty) = self.checked.expr_type(operand).cloned() else {
                    self.report(MirError::MissingType, span);
                    return Rvalue::Use(Operand::Local(value));
                };

                if is_class(&ty, self.deps.builtins().int()) {
                    Rvalue::Prim {
                        op: PrimOp::IntNeg,
                        args: vec![Operand::Local(value)],
                    }
                } else {
                    self.report(
                        MirError::Operator {
                            op: Operator::Unary(op),
                        },
                        span,
                    );

                    Rvalue::Use(Operand::Local(value))
                }
            },
        }
    }

    /// Allocates a slot.
    fn slot(&mut self, span: Span, name: Option<Name>, ty: Ty) -> LocalId {
        self.builder.local(LocalData { span, name, ty })
    }

    /// Writes one assignment into the entry block.
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

    /// Reports what the lowering cannot lower.
    fn report(&mut self, error: MirError, span: Span) {
        self.diags.push(MirDiag::new(error, span));
    }
}

/// Whether a type is a class that takes no arguments, and that class.
fn is_class(ty: &Ty, class: &EntityLoc<ClassLoc>) -> bool {
    matches!(ty, Ty::Class { class: found, args } if args.is_empty() && found == class)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

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

    /// Checks the first body of `source`, lowers it, and answers the MIR, the diagnostics of
    /// the lowering, and the SSA form of it.
    fn lower(source: &str) -> (MirBody, Vec<MirDiag>, MirBody) {
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

        let (mir, diagnostics) = lower_body(
            owner.clone(),
            &fixture.tree,
            &body.body,
            &body.source_map,
            &checked,
            &fixture.resolution,
            &fixture.deps,
        );
        let ssa = construct_ssa(&mir);

        (mir, diagnostics, ssa)
    }

    #[test]
    fn a_body_of_one_expression_lowers_into_slots() {
        let source = format!("{CLASSES}fun double(value: Int): Int = value + value\n");
        let (mir, diagnostics, ssa) = lower(&source);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(mir.validate_cfg(), Ok(()));
        assert_eq!(
            dump_of(&mir),
            "fun double (entry b0)\n  params: v0: Int\n  b0:\n    l0(value) = use v0\n    l2 = use l0\n    l3 = use l0\n    l1 = prim int-add(l2, l3)\n    return l1\n",
        );

        assert_eq!(ssa.validate_ssa(), Ok(()));
        assert_eq!(
            dump_of(&ssa),
            "fun double (entry b0)\n  params: v0: Int\n  b0:\n    v1 = use v0\n    v2 = use v1\n    v3 = use v1\n    v4 = prim int-add(v2, v3)\n    return v4\n",
        );
    }

    #[test]
    fn a_let_binds_a_slot_for_the_scope_of_its_body() {
        let source = format!("{CLASSES}fun identity(value: Int): Int = let x = value in x\n");
        let (mir, diagnostics, _) = lower(&source);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            dump_of(&mir),
            "fun identity (entry b0)\n  params: v0: Int\n  b0:\n    l0(value) = use v0\n    l2 = use l0\n    l3(x) = use l2\n    l4 = use l3\n    l1 = use l4\n    return l1\n",
        );
    }

    #[test]
    fn an_int_out_of_the_range_of_the_word_is_reported() {
        let source = format!("{CLASSES}fun big(): Int = 1099511627776\n");
        let (mir, diagnostics, _) = lower(&source);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].error(), &MirError::IntOutOfRange {
            value: 1_099_511_627_776,
        },);
        assert_eq!(mir.validate_cfg(), Ok(()));
        assert_eq!(
            dump_of(&mir),
            "fun big (entry b0)\n  b0:\n    l0 = const 0\n    return l0\n",
        );
    }

    /// The dump of a body, which the tests read the lowering and the SSA form by.
    fn dump_of(body: &MirBody) -> String {
        mlkc_mir::dump::body(body)
    }
}
