//! The inside of a function: its parameters, the names it binds, and its expressions.
//!
//! A body is lowered from the declaration it is written in and nothing else, together with
//! the names of the module it is in: the paths of a body that name a binding of the body are
//! anchored here, because only the body knows its own names, and the rest are anchored
//! against the names of the module.

use mlkc_hir_def::{
    BinaryOp, BodyBuilder, Expr, ExprId, ItemTree, Literal, Name, Pat, PatId, PathAnchor,
};
use mlkc_intern::Interned;
use mlkc_rowan::AstNode;
use mlkc_syntax::{
    BinExpr, CallExpr, Expr as ExprSyntax, FunDecl, LetExpr, Literal as LiteralSyntax,
    Pat as PatSyntax, SyntaxKind, SyntaxToken, VarExpr,
};
use mlkc_vfs::FileId;

use crate::{
    LoweredBody, LoweringDiag, decl, pat, path,
    syntax::{self, span},
};

/// Lowers the body of `decl`, a function of the module `tree` describes.
pub(crate) fn lower(tree: &ItemTree, decl: &FunDecl) -> LoweredBody {
    let mut lowering = BodyLowering {
        tree,
        builder: BodyBuilder::new(),
        bindings: Vec::new(),
        diagnostics: Vec::new(),
    };

    let root = lowering.function(decl);
    lowering.builder.set_root(root);

    let BodyLowering {
        builder,
        diagnostics,
        ..
    } = lowering;

    LoweredBody {
        body: builder.finish(),
        diagnostics,
    }
}

/// The lowering of one body.
struct BodyLowering<'a> {
    /// The surface of the module the body is in, which a path of the body is anchored against.
    tree: &'a ItemTree,
    builder: BodyBuilder,
    /// The names the body binds, the innermost last, with the pattern that binds each of them.
    bindings: Vec<(Name, PatId)>,
    diagnostics: Vec<LoweringDiag>,
}

