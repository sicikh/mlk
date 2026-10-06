//! The check of one body: bidirectional Hindley–Milner, one body at a time.
//!
//! `infer` computes the type of an expression nothing fixes the type of; `check` verifies an
//! expression where something does. Both record what they found, and nothing of the engine
//! reaches storage: a variable is resolved, generalized into a parameter, or reported
//! ([ADR-0017]).
//!
//! A body reads the surface of its own module --- its own signature, and the signatures of the
//! functions it calls --- and the surfaces of the modules its paths name, which the driver
//! assembles into a [`CheckDeps`] together with the closure of the check and the classes of the
//! language. A path that names a module, a module of a project above all, is walked over that
//! closure ([ADR-0016]); the classes of the language are handed in as
//! [`Builtins`](mlkc_hir_ty::Builtins), and the check knows no primitive:
//! `Int` and `String` are classes like any other.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md

use mlkc_hir_def::{
    BinaryOp, Body, BodyEntityLoc, EntityLoc, Expr, ExprId, IfArm, ItemKind, ItemLoc, ItemLocLike,
    ItemTree, LambdaParam, Literal, LocalDefId, LocalFunctionId, Name, Namespace, Pat, PatId,
    PathAnchor, PathId, TypeRef, UnaryOp,
};
use mlkc_hir_ty::{CheckDeps, CheckedBody, INT_MAX, INT_MIN, Ty};
use mlkc_resolve::Resolution;
use mlkc_stdx::FxIndexMap;
use rustc_hash::FxHashMap;

use crate::{
    diagnostic::{TypeDiag, TypeError, TypePlace},
    engine::{Engine, InferTy, Scheme, UnifyError},
    resolve::PathResolver,
};

/// Checks one body against the signatures its module wrote.
///
/// `owner` is the entity the body belongs to, and its signature is read from the surface of the
/// module the check is given in `deps`. The body's own module's surface must be there: the types
/// of a module are resolved before its bodies are checked, and a missing surface is reported as
/// [`TypeError::MissingSignature`]. `resolution` is what the names of the module denote, and what
/// each of its imports resolved to.
pub fn check_body(
    owner: BodyEntityLoc,
    tree: &ItemTree,
    body: &Body,
    resolution: &Resolution,
    deps: &CheckDeps,
) -> (CheckedBody, Vec<TypeDiag>) {
    let module = owner.module();
    let mut checker = Checker {
        engine: Engine::new(EntityLoc::from(owner.clone())),
        owner,
        tree,
        body,
        resolver: PathResolver::new(module, resolution, deps),
        deps,
        bindings: FxHashMap::default(),
        local_types: FxIndexMap::default(),
        expr_types: FxIndexMap::default(),
        pat_types: FxIndexMap::default(),
        diagnostics: Vec::new(),
    };
    checker.run();

    checker.finish()
}

/// Which position an expression is read in.
///
/// The name of a function is not a value the language has: what a call calls is a name, and
/// a lambda is how a value of a function is written. A name read anywhere else denotes no
/// value ([ADR-0018]).
///
/// [ADR-0018]: ../../docs/adr/0018-values-as-words.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    /// Where a value belongs.
    Value,
    /// The callee of a call.
    Callee,
}

/// What a builtin expression needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Builtin {
    /// `Int`: the type of an integer literal and of an arithmetic operator.
    Int,
    /// `String`: the type of a string literal.
    String,
    /// `Bool`: the type of a comparison and of a logical operator.
    Boolean,
    /// `Unit`: the type of a choice that selects no value, and of a body that gives none.
    Unit,
}

/// The check of one body.
struct Checker<'a> {
    /// The entity the body belongs to, as the HIR names it.
    owner: BodyEntityLoc,
    /// The inference engine of this check.
    engine: Engine,
    /// The surface of the module the body belongs to.
    tree: &'a ItemTree,
    /// The body.
    body: &'a Body,
    /// What resolves the paths of the module, and of the body.
    resolver: PathResolver<'a>,
    /// What the check reads of the rest of the project.
    deps: &'a CheckDeps,
    /// What the patterns of the body are bound to, generalized where a `let` generalized them.
    bindings: FxHashMap<PatId, Scheme>,
    /// The type of every function declared inside the body, in the order they are checked.
    local_types: FxIndexMap<LocalDefId, InferTy>,
    /// The type of every expression checked so far.
    expr_types: FxIndexMap<ExprId, InferTy>,
    /// The type of every pattern checked so far.
    pat_types: FxIndexMap<PatId, InferTy>,
    /// What the check found, in the order of the body.
    diagnostics: Vec<TypeDiag>,
}

