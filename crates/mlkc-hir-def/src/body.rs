//! The HIR of one body: the expressions, the patterns, the paths,
//! and the entities declared inside one function.
//!
//! A body has no name of its own.
//! It is the body of an entity, and the entity's name plus the syntax it was lowered from
//! identify it; everything inside it is addressed positionally,
//! which is sound because the body is one value that is never sliced.

use std::{fmt, ops::Index};

use mlkc_intern::Interned;
use mlkc_la_arena::{Arena, ArenaMap, Idx};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    def_map::LocalScope,
    id::{LocalConstId, LocalDefId, LocalFunctionId},
    item_data::Signature,
    name::Name,
    path::{PathAnchor, PathData, PathId},
    type_ref::TypeRef,
};

/// The id of an expression inside one body.
pub type ExprId = Idx<Expr>;

/// The id of a pattern inside one body.
pub type PatId = Idx<Pat>;

/// An expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An expression that is missing: the syntax was broken.
    Missing,
    /// A path, resolved as far as the module alone can resolve it.
    Path(PathId),
    /// A literal.
    Literal(Literal),
    /// A call: the callee, and the arguments in the order they are written.
    Call {
        /// The expression that is called.
        callee: ExprId,
        /// The arguments, in the order they are written.
        args: Vec<ExprId>,
    },
    /// A field read: the value the field is on, and the name of the field.
    Field {
        /// The value the field is read from.
        receiver: ExprId,
        /// The name of the field, as it is written.
        field: Name,
    },
    /// A binary operation.
    Binary {
        /// The left operand.
        lhs: ExprId,
        /// The operator.
        op: BinaryOp,
        /// The right operand.
        rhs: ExprId,
    },
    /// A prefix operation: a sign written in front of an expression.
    ///
    /// The sign is kept as a node of its own rather than folded into what it applies to:
    /// what it means is a question about the type of the operand, which the HIR does not
    /// answer.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: ExprId,
    },
    /// A `let`: a pattern, the expression it is bound to, and the body it is visible in.
    ///
    /// A spelling that binds a value is lowered to one: the value of a pipeline is bound by
    /// a `let` under a name no module can write ([ADR-0014]).
    ///
    /// [ADR-0014]: ../../docs/adr/0014-syntactic-sugar.md
    Let {
        /// The pattern the binding introduces.
        pat: PatId,
        /// The expression bound to the pattern.
        expr: ExprId,
        /// The expression the binding is visible in.
        body: ExprId,
    },
    /// A function written where a value belongs: `fn(a, b) -> expr`.
    ///
    /// What the lambda is is its parameters, its body, and the bindings of the enclosing body
    /// its body reads: a lambda is a value that carries the environment it cannot make itself,
    /// which is what [`BodyBuilder::captures`] works out.
    Lambda {
        /// The parameters, in the order they are declared.
        params: Vec<LambdaParam>,
        /// The expression that is the body.
        body: ExprId,
        /// The bindings of the enclosing body the body reads, in the order they are first
        /// written, each once.
        captures: Vec<PatId>,
    },
    /// A `local`: the items it declares, and the expression they are visible in.
    ///
    /// The items are the items of a module written where a value belongs: each of them is an
    /// entity of the enclosing body ([`LocalDefId`]), and what the entity is made of --- its
    /// signature, the patterns of its parameters, its root expression --- is read from the body
    /// itself. The expression after the `in` is what the names of the items are visible in, and
    /// the items see one another and themselves, which is what a recursive function is written
    /// with.
    ///
    /// A function declared inside a body is given its own parameters only: what the body that
    /// declares it binds is not part of it, and a path of it that names one is a mistake the
    /// lowering reports. What the body of one reads of the module it reaches without an
    /// environment, the way a function of the module does.
    Local {
        /// The entities the items declare, in the order they are written.
        items: Vec<LocalDefId>,
        /// The expression the names of the items are visible in.
        body: ExprId,
    },
    /// A choice between expressions: a condition, the expression it selects, the `elif` arms
    /// written after it, and the expression selected when no condition holds.
    If {
        /// The condition, which is a `Bool`.
        cond: ExprId,
        /// The expression selected when the condition holds.
        then_: ExprId,
        /// The `elif` arms, in the order they are written.
        arms: Vec<IfArm>,
        /// The expression selected when no condition holds.
        ///
        /// An `if` without one selects no value: its arms are `Unit`, and so it is.
        otherwise: Option<ExprId>,
    },
}