impl BodyLowering<'_> {
    /// Lowers the parameters and the root expression of the function.
    fn function(&mut self, decl: &FunDecl) -> ExprId {
        self.parameters(decl);

        let Some(body) = decl.body() else {
            // A function that declares no body is what its signature says it is: the body of
            // it holds a missing expression, so that a caller has a body to hold and does not
            // need a case for a declaration that has none.
            return self.missing();
        };

        // A body whose expression is not there is what the parser reported;
        // the parameters of the function are read, and the body is missing.
        self.optional(body.expr().ok())
    }

    /// Lowers the parameters of the function into patterns of its body.
    ///
    /// A parameter is a name and a type in the signature of the function, and a binding of
    /// its body: the pattern is what the body binds the name with, and the signature is what
    /// a caller reads. A parameter the parser could not read is bound under a name that is
    /// not there, which is what a body cannot refer to.
    fn parameters(&mut self, decl: &FunDecl) {
        for parameter in decl::parameters(decl) {
            let name = decl::parameter_name(&parameter);
            let pat = self.builder.alloc_pat(Pat::Bind(name.clone()));
            self.builder.push_param(pat);

            if !name.is_missing() {
                self.bindings.push((name, pat));
            }
        }
    }

    /// Lowers one expression.
    fn expr(&mut self, expr: &ExprSyntax) -> ExprId {
        match expr {
            ExprSyntax::Literal(literal) => self.literal(literal),
            ExprSyntax::VarExpr(var) => self.variable(var),
            ExprSyntax::CallExpr(call) => self.call(call),
            ExprSyntax::BinExpr(binary) => self.binary(binary),
            ExprSyntax::LetExpr(let_expr) => self.let_expr(let_expr),
            // A parenthesized expression is the expression it holds: how the source is
            // grouped is the parser's business, and what it hands over is a tree already.
            ExprSyntax::ParenExpr(paren) => self.optional(paren.expr().ok()),
            ExprSyntax::BogusExpr(_) => self.missing(),
        }
    }

    /// Lowers an expression the syntax may not hold.
    fn optional(&mut self, expr: Option<ExprSyntax>) -> ExprId {
        match expr {
            Some(expr) => self.expr(&expr),
            None => self.missing(),
        }
    }

    /// An expression that is not there.
    fn missing(&mut self) -> ExprId {
        self.builder.alloc_expr(Expr::Missing)
    }

    /// Lowers a reference to a name.
    fn variable(&mut self, var: &VarExpr) -> ExprId {
        let name = syntax::name(var.name());
        let anchor = self.anchor(&name);
        let path = self.builder.alloc_path(path::ident(name, anchor));

        self.builder.alloc_expr(Expr::Path(path))
    }

    /// Lowers a call.
    fn call(&mut self, call: &CallExpr) -> ExprId {
        let callee = self.optional(call.function().ok());
        let args = call
            .arguments()
            .syntax()
            .children()
            .filter_map(ExprSyntax::cast)
            .map(|arg| self.expr(&arg))
            .collect();

        self.builder.alloc_expr(Expr::Call { callee, args })
    }

    /// Lowers a binary operation.
    fn binary(&mut self, binary: &BinExpr) -> ExprId {
        let operator = binary
            .operator_token()
            .ok()
            .and_then(|token| operator(token.kind()));

        let Some(op) = operator else {
            // A binary expression is built around the operator that was read, so a tree
            // without one is not a tree the parser makes: a reader of the HIR is handed a
            // missing expression rather than an operation the source does not have.
            return self.missing();
        };

        let lhs = self.optional(binary.lhs().ok());
        let rhs = self.optional(binary.rhs().ok());

        self.builder.alloc_expr(Expr::Binary { lhs, op, rhs })
    }

    /// Lowers a `let`: the pattern it binds, the expression it is bound to, and the body the
    /// binding is visible in.
    fn let_expr(&mut self, let_expr: &LetExpr) -> ExprId {
        // The pattern is lowered before the expression it is bound to, and the names it binds
        // are added to the body only for the expression they are visible in:
        // a binding is not one of itself.
        let (pat, names) = self.pattern(let_expr.pat().ok());
        let expr = self.optional(let_expr.expr().ok());

        let mark = self.bindings.len();
        self.bindings
            .extend(names.into_iter().map(|name| (name, pat)));
        let body = self.optional(let_expr.body().ok());
        self.bindings.truncate(mark);

        self.builder.alloc_expr(Expr::Let { pat, expr, body })
    }

    /// Lowers a pattern, and reads the names it binds.
    fn pattern(&mut self, pat: Option<PatSyntax>) -> (PatId, Vec<Name>) {
        let Some(pat) = pat else {
            return (self.builder.alloc_pat(Pat::Missing), Vec::new());
        };

        let lowered = pat::pat(&pat);
        let names = pat::bindings(&lowered);
        let id = self.builder.alloc_pat(lowered);

        (id, names)
    }

    /// Lowers a literal.
    fn literal(&mut self, literal: &LiteralSyntax) -> ExprId {
        let expr = match literal {
            LiteralSyntax::IntLiteral(int) => {
                let Some(token) = int.value_token().ok() else {
                    return self.missing();
                };

                match token.text_trimmed().parse::<i64>() {
                    Ok(value) => Expr::Literal(Literal::Int(value)),
                    // The HIR holds an integer literal as a 64-bit value, and the module
                    // wrote one that does not fit: the value is not there, and the parse says
                    // the literal was.
                    Err(_) => {
                        let message = "the integer literal does not fit a 64-bit integer";
                        let diagnostic =
                            LoweringDiag::new(message, span(self.file(), int.syntax()));
                        self.diagnostics.push(diagnostic);

                        return self.missing();
                    },
                }
            },
            LiteralSyntax::StringLiteral(string) => {
                let Some(token) = string.value_token().ok() else {
                    return self.missing();
                };

                Expr::Literal(Literal::Str(Interned::new_str(string_text(&token))))
            },
        };

        self.builder.alloc_expr(expr)
    }

    /// What a name in the body denotes: a binding if the body has one, and otherwise what the
    /// module declares.
    ///
    /// The bindings come first, and the innermost of them: a `let` shadows a parameter of the
    /// same name, which is the order the bindings are held in.
    fn anchor(&self, name: &Name) -> PathAnchor {
        match self.binding(name) {
            Some(pat) => PathAnchor::Binding(pat),
            None => self.tree.scope().anchor(name),
        }
    }

    /// The innermost binding of a name in the body, if it binds one.
    fn binding(&self, name: &Name) -> Option<PatId> {
        self.bindings
            .iter()
            .rev()
            .find(|(bound, _)| bound == name)
            .map(|(_, pat)| *pat)
    }

    /// The file the body was read from, which is what a diagnostic points into.
    fn file(&self) -> FileId {
        self.tree.module().0
    }
}

/// The text a string literal holds, without the quotes around it.
///
/// The language has no escapes yet: the text of the literal is the text of the string, and a
/// literal the lexer could not close keeps the quote it holds.
fn string_text(token: &SyntaxToken) -> &str {
    let text = token.text_trimmed();

    text.strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text)
}

/// The operator a token is, if the language has one.
fn operator(kind: SyntaxKind) -> Option<BinaryOp> {
    let operator = match kind {
        SyntaxKind::PLUS => BinaryOp::Add,
        SyntaxKind::MINUS => BinaryOp::Sub,
        SyntaxKind::STAR => BinaryOp::Mul,
        SyntaxKind::SLASH => BinaryOp::Div,
        SyntaxKind::EQ2 => BinaryOp::Eq,
        SyntaxKind::BANG_EQ => BinaryOp::Ne,
        SyntaxKind::LT => BinaryOp::Lt,
        SyntaxKind::LT_EQ => BinaryOp::Le,
        SyntaxKind::GT => BinaryOp::Gt,
        SyntaxKind::GT_EQ => BinaryOp::Ge,
        SyntaxKind::AND2 => BinaryOp::And,
        SyntaxKind::OR2 => BinaryOp::Or,
        _ => return None,
    };

    Some(operator)
}