impl Checker<'_> {
    /// Checks the body: its parameters against the signature, and its root against the result.
    fn run(&mut self) {
        let entity = EntityLoc::from(self.owner.clone());
        let signature = self
            .deps
            .types(self.owner.module())
            .and_then(|types| types.get(&entity))
            .cloned();

        let (root, expected) = match signature {
            Some(Ty::Fn { params, ret }) => {
                let pats = self.body.params();

                if pats.len() != params.len() {
                    self.report_at(
                        TypeError::ParameterCount {
                            expected: params.len(),
                            found: pats.len(),
                        },
                        TypePlace::Entity(ItemLoc::from(self.owner.item.clone())),
                    );
                }

                for (index, pat) in pats.iter().enumerate() {
                    let ty = params.get(index).map_or(InferTy::Error, InferTy::of);
                    let scheme = Scheme {
                        ty,
                        params: Vec::new(),
                    };
                    self.bind_pat(*pat, &scheme);
                }

                (self.body.root(), InferTy::of(&ret))
            },
            // A body that is not a function, a constant above all: checked against its type.
            Some(ty) => (self.body.root(), InferTy::of(&ty)),
            None => {
                self.report_at(
                    TypeError::MissingSignature,
                    TypePlace::Entity(ItemLoc::from(self.owner.item.clone())),
                );

                (self.body.root(), InferTy::Error)
            },
        };

        self.check(root, &expected);
    }

    /// The type of an expression, computed where nothing fixes it.
    fn infer(&mut self, expr: ExprId) -> InferTy {
        let node = self.body[expr].clone();

        let ty = match node {
            Expr::Missing => InferTy::Error,
            Expr::Literal(literal) => self.literal(expr, &literal),
            Expr::Path(path) => self.value_path(expr, path, Position::Value),
            Expr::Call { callee, args } => self.call(callee, &args),
            Expr::Field { field, .. } => {
                // The names inside a value are not a thing the language has yet.
                self.report(TypeError::NestedName { name: field }, expr);
                InferTy::Error
            },
            Expr::Binary { lhs, op, rhs } => self.binary(expr, lhs, op, rhs),
            Expr::Unary { op, operand } => self.unary(op, operand),
            Expr::Let {
                pat,
                expr: init,
                body,
            } => self.let_binding(pat, init, body),
            Expr::If {
                cond,
                then_,
                arms,
                otherwise,
            } => self.if_expr(cond, then_, &arms, otherwise),
            Expr::Lambda { params, body, .. } => self.lambda(&params, body),
            Expr::Local { items, body } => self.local_expr(expr, &items, body),
        };

        self.record_expr(expr, ty.clone());
        ty
    }

    /// Checks an expression where something fixes its type.
    ///
    /// A `let` propagates the expectation into its own body; everything else is inferred and
    /// then unified with what the place expects.
    fn check(&mut self, expr: ExprId, expected: &InferTy) {
        if let Expr::Let {
            pat,
            expr: init,
            body,
        } = self.body[expr].clone()
        {
            self.engine.enter();
            let init_ty = self.infer(init);
            self.engine.leave();

            let scheme = self.engine.generalize(&init_ty);
            self.bind_pat(pat, &scheme);
            self.check(body, expected);
            self.record_expr(expr, expected.clone());

            return;
        }

        // An `if` propagates the expectation into every branch of it: the branches are the
        // expressions the place expects a value from, and a mistake is reported where the
        // branch that is one is written. An `if` without an `else` is a `Unit` whatever the
        // place expects: what its arms are is checked against `Unit`, and the whole of it
        // against the place.
        if let Expr::If {
            cond,
            then_,
            arms,
            otherwise,
        } = self.body[expr].clone()
        {
            let boolean = self.builtin(Builtin::Boolean);
            self.check(cond, &boolean);

            if let Some(otherwise) = otherwise {
                self.check(then_, expected);

                for arm in &arms {
                    self.check(arm.cond, &boolean);
                    self.check(arm.body, expected);
                }

                self.check(otherwise, expected);
                self.record_expr(expr, expected.clone());
            } else {
                let unit = self.builtin(Builtin::Unit);
                self.check(then_, &unit);

                for arm in &arms {
                    self.check(arm.cond, &boolean);
                    self.check(arm.body, &unit);
                }

                let expected = self.engine.repr(expected);

                match self.engine.unify(&expected, &unit) {
                    Ok(()) => {
                        let recorded = if expected.is_error() { unit } else { expected };
                        self.record_expr(expr, recorded);
                    },
                    Err(error) => {
                        self.report_unify(error, expr);
                        self.record_expr(expr, InferTy::Error);
                    },
                }
            }

            return;
        }

        let found = self.infer(expr);
        let expected = self.engine.repr(expected);

        match self.engine.unify(&expected, &found) {
            Ok(()) => {
                // What is recorded is what the expression is: the expectation, unless it is a
                // mistake and the expression is something more specific.
                let recorded = if expected.is_error() { found } else { expected };
                self.record_expr(expr, recorded);
            },
            Err(error) => {
                self.report_unify(error, expr);
                self.record_expr(expr, InferTy::Error);
            },
        }
    }

    /// The type of a literal: an integer is `Int`, a truth value is `Bool`, and a string is
    /// `String`.
    ///
    /// An integer literal that does not fit the 31-bit representation of `Int` is the check's to
    /// report: the representation is what the type means ([ADR-0018]).
    ///
    /// [ADR-0018]: ../../docs/adr/0018-values-as-words.md
    fn literal(&mut self, expr: ExprId, literal: &Literal) -> InferTy {
        match literal {
            Literal::Int(value) => {
                if !(INT_MIN..=INT_MAX).contains(value) {
                    self.report(TypeError::IntOutOfRange { value: *value }, expr);
                }

                self.builtin(Builtin::Int)
            },
            Literal::Bool(_) => self.builtin(Builtin::Boolean),
            Literal::Str(_) => self.builtin(Builtin::String),
        }
    }

    /// The type of a path written where a value belongs.
    fn value_path(&mut self, expr: ExprId, path: PathId, position: Position) -> InferTy {
        let path = self.body[path].clone();

        // A path applied to arguments is a path of a value the language cannot have yet, and a
        // mistake inside one of them is reported as the mistake it is.
        let (_, mut errors) = self.resolver.arguments_of(&path);

        let ty = match path.anchor.clone() {
            PathAnchor::Binding(pat) => {
                match self.bindings.get(&pat) {
                    Some(scheme) => {
                        let scheme = scheme.clone();
                        self.engine.instantiate(&scheme.ty, &scheme.params)
                    },
                    // A binding the lowering resolved is a binding this check bound; one that is
                    // not is broken input, and errors absorb it.
                    None => InferTy::Error,
                }
            },
            // A function declared inside a body: the type of it where a call reads it, and no
            // value at all, the way the name of a function of a module is ([ADR-0018]).
            //
            // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
            PathAnchor::Local(local) => {
                if position == Position::Value {
                    errors.push(TypeError::NotAValue { name: path.name() });

                    InferTy::Error
                } else {
                    // A function declared inside a body is checked before a call of it is read,
                    // and a name of one the check did not reach is broken input.
                    self.local_types
                        .get(&local)
                        .cloned()
                        .unwrap_or(InferTy::Error)
                }
            },
            // A type variable is a type, and a value belongs here.
            PathAnchor::TypeVar(_) => {
                errors.push(TypeError::NotAValue {
                    name: path.root.name().cloned().unwrap_or_else(Name::missing),
                });
                InferTy::Error
            },
            // A name the lowering could not resolve: a class of the module written where a
            // value belongs, or a name that is not there at all, which the lowering reported.
            PathAnchor::Unresolved => {
                if let Some(name) = path.root.name()
                    && self
                        .tree
                        .scope()
                        .get(name)
                        .is_some_and(|entry| entry.ty.is_some() && entry.value.is_none())
                {
                    errors.push(TypeError::NotAValue { name: name.clone() });
                }

                InferTy::Error
            },
            PathAnchor::Item(_) | PathAnchor::Use(_) | PathAnchor::Project(_) => {
                let (entity, mut found) = self.resolver.entity_of(&path, Namespace::Value);
                errors.append(&mut found);

                match entity {
                    // The name of a function is what a call calls, and not a value the language
                    // has: a lambda is how a value of a function is written.
                    Some(entity)
                        if position == Position::Value
                            && entity.item.kind() == ItemKind::Function =>
                    {
                        errors.push(TypeError::NotAValue { name: path.name() });

                        InferTy::Error
                    },
                    Some(entity) => self.entity_value(&entity),
                    None => InferTy::Error,
                }
            },
        };

        for error in errors {
            self.report(error, expr);
        }

        ty
    }

    /// The type of the entity a path denotes, with the parameters of the entity replaced by
    /// fresh variables.
    fn entity_value(&mut self, entity: &EntityLoc) -> InferTy {
        let ty = self
            .deps
            .types(entity.module())
            .and_then(|types| types.get(entity))
            .cloned();

        let Some(ty) = ty else {
            // An entity that has no type is the input being incomplete, and the body absorbs
            // it; a name that denotes an entity nothing resolved is what the resolution
            // reported.
            return InferTy::Error;
        };

        let ty = InferTy::of(&ty);
        self.engine.instantiate_owned(&ty, entity)
    }

    /// The type of a call: the callee's parameters check the arguments, and its result is the
    /// type of the call.
    fn call(&mut self, callee: ExprId, args: &[ExprId]) -> InferTy {
        let callee_ty = self.callee_value(callee);
        let (params, ret) = self.function_of(callee, callee_ty, args.len());

        for (index, arg) in args.iter().enumerate() {
            let expected = params.get(index).cloned().unwrap_or(InferTy::Error);
            self.check(*arg, &expected);
        }

        ret
    }

    /// The type of the expression a call calls.
    ///
    /// A path is read as a callee: a function the name denotes is what the call calls, and not
    /// a mistake. Any other expression is a value like any other.
    fn callee_value(&mut self, expr: ExprId) -> InferTy {
        let Expr::Path(path) = self.body[expr].clone() else {
            return self.infer(expr);
        };

        let ty = self.value_path(expr, path, Position::Callee);
        self.record_expr(expr, ty.clone());

        ty
    }

    /// The parameters and the result of a value that is called, invented where the value is a
    /// variable.
    fn function_of(
        &mut self,
        callee: ExprId,
        ty: InferTy,
        arity: usize,
    ) -> (Vec<InferTy>, InferTy) {
        match self.engine.repr(&ty) {
            InferTy::Fn { params, ret } => {
                if params.len() != arity {
                    self.report(
                        TypeError::ArgumentCount {
                            expected: params.len(),
                            found: arity,
                        },
                        callee,
                    );
                }

                (params, *ret)
            },
            // A mistake reported once and does not cascade.
            InferTy::Error => (Vec::new(), InferTy::Error),
            // A variable is called: it is a function of the arguments it is given.
            InferTy::Var(var) => {
                let params: Vec<InferTy> = (0..arity).map(|_| self.engine.fresh_var()).collect();
                let ret = self.engine.fresh_var();
                let function = InferTy::Fn {
                    params: params.clone(),
                    ret: Box::new(ret.clone()),
                };

                // A fresh variable unified with a function of fresh variables cannot fail.
                let _ = self.engine.unify(&InferTy::Var(var), &function);

                (params, ret)
            },
            other => {
                let found = self.engine.zonk(&other);
                self.report(TypeError::NotCallable { found }, callee);

                (Vec::new(), InferTy::Error)
            },
        }
    }

    /// The type of a binary operation.
    ///
    /// Every operator of the language is a rule of the check for now: arithmetic and ordering
    /// are over `Int`, equality compares two values of one type, and `&&` and `||` are over
    /// `Bool`. When `impl`s arrive, the rules become the types of the `impl`s, and no type
    /// changes.
    fn binary(&mut self, expr: ExprId, lhs: ExprId, op: BinaryOp, rhs: ExprId) -> InferTy {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let int = self.builtin(Builtin::Int);
                self.check(lhs, &int);
                self.check(rhs, &int);

                int
            },
            BinaryOp::Eq | BinaryOp::Ne => {
                let lhs_ty = self.infer(lhs);
                self.check(rhs, &lhs_ty);
                self.equality(expr, rhs, &lhs_ty);
                self.builtin(Builtin::Boolean)
            },
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let int = self.builtin(Builtin::Int);
                self.check(lhs, &int);
                self.check(rhs, &int);
                self.builtin(Builtin::Boolean)
            },
            BinaryOp::And | BinaryOp::Or => {
                let boolean = self.builtin(Builtin::Boolean);
                self.check(lhs, &boolean);
                self.check(rhs, &boolean);

                boolean
            },
        }
    }

    /// The type of a prefix sign: a sign applies to an `Int` and is an `Int`.
    fn unary(&mut self, _op: UnaryOp, operand: ExprId) -> InferTy {
        let int = self.builtin(Builtin::Int);
        self.check(operand, &int);

        int
    }

    /// Reports equality over a type the language has no equality for yet.
    ///
    /// Equality is over `Int` and `Bool` for now, and becomes a class of its own when the
    /// language has `impl`s ([ADR-0019]). A mistake absorbs the report: the mistake was reported
    /// where it was made, and one expression is one mistake.
    ///
    /// [ADR-0019]: ../../docs/adr/0019-mir.md
    fn equality(&mut self, expr: ExprId, rhs: ExprId, ty: &InferTy) {
        // The right side is the error type when it did not check against the left one, and the
        // mismatch it reported is the mistake here.
        if matches!(self.expr_types.get(&rhs), Some(InferTy::Error)) {
            return;
        }

        let represented = self.engine.repr(ty);

        let supported = match &represented {
            InferTy::Error | InferTy::Var(_) => return,
            InferTy::Class { class, args } => {
                args.is_empty()
                    && (class == self.deps.builtins().int()
                        || class == self.deps.builtins().boolean())
            },
            InferTy::Fn { .. } | InferTy::Param(_) => false,
        };

        if !supported {
            let ty = self.engine.zonk(&represented);
            self.report(TypeError::NoEquality { ty }, expr);
        }
    }

    /// The type of an `if`: every condition is a `Bool`, and every branch is a value of the
    /// type of the expression.
    ///
    /// Nothing fixes the type here, so the branch the first condition selects is what the arms
    /// after it are checked against: the first branch is the one a reader reads first, and
    /// a mistake is reported at the branch that is one. An `if` without an `else` selects no
    /// value: its arms are `Unit`, and so it is.
    fn if_expr(
        &mut self,
        cond: ExprId,
        then_: ExprId,
        arms: &[IfArm],
        otherwise: Option<ExprId>,
    ) -> InferTy {
        let boolean = self.builtin(Builtin::Boolean);
        self.check(cond, &boolean);

        let Some(otherwise) = otherwise else {
            let unit = self.builtin(Builtin::Unit);
            self.check(then_, &unit);

            for arm in arms {
                self.check(arm.cond, &boolean);
                self.check(arm.body, &unit);
            }

            return unit;
        };

        let ty = self.infer(then_);

        for arm in arms {
            self.check(arm.cond, &boolean);
            self.check(arm.body, &ty);
        }

        self.check(otherwise, &ty);

        ty
    }

    /// The type of a `let`: the right side is inferred, generalized, and bound to the pattern,
    /// and the type of the expression is the type of its body.
    fn let_binding(&mut self, pat: PatId, init: ExprId, body: ExprId) -> InferTy {
        self.engine.enter();
        let init_ty = self.infer(init);
        self.engine.leave();

        let scheme = self.engine.generalize(&init_ty);
        self.bind_pat(pat, &scheme);

        self.infer(body)
    }

    /// The type of a lambda: its parameters, and the type of its body.
    ///
    /// A parameter that writes a type takes it, and one that writes none takes a variable the
    /// body works out. The parameters are bound in a level of their own, and are monomorphic
    /// inside the body: a lambda is the signature of what it computes, and a signature is one
    /// type. What the body leaves unresolved is a variable of the lambda's own level, and the
    /// `let` the lambda is bound in is what generalizes it, if the lambda is bound at all.
    ///
    /// What the lambda captures is not read here: the HIR holds the bindings its body reads,
    /// and the type of a lambda says nothing about the environment it carries ([ADR-0018]).
    ///
    /// [ADR-0018]: ../../docs/adr/0018-values-as-words.md
    fn lambda(&mut self, params: &[LambdaParam], body: ExprId) -> InferTy {
        self.engine.enter();

        let mut types = Vec::with_capacity(params.len());

        for param in params {
            let ty = self.parameter_type(param);

            self.bind_pat(param.pat, &Scheme {
                ty: ty.clone(),
                params: Vec::new(),
            });

            types.push(ty);
        }

        let ret = self.infer(body);
        self.engine.leave();

        InferTy::Fn {
            params: types,
            ret: Box::new(ret),
        }
    }

    /// The type of a `local`: the functions it declares, and the expression their names are
    /// visible in.
    ///
    /// A function declared inside a body is checked with it: the patterns of its parameters are
    /// bound to the types its signature writes, and its root is checked against the type of its
    /// result. What a call of it reads is the type of the function, which is recorded with the
    /// entities declared inside the body ([`CheckedBody::local_types`]).
    ///
    /// The signatures are read before any body: the items see one another, and a call of an item
    /// written after the caller is answered by the signature of it.
    fn local_expr(&mut self, expr: ExprId, items: &[LocalDefId], body: ExprId) -> InferTy {
        let mut declared = Vec::new();

        for item in items {
            // The language declares functions inside a body today; the check of another kind of
            // entity of a body is not written.
            let LocalDefId::Function(id) = item else {
                continue;
            };

            declared.push(self.local_function(expr, *id));
        }

        for function in &declared {
            let params: Vec<InferTy> = function.params.iter().map(|(_, ty)| ty.clone()).collect();
            let fn_ty = InferTy::Fn {
                params,
                ret: Box::new(function.ret.clone()),
            };

            self.local_types
                .insert(LocalDefId::Function(function.id), fn_ty);
        }

        for function in declared {
            for (pat, ty) in function.params {
                self.bind_pat(pat, &Scheme {
                    ty,
                    params: Vec::new(),
                });
            }

            if let Some(root) = self.body.local_function_root(function.id) {
                self.check(root, &function.ret);
            }
        }

        self.infer(body)
    }

    /// Reads the signature of one function declared inside a body: the types of its parameters,
    /// and the type of its result.
    ///
    /// A signature is written where the function is declared, so a type it does not write is
    /// a mistake and not something to infer: what a call of the function takes and gives back is
    /// read before the body of the function is, the way the signature of a declaration of
    /// a module is ([ADR-0017]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn local_function(&mut self, place: ExprId, id: LocalFunctionId) -> DeclaredLocal {
        let data = self.body[id].clone();
        let mut params = Vec::with_capacity(data.params.len());

        for (index, pat) in data.params.iter().enumerate() {
            let annotation = data
                .signature
                .params
                .get(index)
                .and_then(|param| param.ty.as_ref());

            match annotation {
                Some(ty) if !matches!(ty, TypeRef::Infer) => {
                    let (resolved, errors) = self.resolver.type_of(ty);

                    for error in errors {
                        self.report_at(error, TypePlace::Pat(*pat));
                    }

                    params.push((*pat, InferTy::of(&resolved)));
                },
                _ => {
                    self.report_at(
                        TypeError::LocalFunctionMissingType {
                            function: data.name.clone(),
                            parameter: Some(index),
                        },
                        TypePlace::Pat(*pat),
                    );

                    params.push((*pat, InferTy::Error));
                },
            }
        }

        let ret = match data.signature.ret.as_ref() {
            Some(ty) if !matches!(ty, TypeRef::Infer) => {
                let (resolved, errors) = self.resolver.type_of(ty);

                for error in errors {
                    self.report(error, place);
                }

                InferTy::of(&resolved)
            },
            _ => {
                self.report(
                    TypeError::LocalFunctionMissingType {
                        function: data.name.clone(),
                        parameter: None,
                    },
                    place,
                );

                InferTy::Error
            },
        };

        DeclaredLocal { id, params, ret }
    }

    /// The type a parameter of a lambda takes: the type it writes, or a variable the body and
    /// the callers of the lambda work out.
    ///
    /// A type is read where a type belongs, and the lowering resolved the roots of a path of
    /// a body against the module: a name that is no type of the module is a name of another
    /// namespace, which the check is what reports --- the lowering reports a name that is not
    /// there at all.
    fn parameter_type(&mut self, param: &LambdaParam) -> InferTy {
        let place = TypePlace::Pat(param.pat);

        match &param.ty {
            // `_` written for a parameter of a lambda is a type to infer, as a parameter that
            // writes no type is: the body is where it is inferred from.
            Some(TypeRef::Infer) | None => self.engine.fresh_var(),
            Some(ty) => {
                if let TypeRef::Path(path) = ty
                    && matches!(path.anchor, PathAnchor::Unresolved)
                    && let Some(name) = path.root.name()
                    && self
                        .tree
                        .scope()
                        .get(name)
                        .is_some_and(|entry| entry.value.is_some() && entry.ty.is_none())
                {
                    self.report_at(TypeError::NotAType { name: name.clone() }, place);
                    return InferTy::Error;
                }

                let (resolved, errors) = self.resolver.type_of(ty);

                for error in errors {
                    self.report_at(error, place.clone());
                }

                InferTy::of(&resolved)
            },
        }
    }

    /// Binds a pattern to a type: what the pattern binds is what the type is.
    fn bind_pat(&mut self, pat: PatId, scheme: &Scheme) {
        match self.body[pat].clone() {
            Pat::Missing => self.record_pat(pat, InferTy::Error),
            Pat::Wildcard => self.record_pat(pat, scheme.ty.clone()),
            Pat::Bind(_) => {
                self.record_pat(pat, scheme.ty.clone());
                self.bindings.insert(pat, scheme.clone());
            },
        }
    }

    /// The type of a class the language declares.
    fn builtin(&self, which: Builtin) -> InferTy {
        let class = match which {
            Builtin::Int => self.deps.builtins().int(),
            Builtin::String => self.deps.builtins().string(),
            Builtin::Boolean => self.deps.builtins().boolean(),
            Builtin::Unit => self.deps.builtins().unit(),
        };

        InferTy::Class {
            class: class.clone(),
            args: Vec::new(),
        }
    }

    /// Records what an expression turned out to be.
    fn record_expr(&mut self, expr: ExprId, ty: InferTy) {
        self.expr_types.insert(expr, ty);
    }

    /// Records what a pattern turned out to be.
    fn record_pat(&mut self, pat: PatId, ty: InferTy) {
        self.pat_types.insert(pat, ty);
    }

    /// Reports an error at an expression.
    fn report(&mut self, error: TypeError, expr: ExprId) {
        self.report_at(error, TypePlace::Expr(expr));
    }

    /// Reports an error at a place.
    fn report_at(&mut self, error: TypeError, place: TypePlace) {
        self.diagnostics.push(TypeDiag::new(error, place));
    }

    /// Reports what unification failed at, with the types as a reader reads them.
    fn report_unify(&mut self, error: UnifyError, expr: ExprId) {
        match error {
            UnifyError::Mismatch { expected, found } => {
                let expected = self.engine.zonk(&expected);
                let found = self.engine.zonk(&found);

                self.report(TypeError::TypeMismatch { expected, found }, expr);
            },
            UnifyError::Recursive => self.report(TypeError::RecursiveType, expr),
        }
    }

    /// The values to store: every recorded type resolved, a variable that was never resolved
    /// becoming the error type.
    fn finish(self) -> (CheckedBody, Vec<TypeDiag>) {
        let mut checked = CheckedBody::new();

        for (expr, ty) in self.expr_types {
            checked.set_expr_type(expr, self.engine.zonk(&ty));
        }

        for (pat, ty) in self.pat_types {
            checked.set_pat_type(pat, self.engine.zonk(&ty));
        }

        for (local, ty) in self.local_types {
            checked.set_local_type(local, self.engine.zonk(&ty));
        }

        (checked, self.diagnostics)
    }
}