/// One `elif` arm of an [`Expr::If`]: its own condition and the expression it selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfArm {
    /// The condition, which is a `Bool`.
    pub cond: ExprId,
    /// The expression selected when the condition holds.
    pub body: ExprId,
}

/// One parameter of an [`Expr::Lambda`]: the pattern it binds, and the type it takes.
///
/// A lambda has no signature to write a type in, and a parameter may take one next to the
/// pattern it binds. The type is what the module wrote, resolved the way a type of a body is:
/// its names are looked for in the type namespace of the module ([`BodyBuilder::finish`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaParam {
    /// The pattern the parameter binds.
    pub pat: PatId,
    /// The type the parameter takes, where the module wrote one.
    pub ty: Option<TypeRef>,
}

/// A binary operator, as the language spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
}

impl BinaryOp {
    /// The operator as it is written.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A unary operator, as the language spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOp {
    /// `-`
    Neg,
    /// `+`
    Pos,
}

impl UnaryOp {
    /// The operator as it is written.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pat {
    /// A pattern that is missing: the syntax was broken.
    Missing,
    /// `_`.
    Wildcard,
    /// A name the pattern binds.
    Bind(Name),
}

/// A literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    /// An integer literal, with the value the source wrote.
    Int(i64),
    /// A truth value, with the value the word it is written with means.
    Bool(bool),
    /// A string literal, with the value its escapes decode to.
    Str(Interned<str>),
}

/// What a function declared inside a body is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFunctionData {
    /// The name the function is declared under.
    pub name: Name,
    /// The signature a caller reads.
    pub signature: Signature,
    /// The patterns of the parameters, in the order they are declared.
    ///
    /// The types of the parameters are the ones [`Self::signature`] holds, position by position:
    /// a signature is what a caller reads, and the patterns are what the body of the function is
    /// checked and lowered with.
    pub params: Vec<PatId>,
}

/// What a constant declared inside a body is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalConstData {
    /// The name the constant is declared under.
    pub name: Name,
    /// The type of the constant, as written.
    pub ty: Option<TypeRef>,
}

/// The HIR of one body.
#[derive(Debug, PartialEq, Eq)]
pub struct Body {
    root: ExprId,
    exprs: Arena<Expr>,
    pats: Arena<Pat>,
    paths: Arena<PathData>,
    local_functions: Arena<LocalFunctionData>,
    local_consts: Arena<LocalConstData>,
    local_function_roots: ArenaMap<LocalFunctionId, ExprId>,
    local_const_roots: ArenaMap<LocalConstId, ExprId>,
    params: Vec<PatId>,
}

impl Body {
    /// The root expression of the owner of this body.
    pub fn root(&self) -> ExprId {
        self.root
    }

    /// The patterns of the parameters of the owner, in the order they are declared.
    pub fn params(&self) -> &[PatId] {
        &self.params
    }

    /// The expressions of the body, the ones of its local entities included.
    pub fn exprs(&self) -> &Arena<Expr> {
        &self.exprs
    }

    /// The patterns of the body, the ones of its local entities included.
    pub fn pats(&self) -> &Arena<Pat> {
        &self.pats
    }

    /// The paths of the body, in the order they were first written.
    pub fn paths(&self) -> &Arena<PathData> {
        &self.paths
    }

    /// The functions declared inside the body.
    pub fn local_functions(&self) -> &Arena<LocalFunctionData> {
        &self.local_functions
    }

    /// The ids of the functions declared inside the body, in the order they are declared.
    pub fn local_function_ids(&self) -> impl Iterator<Item = LocalFunctionId> + '_ {
        self.local_functions
            .iter()
            .map(|(id, _)| LocalFunctionId(id))
    }

    /// The constants declared inside the body.
    pub fn local_consts(&self) -> &Arena<LocalConstData> {
        &self.local_consts
    }

    /// The root expression of a function declared inside the body.
    pub fn local_function_root(&self, id: LocalFunctionId) -> Option<ExprId> {
        self.local_function_roots.get(id).copied()
    }

    /// The root expression of a constant declared inside the body.
    pub fn local_const_root(&self, id: LocalConstId) -> Option<ExprId> {
        self.local_const_roots.get(id).copied()
    }
}

impl Index<ExprId> for Body {
    type Output = Expr;

    fn index(&self, id: ExprId) -> &Self::Output {
        &self.exprs[id]
    }
}

