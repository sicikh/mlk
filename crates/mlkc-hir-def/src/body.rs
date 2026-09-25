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

use crate::{
    id::{LocalConstId, LocalFunctionId},
    item_data::Signature,
    name::Name,
    path::{PathData, PathId},
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
    /// The first expression, then the second.
    Seq {
        /// The expression evaluated first.
        first: ExprId,
        /// The expression evaluated after it.
        then: ExprId,
    },
    /// A `let`: a pattern, the expression it is bound to, and the body it is visible in.
    Let {
        /// The pattern the binding introduces.
        pat: PatId,
        /// The expression bound to the pattern.
        expr: ExprId,
        /// The expression the binding is visible in.
        body: ExprId,
    },
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

    /// The paths of the body, in the order they were allocated.
    pub fn paths(&self) -> &Arena<PathData> {
        &self.paths
    }

    /// The functions declared inside the body.
    pub fn local_functions(&self) -> &Arena<LocalFunctionData> {
        &self.local_functions
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

    /// Allocates a path.
    pub fn alloc_path(&mut self, path: PathData) -> PathId {
        self.paths.alloc(path)
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

    /// Finishes the body.
    ///
    /// # Panics
    ///
    /// Panics if the body has no root expression.
    /// A body is the body of an entity, and an entity that owns one has a root;
    /// a lowering that forgets to set it is a bug, not an input to recover from.
    pub fn finish(self) -> Body {
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

    use super::*;
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
        let name = builder.alloc_path(PathData::ident(
            Name::new("g"),
            PathAnchor::Item(entity("g")),
        ));
        let callee = builder.alloc_expr(Expr::Path(name));
        let literal = builder.alloc_expr(Expr::Literal(Literal::Int(1)));
        let pat = builder.alloc_pat(Pat::Bind(Name::new("x")));
        let binding = builder.alloc_path(PathData::ident(Name::new("x"), PathAnchor::Binding(pat)));
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

        builder.finish()
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
        let body = builder.finish();

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
        });
        let root = builder.alloc_expr(Expr::Literal(Literal::Str(Interned::new_str("hi"))));
        builder.set_local_function_root(local, root);
        builder.set_root(root);
        let body = builder.finish();

        assert_eq!(body.local_function_root(local), Some(root));
        assert_eq!(body[local].name, Name::new("helper"));
        // The expressions of a local entity live in the arena of the enclosing body.
        assert_eq!(body.exprs().len(), 1);
        assert_eq!(body.root(), root);
    }
}