/// One function declared inside a body, as the check of a `local` read it: the patterns of its
/// parameters with the types they take, and the type of its result.
struct DeclaredLocal {
    /// The function in the arena of the body.
    id: LocalFunctionId,
    /// The pattern of every parameter, with the type it takes.
    params: Vec<(PatId, InferTy)>,
    /// The type of the result.
    ret: InferTy,
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use mlkc_hir_def::{
        Body, BodyEntityLoc, ClassLoc, EntityLoc, ItemKind, ItemLoc, ItemLocLike, ItemTree,
        LocalDefId, ModuleId, ModuleScope, Name, Prelude, ProjectGraph, ProjectId, TypeVarId,
        UseLoc,
    };
    use mlkc_hir_ty::{Builtins, CheckDeps, Ty};
    use mlkc_lower::{lower_body, lower_module};
    use mlkc_parser::parse;
    use mlkc_resolve::{Resolution, Target, closure};
    use mlkc_syntax::ModuleRoot;
    use mlkc_vfs::{FileId, RelPathBuf};

    use super::{Expr, TypeError, check_body};
    use crate::signatures::resolve_module_types;

    /// The classes of the language, as a module of a test declares them.
    const CLASSES: &str = "#[builtin]\ntype Int\n\n#[builtin]\ntype Unit\n\n#[builtin]\ntype String\n\n#[builtin]\ntype Bool\n";