impl Index<PatId> for Body {
    type Output = Pat;

    fn index(&self, id: PatId) -> &Self::Output {
        &self.pats[id]
    }
}

impl Index<PathId> for Body {
    type Output = PathData;

    fn index(&self, id: PathId) -> &Self::Output {
        &self.paths[id]
    }
}

impl Index<LocalFunctionId> for Body {
    type Output = LocalFunctionData;

    fn index(&self, id: LocalFunctionId) -> &Self::Output {
        &self.local_functions[id.0]
    }
}

impl Index<LocalConstId> for Body {
    type Output = LocalConstData;

    fn index(&self, id: LocalConstId) -> &Self::Output {
        &self.local_consts[id.0]
    }
}

/// Builds the body of one entity.
///
/// The builder owns the arenas, so that a caller allocates nodes and declares local entities
/// in the order the lowering meets them, and a root that is only known later
/// is set when it is known.
#[derive(Debug, Default)]
pub struct BodyBuilder {
    root: Option<ExprId>,
    exprs: Arena<Expr>,
    pats: Arena<Pat>,
    paths: Arena<PathData>,
    /// The paths the body holds, by value, so that a path written twice is one entry.
    path_ids: FxHashMap<PathData, PathId>,
    local_functions: Arena<LocalFunctionData>,
    local_consts: Arena<LocalConstData>,
    local_function_roots: ArenaMap<LocalFunctionId, ExprId>,
    local_const_roots: ArenaMap<LocalConstId, ExprId>,
    params: Vec<PatId>,
}