    fn module(id: ModuleId, source: &str) -> (ItemTree, Vec<(BodyEntityLoc, Body)>) {
        let parsed = parse(source);
        let root = parsed.tree::<ModuleRoot>();
        let relative = RelPathBuf::try_from("main.mlk").expect("a relative path");
        let lowered = lower_module(
            id,
            &root,
            &Prelude::none(),
            &[ProjectId::new("std")],
            relative.as_path(),
        );
        let bodies = lowered
            .bodies
            .iter()
            .filter_map(|decl| {
                lower_body(&lowered.item_tree, &decl.decl)
                    .map(|lowered_body| (decl.owner.clone(), lowered_body.body))
            })
            .collect();

        (lowered.item_tree, bodies)
    }

    fn entity(tree: &ItemTree, name: &str) -> EntityLoc {
        tree.entities()
            .find(|(item, _)| item.name() == Some(&Name::new(name)))
            .map(|(item, _)| {
                EntityLoc {
                    module: tree.module(),
                    item,
                }
            })
            .expect("the entity to be declared")
    }

    fn class(tree: &ItemTree, name: &str) -> EntityLoc<ClassLoc> {
        let entity = entity(tree, name);

        EntityLoc {
            module: entity.module,
            item: ClassLoc::try_from(entity.item).expect("a class"),
        }
    }

    /// The classes of the language, read off a module that declares all four.
    fn builtins(tree: &ItemTree) -> Builtins {
        Builtins::new(
            class(tree, "Int"),
            class(tree, "Unit"),
            class(tree, "String"),
            class(tree, "Bool"),
        )
    }

    fn ids() -> (ModuleId, ModuleId, ModuleId) {
        (
            ModuleId(FileId::from_raw(0)),
            ModuleId(FileId::from_raw(1)),
            ModuleId(FileId::from_raw(2)),
        )
    }

    /// The import of the module under `name`, as the resolution records it.
    fn import(tree: &ItemTree, name: &str) -> UseLoc {
        tree.entities()
            .find_map(|(item, _)| {
                match item {
                    ItemLoc::Use(use_loc) if item.name() == Some(&Name::new(name)) => Some(use_loc),
                    _ => None,
                }
            })
            .expect("the import to be declared")
    }

    /// The resolution of a module of a test: the names of the module are the ones the test
    /// hands over, and the imports resolved to the entities it hands over.
    fn resolution(tree: &ItemTree, imports: &[(&str, EntityLoc)]) -> Resolution {
        let mut targets = BTreeMap::new();

        for (name, entity) in imports {
            let target = match entity.item.kind() {
                ItemKind::Class => Target::entity(Some(entity.clone()), None),
                _ => Target::entity(None, Some(entity.clone())),
            };

            targets.insert(import(tree, name), target);
        }

        Resolution::new(
            Arc::new(ModuleScope::default()),
            targets,
            Arc::from(Vec::new()),
        )
    }