impl BodyBuilder {
    /// A builder with no nodes in it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates an expression.
    pub fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        self.exprs.alloc(expr)
    }

    /// Allocates a pattern.
    pub fn alloc_pat(&mut self, pat: Pat) -> PatId {
        self.pats.alloc(pat)
    }

    /// Interns a path: the body holds one entry per path, however many places the path is
    /// written in.
    ///
    /// A path is a value rather than a place: two paths written the same way that denote the
    /// same thing are one path, and what tells the places apart is the expression that holds
    /// each of them. A place no module wrote --- the binding of a pipeline is one --- is
    /// interned by the same rule.
    pub fn intern_path(&mut self, path: PathData) -> PathId {
        if let Some(id) = self.path_ids.get(&path) {
            return *id;
        }

        let id = self.paths.alloc(path.clone());
        self.path_ids.insert(path, id);

        id
    }

    /// Declares a function inside the body, and returns its id.
    pub fn declare_local_function(&mut self, data: LocalFunctionData) -> LocalFunctionId {
        LocalFunctionId(self.local_functions.alloc(data))
    }

    /// Sets the root expression of a function declared inside the body.
    pub fn set_local_function_root(&mut self, id: LocalFunctionId, root: ExprId) {
        self.local_function_roots.insert(id, root);
    }

    /// Declares a constant inside the body, and returns its id.
    pub fn declare_local_const(&mut self, data: LocalConstData) -> LocalConstId {
        LocalConstId(self.local_consts.alloc(data))
    }

    /// Sets the root expression of a constant declared inside the body.
    pub fn set_local_const_root(&mut self, id: LocalConstId, root: ExprId) {
        self.local_const_roots.insert(id, root);
    }

    /// Adds a pattern of a parameter of the owner, in the order the parameters are declared.
    pub fn push_param(&mut self, pat: PatId) {
        self.params.push(pat);
    }

    /// Sets the root expression of the owner of this body.
    pub fn set_root(&mut self, root: ExprId) {
        self.root = Some(root);
    }

    /// The bindings of the enclosing body a lambda captures: the free variables of its body.
    ///
    /// A path of the body that names a binding is free when the binding is one the lambda does
    /// not make: neither a parameter of it nor a `let` inside it. What is captured is the
    /// binding, and not the name: a `let` inside the lambda that shadows a name of the
    /// enclosing body changes which binding the paths after it read, and the analysis reads the
    /// paths the body holds, not the names they were written with.
    ///
    /// A name of the module is not a variable of the body: a lambda reaches the declarations of
    /// the module wherever it runs, so what it captures is only what it cannot reach without
    /// an environment.
    ///
    /// A lambda written inside the body is a value of it, and what is free in it is not free in
    /// the lambda being analyzed: its captures are read as the names it needs of the body it is
    /// written in, and a binding the analyzed lambda makes does not reach the list.
    ///
    /// The list is in the order the body first writes each binding, and a binding written twice
    /// is one entry: what the list is read for is the environment to build, and building it
    /// once is what building it means.
    ///
    /// # Panics
    ///
    /// Panics if `body` or a node it reaches is not an expression of this builder: the
    /// analysis walks a body its lowering built, where every field holds a node of the body.
    pub fn captures(&self, body: ExprId, params: &[PatId]) -> Vec<PatId> {
        let mut bound: FxHashSet<PatId> = params.iter().copied().collect();
        let mut captures = Vec::new();

        self.free_variables(body, &mut bound, &mut captures);

        captures
    }

    /// Collects the bindings of `expr` that `bound` does not hold, and the bindings nested
    /// lambdas capture that it does not hold either.
    ///
    /// `bound` grows as the walk meets the bindings of the expression: a `let` binds what is
    /// after it, and not what it is bound to, which is the order the lowering resolved the
    /// paths of the body in.
    fn free_variables(
        &self,
        expr: ExprId,
        bound: &mut FxHashSet<PatId>,
        captures: &mut Vec<PatId>,
    ) {
        match &self.exprs[expr] {
            Expr::Missing | Expr::Literal(_) => {},
            Expr::Path(path) => {
                if let PathAnchor::Binding(pat) = self.paths[*path].anchor
                    && !bound.contains(&pat)
                    && !captures.contains(&pat)
                {
                    captures.push(pat);
                }
            },
            Expr::Call { callee, args } => {
                self.free_variables(*callee, bound, captures);

                for arg in args {
                    self.free_variables(*arg, bound, captures);
                }
            },
            Expr::Field { receiver, .. } => self.free_variables(*receiver, bound, captures),
            Expr::Binary { lhs, rhs, .. } => {
                self.free_variables(*lhs, bound, captures);
                self.free_variables(*rhs, bound, captures);
            },
            Expr::Unary { operand, .. } => self.free_variables(*operand, bound, captures),
            Expr::Let { pat, expr, body } => {
                self.free_variables(*expr, bound, captures);
                bound.insert(*pat);
                self.free_variables(*body, bound, captures);
            },
            Expr::If {
                cond,
                then_,
                arms,
                otherwise,
            } => {
                self.free_variables(*cond, bound, captures);
                self.free_variables(*then_, bound, captures);

                for arm in arms {
                    self.free_variables(arm.cond, bound, captures);
                    self.free_variables(arm.body, bound, captures);
                }

                if let Some(otherwise) = otherwise {
                    self.free_variables(*otherwise, bound, captures);
                }
            },
            // A lambda is a value the body holds: what its own body reads of this one is
            // what it captures, which is already the free variables of it, and walking the
            // body of it again would read the parameters of the inner lambda as free names
            // of the outer one.
            Expr::Lambda {
                captures: inner, ..
            } => {
                for pat in inner {
                    if !bound.contains(pat) && !captures.contains(pat) {
                        captures.push(*pat);
                    }
                }
            },
            // A `local` declares items inside the body, and a function declared inside a body is
            // given its own parameters only: it reads nothing of the body that declares it, so
            // nothing of it is free in the body that holds the `local`. The expression after the
            // `in` is read the way the body of a `let` is.
            Expr::Local { body, .. } => self.free_variables(*body, bound, captures),
        }
    }

    /// Finishes the body, resolving the anchors the types it writes left unresolved.
    ///
    /// Which names the paths of a body are rooted at is what the lowering of the body decides
    /// --- the bindings of the body are what only it knows --- and a name it left unresolved is
    /// a name a later stage reads. A type is not such a name: a type is read where a type
    /// belongs, so the names of it are looked for in the type namespace of the module, which is
    /// what `scope` is.
    ///
    /// # Panics
    ///
    /// Panics if the body has no root expression.
    /// A body is the body of an entity, and an entity that owns one has a root;
    /// a lowering that forgets to set it is a bug, not an input to recover from.
    pub fn finish(mut self, scope: &LocalScope) -> Body {
        for path in self.paths.values_mut() {
            for arg in &mut path.root_args {
                arg.resolve(scope);
            }

            for segment in &mut path.segments {
                for arg in &mut segment.args {
                    arg.resolve(scope);
                }
            }
        }

        // A parameter of a lambda is where a type is written inside a body, and a type is read
        // where a type belongs: the names of it are looked for in the type namespace of the
        // module, which is what `scope` is.
        for expr in self.exprs.values_mut() {
            if let Expr::Lambda { params, .. } = expr {
                for param in params {
                    if let Some(ty) = &mut param.ty {
                        ty.resolve(scope);
                    }
                }
            }
        }

        // The signature of a function declared inside a body is written in that body as well:
        // a type is read where a type belongs, wherever it is written.
        for data in self.local_functions.values_mut() {
            data.signature.resolve(scope);
        }

        debug_assert_eq!(
            self.local_function_roots.iter().count(),
            self.local_functions.len(),
            "every function declared inside a body has a root",
        );
        debug_assert_eq!(
            self.local_const_roots.iter().count(),
            self.local_consts.len(),
            "every constant declared inside a body has a root",
        );

        Body {
            root: self.root.expect("a body has a root expression"),
            exprs: self.exprs,
            pats: self.pats,
            paths: self.paths,
            local_functions: self.local_functions,
            local_consts: self.local_consts,
            local_function_roots: self.local_function_roots,
            local_const_roots: self.local_const_roots,
            params: self.params,
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::{
        BinaryOp, Body, BodyBuilder, Expr, ExprId, Interned, LambdaParam, Literal, LocalDefId,
        LocalFunctionData, LocalScope, Name, Pat, PatId, PathData, Signature,
    };
    use crate::{
        id::{EntityLoc, FunctionLoc, ItemLoc, ItemLocData, ModuleId},
        path::PathAnchor,
    };

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn entity(name: &str) -> EntityLoc {
        EntityLoc {
            module: module(),
            item: ItemLoc::Function(FunctionLoc(ItemLocData {
                name: Some(Name::new(name)),
                disambiguator: 0,
            })),
        }
    }

    /// A body of one function: `let x = 1 in g(x)`.
    fn body_with_a_call() -> Body {
        let mut builder = BodyBuilder::new();
        let name = builder.intern_path(PathData::ident(
            Name::new("g"),
            PathAnchor::Item(entity("g")),
        ));
        let callee = builder.alloc_expr(Expr::Path(name));
        let literal = builder.alloc_expr(Expr::Literal(Literal::Int(1)));
        let pat = builder.alloc_pat(Pat::Bind(Name::new("x")));
        let binding =
            builder.intern_path(PathData::ident(Name::new("x"), PathAnchor::Binding(pat)));
        let bound = builder.alloc_expr(Expr::Path(binding));
        let call = builder.alloc_expr(Expr::Call {
            callee,
            args: vec![bound],
        });
        let root = builder.alloc_expr(Expr::Let {
            pat,
            expr: literal,
            body: call,
        });
        builder.set_root(root);

        builder.finish(&LocalScope::default())
    }

    #[test]
    fn a_body_of_the_same_shape_is_the_same_value() {
        // The driver keeps the value it has when a new one is equal to it;
        // a body that was lowered twice from the same text has to compare equal.
        assert_eq!(body_with_a_call(), body_with_a_call());
    }

    #[test]
    fn the_root_and_the_parameters_are_kept() {
        let mut builder = BodyBuilder::new();
        let missing = builder.alloc_expr(Expr::Missing);
        let pat = builder.alloc_pat(Pat::Wildcard);
        builder.push_param(pat);
        builder.set_root(missing);
        let body = builder.finish(&LocalScope::default());

        assert_eq!(body.root(), missing);
        assert_eq!(body.params(), [pat]);
        assert_eq!(body[missing], Expr::Missing);
        assert_eq!(body[pat], Pat::Wildcard);
    }

    #[test]
    fn a_local_function_keeps_its_root() {
        let mut builder = BodyBuilder::new();
        let local = builder.declare_local_function(LocalFunctionData {
            name: Name::new("helper"),
            signature: Signature::default(),
            params: Vec::new(),
        });
        let root = builder.alloc_expr(Expr::Literal(Literal::Str(Interned::new_str("hi"))));
        builder.set_local_function_root(local, root);
        builder.set_root(root);
        let body = builder.finish(&LocalScope::default());

        assert_eq!(body.local_function_root(local), Some(root));
        assert_eq!(body[local].name, Name::new("helper"));
        // The expressions of a local entity live in the arena of the enclosing body.
        assert_eq!(body.exprs().len(), 1);
        assert_eq!(body.root(), root);
    }

    /// Binds `name` to a pattern of the body, and returns both.
    fn binding(builder: &mut BodyBuilder, name: &str) -> (PatId, ExprId) {
        let pat = builder.alloc_pat(Pat::Bind(Name::new(name)));
        let path = builder.intern_path(PathData::ident(Name::new(name), PathAnchor::Binding(pat)));

        (pat, builder.alloc_expr(Expr::Path(path)))
    }

    #[test]
    fn a_lambda_captures_the_bindings_of_the_enclosing_body_it_reads() {
        // `fun f(x: Int): Int = fn(y) -> x + y + x`: the body reads the outer `x` twice and
        // its own `y`, so what it captures is `x`, written once.
        let mut builder = BodyBuilder::new();
        let (x, x_expr) = binding(&mut builder, "x");
        let (y, y_expr) = binding(&mut builder, "y");
        let first = builder.alloc_expr(Expr::Binary {
            lhs: x_expr,
            op: BinaryOp::Add,
            rhs: y_expr,
        });
        let sum = builder.alloc_expr(Expr::Binary {
            lhs: first,
            op: BinaryOp::Add,
            rhs: x_expr,
        });

        assert_eq!(builder.captures(sum, &[y]), [x]);
    }

    #[test]
    fn a_binding_the_lambda_makes_is_not_captured() {
        // `fn(y) -> x + (let x = 1 in x)`: the `let` shadows the outer `x`, and only the `x`
        // written before it is captured.
        let mut builder = BodyBuilder::new();
        let (outer, outer_expr) = binding(&mut builder, "x");
        let (inner, inner_expr) = binding(&mut builder, "x");
        let one = builder.alloc_expr(Expr::Literal(Literal::Int(1)));
        let bound = builder.alloc_expr(Expr::Let {
            pat: inner,
            expr: one,
            body: inner_expr,
        });
        let sum = builder.alloc_expr(Expr::Binary {
            lhs: outer_expr,
            op: BinaryOp::Add,
            rhs: bound,
        });

        assert_eq!(builder.captures(sum, &[]), [outer]);
    }

    #[test]
    fn a_nested_lambda_is_a_value_and_not_a_body_of_the_outer_one() {
        // `fn(x) -> fn(y) -> x + z`: the inner lambda captures the `x` of the outer one and
        // the `z` of the enclosing body; the outer one captures only `z`.
        let mut builder = BodyBuilder::new();
        let (x, x_expr) = binding(&mut builder, "x");
        let (y, _) = binding(&mut builder, "y");
        let (z, z_expr) = binding(&mut builder, "z");
        let sum = builder.alloc_expr(Expr::Binary {
            lhs: x_expr,
            op: BinaryOp::Add,
            rhs: z_expr,
        });
        let inner = builder.alloc_expr(Expr::Lambda {
            params: vec![LambdaParam { pat: y, ty: None }],
            body: sum,
            captures: builder.captures(sum, &[y]),
        });

        assert_eq!(builder.captures(sum, &[y]), [x, z]);
        assert_eq!(builder.captures(inner, &[x]), [z]);
    }

    #[test]
    fn a_local_function_is_not_part_of_what_the_enclosing_body_captures() {
        // `local fun add(y) = x + y in add(z)`: a function declared inside a body is given its
        // own parameters only, so what the body of the item reads of the enclosing body is not
        // free in the body that declares it --- the lowering reports such a read --- and what
        // the `local` makes free is what the expression after the `in` reads.
        let mut builder = BodyBuilder::new();
        let (_, x_expr) = binding(&mut builder, "x");
        let (y, y_expr) = binding(&mut builder, "y");
        let (z, z_expr) = binding(&mut builder, "z");
        let sum = builder.alloc_expr(Expr::Binary {
            lhs: x_expr,
            op: BinaryOp::Add,
            rhs: y_expr,
        });
        let item = builder.declare_local_function(LocalFunctionData {
            name: Name::new("add"),
            signature: Signature::default(),
            params: vec![y],
        });
        builder.set_local_function_root(item, sum);

        let callee = builder.intern_path(PathData::ident(
            Name::new("add"),
            PathAnchor::Local(LocalDefId::Function(item)),
        ));
        let callee = builder.alloc_expr(Expr::Path(callee));
        let call = builder.alloc_expr(Expr::Call {
            callee,
            args: vec![z_expr],
        });
        let local = builder.alloc_expr(Expr::Local {
            items: vec![LocalDefId::Function(item)],
            body: call,
        });

        assert_eq!(builder.captures(local, &[]), [z]);
    }
}