    /// The deps of a module of a test: an empty world, and the closure of the module's own
    /// surface.
    fn deps(module: ModuleId, tree: &ItemTree, builtins: Builtins) -> CheckDeps {
        let graph = Arc::new(ProjectGraph::default());
        let closure = closure::of(module, tree, &graph, &BTreeMap::new(), &mut |_| None);

        CheckDeps::new(builtins)
            .with_graph(graph)
            .with_closure(closure)
    }

    #[test]
    fn a_body_reads_the_surface_of_its_own_module_and_of_the_modules_it_names() {
        let (std, a, b) = ids();

        // The library: the classes of the language, and nothing else.
        let (std_tree, _) = module(std, CLASSES);
        let std_builtins = builtins(&std_tree);
        let std_resolution = resolution(&std_tree, &[]);
        let std_deps = deps(std, &std_tree, std_builtins.clone());
        let (std_types, diagnostics) = resolve_module_types(&std_tree, &std_resolution, &std_deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let std_types = Arc::new(std_types);

        let int = class(&std_tree, "Int");

        // Module `a`: a function whose written signature names the `Int` of the library.
        let (a_tree, a_bodies) = module(
            a,
            "use project::std::Int\npub fun double(value: Int): Int = value\n",
        );
        let a_resolution = resolution(&a_tree, &[("Int", EntityLoc::from(int.clone()))]);
        let a_deps = deps(a, &a_tree, std_builtins.clone()).with_types(std, std_types.clone());
        let (a_types, diagnostics) = resolve_module_types(&a_tree, &a_resolution, &a_deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let a_types = Arc::new(a_types);
        let a_deps = a_deps.with_types(a, a_types.clone());

        let (owner, body) = &a_bodies[0];
        let (checked, diagnostics) =
            check_body(owner.clone(), &a_tree, body, &a_resolution, &a_deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            checked.expr_type(body.root()),
            Some(&Ty::class(int.clone()))
        );
        assert_eq!(
            checked.pat_type(body.params()[0]),
            Some(&Ty::class(int.clone())),
        );

        // Module `b`: a body that calls the function of `a`, whose type it reads from `a`'s
        // surface.
        let (b_tree, b_bodies) = module(
            b,
            "use project::std::Int\nuse project::a::double\nfun main(): Int = double(1)\n",
        );
        let b_resolution = resolution(&b_tree, &[
            ("Int", EntityLoc::from(int.clone())),
            ("double", entity(&a_tree, "double")),
        ]);
        let b_deps = deps(b, &b_tree, std_builtins)
            .with_types(std, std_types)
            .with_types(a, a_types);
        let (b_types, diagnostics) = resolve_module_types(&b_tree, &b_resolution, &b_deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let b_deps = b_deps.with_types(b, Arc::new(b_types));

        let (owner, body) = &b_bodies[0];
        let (checked, diagnostics) =
            check_body(owner.clone(), &b_tree, body, &b_resolution, &b_deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(checked.expr_type(body.root()), Some(&Ty::class(int)));
    }

    #[test]
    fn an_if_is_a_value_of_the_type_its_branches_agree_on() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun pick(low: Bool, high: Bool): Int = \
             if low then 1 elif high then 2 else 3\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let int = Ty::class(class(&tree, "Int"));
        let boolean = Ty::class(class(&tree, "Bool"));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(checked.expr_type(body.root()), Some(&int));

        // Every condition is a `Bool`, and every branch is an `Int`: the arms after the first
        // are checked against it, and a branch that is a `let` is as much a value as any other.
        let Expr::If {
            cond,
            then_,
            arms,
            otherwise,
        } = &body[body.root()]
        else {
            panic!("an `if` is the root of the body");
        };

        assert_eq!(checked.expr_type(*cond), Some(&boolean));
        assert_eq!(checked.expr_type(*then_), Some(&int));
        assert_eq!(
            checked.expr_type(otherwise.expect("the choice to have an `else`")),
            Some(&int),
        );

        for arm in arms {
            assert_eq!(checked.expr_type(arm.cond), Some(&boolean));
            assert_eq!(checked.expr_type(arm.body), Some(&int));
        }
    }

    #[test]
    fn a_condition_that_is_not_a_bool_is_reported() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun wrong(): Int = if 1 then 2 else 3\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::TypeMismatch { expected, found }
                    if *expected == Ty::class(class(&tree, "Bool"))
                        && *found == Ty::class(class(&tree, "Int"))
            ),
            "{diagnostics:?}",
        );

        // The branches agree on `Int`, and a condition that is not a `Bool` does not take the
        // value of the expression away: what the `if` is is still known.
        assert_eq!(
            checked.expr_type(body.root()),
            Some(&Ty::class(class(&tree, "Int"))),
        );
    }

    #[test]
    fn an_if_without_an_else_selects_the_unit_its_arms_are() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun log(flag: Bool): Unit =\n    if flag then\n        log(flag)\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let boolean = Ty::class(class(&tree, "Bool"));
        let unit = Ty::class(class(&tree, "Unit"));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        // An `if` without an `else` selects no value: its arms are `Unit`, and so it is.
        assert_eq!(checked.expr_type(body.root()), Some(&unit));

        let Expr::If {
            cond,
            then_,
            otherwise,
            ..
        } = &body[body.root()]
        else {
            panic!("an `if` is the root of the body");
        };

        assert!(otherwise.is_none(), "the choice to have no `else`");
        assert_eq!(checked.expr_type(*cond), Some(&boolean));
        assert_eq!(checked.expr_type(*then_), Some(&unit));
    }

    #[test]
    fn an_arm_that_is_not_a_unit_is_reported_when_there_is_no_else() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun wrong(flag: Bool): Unit = if flag then 1\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // The arm is what the choice selects when the condition holds, and a choice that
        // selects no value selects a `Unit`.
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::TypeMismatch { expected, found }
                    if *expected == Ty::class(class(&tree, "Unit"))
                        && *found == Ty::class(class(&tree, "Int"))
            ),
            "{diagnostics:?}",
        );
        assert_eq!(
            checked.expr_type(body.root()),
            Some(&Ty::class(class(&tree, "Unit"))),
        );
    }

    #[test]
    fn branches_of_different_types_are_reported() {
        let (id, ..) = ids();
        let source =
            format!("{CLASSES}\nfun wrong(flag: Bool): Int = if flag then 1 else \"text\"\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // The first branch is what the arms after it are checked against: the branch that is
        // not an `Int` is what is reported, and not the `if` around it.
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::TypeMismatch { expected, found }
                    if *expected == Ty::class(class(&tree, "Int"))
                        && *found == Ty::class(class(&tree, "String"))
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn a_lambda_is_a_function_of_its_parameters() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(value: Int): Int = (fn(x) -> x + value)(1)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let int = Ty::class(class(&tree, "Int"));
        assert_eq!(checked.expr_type(body.root()), Some(&int));

        let Expr::Call { callee, args } = &body[body.root()] else {
            panic!("the root to be a call");
        };
        assert_eq!(args.len(), 1, "the call to pass one argument");

        // What is called is the lambda, and the argument and the body fix what its parameter
        // is: the sum makes it an `Int` --- it reads the `value` of the enclosing body as
        // well --- and the call takes an `Int` and gives one back.
        assert_eq!(
            checked.expr_type(*callee),
            Some(&Ty::function(vec![int.clone()], int.clone())),
        );
        assert_eq!(checked.expr_type(args[0]), Some(&int));

        let Expr::Lambda { params, .. } = &body[*callee] else {
            panic!("the callee to be a lambda");
        };
        assert_eq!(checked.pat_type(params[0].pat), Some(&int));
    }

    #[test]
    fn a_function_declared_inside_a_body_is_checked_and_called() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun main(value: Int): Int = local fun double(x: Int): Int = x * 2 in \
             double(value)\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let int = Ty::class(class(&tree, "Int"));
        assert_eq!(checked.expr_type(body.root()), Some(&int));

        // The type of the function the `local` declares is recorded with the body, and it is
        // the signature the function writes: a function declared inside a body is not
        // generalized, the way a declaration of a module is not.
        let local = body
            .local_function_ids()
            .next()
            .expect("a function declared inside the body");
        assert_eq!(
            checked.local_type(LocalDefId::Function(local)),
            Some(&Ty::function(vec![int.clone()], int.clone())),
        );

        // The parameter of the function takes the type its signature writes.
        assert_eq!(checked.pat_type(body[local].params[0]), Some(&int));
    }

    #[test]
    fn a_function_declared_inside_a_body_may_call_itself_and_the_ones_written_after_it() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun main(value: Int): Bool = local\n    fun even(x: Int): Bool = if x == 0 \
             then true else odd(x - 1)\n    fun odd(x: Int): Bool = if x == 0 then false \
             else even(x - 1)\nin even(value)\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let boolean = Ty::class(class(&tree, "Bool"));
        assert_eq!(checked.expr_type(body.root()), Some(&boolean));

        // Both names are functions of the body, and each of them is checked against its own
        // signature.
        for local in body.local_function_ids() {
            assert_eq!(
                checked.local_type(LocalDefId::Function(local)),
                Some(&Ty::function(
                    vec![Ty::class(class(&tree, "Int"))],
                    boolean.clone()
                )),
            );
        }
    }

    #[test]
    fn a_call_of_a_function_declared_inside_a_body_checks_its_arguments() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun main(): Int = local fun double(x: Int): Int = x * 2 in double(true)\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // What a call passes is checked against the signature of the function it calls, the way
        // a call of a function of the module is.
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::TypeMismatch { expected, found }
                    if *expected == Ty::class(class(&tree, "Int"))
                        && *found == Ty::class(class(&tree, "Bool"))
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn the_result_of_a_function_declared_inside_a_body_is_its_signature() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = local fun wrong(): Int = true in 0\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // The body of the function is checked against the type its signature gives back, and
        // the expression after the `in` is checked on its own.
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::TypeMismatch { expected, found }
                    if *expected == Ty::class(class(&tree, "Int"))
                        && *found == Ty::class(class(&tree, "Bool"))
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn a_function_declared_inside_a_body_is_not_a_value() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = local fun one(): Int = 1 in one\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // The name of a function is what a call calls, and not a value the language has, the way
        // the name of a function of a module is ([ADR-0018]).
        //
        // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::NotAValue { name } if name == &Name::new("one")
        ));
    }

    #[test]
    fn a_function_declared_inside_a_body_writes_its_types() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = local fun id(x) = x in id(1)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // A signature is what a call of the function reads, and a type it does not write is
        // a mistake: the parameter and the result are both reported.
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::LocalFunctionMissingType {
                    function,
                    parameter: Some(0),
                } if function == &Name::new("id")
            ),
            "{diagnostics:?}",
        );
        assert!(
            matches!(
                diagnostics[1].error(),
                TypeError::LocalFunctionMissingType {
                    function,
                    parameter: None,
                } if function == &Name::new("id")
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn a_lambda_parameter_takes_the_type_it_writes() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = (fn(x: Int) -> x)(1)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let int = Ty::class(class(&tree, "Int"));
        let Expr::Call { callee, .. } = &body[body.root()] else {
            panic!("the root to be a call");
        };

        assert_eq!(
            checked.expr_type(*callee),
            Some(&Ty::function(vec![int.clone()], int.clone())),
        );

        let Expr::Lambda { params, .. } = &body[*callee] else {
            panic!("the callee to be a lambda");
        };
        assert_eq!(checked.pat_type(params[0].pat), Some(&int));
    }

    #[test]
    fn a_lambda_parameter_left_to_infer_is_inferred_from_the_body() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = (fn(x: _) -> x + 1)(2)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let int = Ty::class(class(&tree, "Int"));
        let Expr::Call { callee, .. } = &body[body.root()] else {
            panic!("the root to be a call");
        };

        assert_eq!(
            checked.expr_type(*callee),
            Some(&Ty::function(vec![int.clone()], int.clone())),
        );
    }

    #[test]
    fn a_lambda_bound_by_a_let_is_used_at_two_types() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun main(): Int =\n    let id = fn(x) -> x in\n    let _ = id(true) in\n    id(1)\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // The same lambda is read as a function of `Bool` and of `Int`: a lambda bound by
        // a `let` is generalized like any other value of it.
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            checked.expr_type(body.root()),
            Some(&Ty::class(class(&tree, "Int"))),
        );
    }

    #[test]
    fn a_lambda_bound_by_a_let_reads_as_the_parameter_it_was_generalized_over() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = let id = fn(x) -> x in id(1)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let Expr::Let { expr: init, .. } = &body[body.root()] else {
            panic!("a `let` is the root of the body");
        };
        let Expr::Lambda {
            params,
            body: lambda,
            ..
        } = &body[*init]
        else {
            panic!("the right side of the `let` to be a lambda");
        };

        // The lambda generalized over its parameter, and every place the variable of it was
        // recorded at reads as that parameter: one parameter on both sides of the arrow, and
        // nothing of it a mistake.
        let param = Ty::Param(TypeVarId {
            owner: EntityLoc::from(owner.clone()),
            index: 0,
        });
        let function = Ty::function(vec![param.clone()], param.clone());

        assert_eq!(checked.expr_type(*init), Some(&function));
        assert_eq!(checked.pat_type(params[0].pat), Some(&param));
        assert_eq!(checked.expr_type(*lambda), Some(&param));
    }

    #[test]
    fn a_lambda_parameter_that_is_not_its_argument_is_reported() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun main(): Int = (fn(x: Int) -> x)(true)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::TypeMismatch { expected, found }
                    if *expected == Ty::class(class(&tree, "Int"))
                        && *found == Ty::class(class(&tree, "Bool"))
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn a_lambda_parameter_type_that_names_a_value_is_reported() {
        let (id, ..) = ids();
        let source =
            format!("{CLASSES}\nfun value(): Int = 1\nfun main(): Int = (fn(x: value) -> x)(1)\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        // The module declares `value` before `main`, so the body of `main` is the last one.
        let (owner, body) = bodies.last().expect("the body of `main`");
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // A name written where a type belongs is a type of the module, and a value of it is
        // a mistake the check reports: the lowering reports a name that is not there at all.
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            matches!(
                diagnostics[0].error(),
                TypeError::NotAType { name } if *name == Name::new("value")
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn arithmetic_comparisons_and_lets_are_typed_by_the_rules_of_the_check() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\nfun main(): Int = let x = 1 in x + 2\nfun compare(): Bool = 1 == 2 && 3 < 4\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let int = Ty::class(class(&tree, "Int"));
        let boolean = Ty::class(class(&tree, "Bool"));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(checked.expr_type(body.root()), Some(&int));

        // The binding of the `let` is an `Int`, and so is what the body computes.
        let Expr::Let { pat, .. } = &body[body.root()] else {
            panic!("a `let` is the root of the body");
        };
        assert_eq!(checked.pat_type(*pat), Some(&int));

        let (owner, body) = &bodies[1];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(checked.expr_type(body.root()), Some(&boolean));
    }

    #[test]
    fn a_truth_value_is_a_bool() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun truth(): Bool = true\nfun untruth(): Bool = false\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let boolean = Ty::class(class(&tree, "Bool"));

        for (owner, body) in &bodies {
            let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

            assert!(diagnostics.is_empty(), "{diagnostics:?}");
            assert_eq!(checked.expr_type(body.root()), Some(&boolean));
        }
    }

    #[test]
    fn a_literal_out_of_the_range_of_int_is_reported() {
        let (id, ..) = ids();
        let source = format!("{CLASSES}\nfun big(): Int = 1099511627776\n");
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let (owner, body) = &bodies[0];
        let (checked, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);

        // The range of a literal is the meaning of the type it is written with, so the check is
        // what reports it ([ADR-0018]).
        //
        // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::IntOutOfRange { value } if *value == 1_099_511_627_776
        ));

        // The literal is still an `Int`: what is wrong is the value, and not the type.
        let int = Ty::class(class(&tree, "Int"));
        assert_eq!(checked.expr_type(body.root()), Some(&int));
    }

    #[test]
    fn equality_is_over_int_and_bool_for_now() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\n\
             fun ints(left: Int, right: Int): Bool = left != right\n\
             fun booleans(left: Bool, right: Bool): Bool = left == right\n\
             fun strings(left: String, right: String): Bool = left == right\n\
             fun mixed(left: String, right: Int): Bool = left == right\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let check = |index: usize| {
            let (owner, body) = &bodies[index];
            check_body(owner.clone(), &tree, body, &resolution, &deps)
        };

        // Equality is over `Int` and `Bool`: the primitive of MIR exists for the two.
        assert!(check(0).1.is_empty(), "{:?}", check(0).1);
        assert!(check(1).1.is_empty(), "{:?}", check(1).1);

        // A `String` has no equality yet, and the check is what says so.
        let (_, diagnostics) = check(2);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::NoEquality { ty } if *ty == Ty::class(class(&tree, "String"))
        ));

        // Two operands that do not check against each other are one mismatch, and not a mismatch
        // and a missing equality: a mistake is reported once.
        let (_, diagnostics) = check(3);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn a_function_is_not_a_value() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\n\
             fun id(x: Int): Int = x\n\
             fun bound(): Int = let f = id in f(1)\n\
             fun called(): Int = id(1)\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        // The name of a function in the position of a callee is what a call calls.
        let (owner, body) = &bodies[2];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        // The same name bound like a value denotes no value: the language has no function
        // value yet, and MIR has nothing to lower one to ([ADR-0019]).
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        let (owner, body) = &bodies[1];
        let (_, diagnostics) = check_body(owner.clone(), &tree, body, &resolution, &deps);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::NotAValue { name } if name == &Name::new("id")
        ));
    }

    #[test]
    fn a_mistake_is_reported_once_and_absorbs_what_it_meets() {
        let (id, ..) = ids();
        let source = format!(
            "{CLASSES}\n\
             fun wrong(): Bool = 1\n\
             fun arity(): Int = one(1, 2)\n\
             fun one(x: Int): Int = x\n\
             fun text(): String = \"s\"\n\
             fun class_as_value(): Int = Int\n"
        );
        let (tree, bodies) = module(id, &source);
        let resolution = resolution(&tree, &[]);
        let deps = deps(id, &tree, builtins(&tree));
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let deps = deps.with_types(id, Arc::new(types));

        let int = Ty::class(class(&tree, "Int"));
        let boolean = Ty::class(class(&tree, "Bool"));
        let string = Ty::class(class(&tree, "String"));

        let check = |index: usize| {
            let (owner, body) = &bodies[index];
            check_body(owner.clone(), &tree, body, &resolution, &deps)
        };

        // `Bool` where `Int` belongs: one mismatch, and the expression is the error type.
        let (checked, diagnostics) = check(0);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::TypeMismatch { expected, found }
                if expected == &boolean && found == &int
        ));
        assert_eq!(checked.expr_type(bodies[0].1.root()), Some(&Ty::Error));

        // A call with the wrong number of arguments: one report, and the call is the result.
        let (checked, diagnostics) = check(1);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].error(), TypeError::ArgumentCount {
            expected: 1,
            found: 2
        }));
        assert_eq!(checked.expr_type(bodies[1].1.root()), Some(&int));

        // A string literal is a `String`.
        let (checked, diagnostics) = check(3);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(checked.expr_type(bodies[3].1.root()), Some(&string));

        // A class written where a value belongs.
        let (_, diagnostics) = check(4);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::NotAValue { .. }
        ));
    }
}
