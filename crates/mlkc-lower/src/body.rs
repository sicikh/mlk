//! The inside of a function: its parameters, the names it binds, and its expressions.
//!
//! A body is lowered from the declaration it is written in and nothing else, together with
//! the names of the module it is in: the paths of a body that name a binding of the body are
//! anchored here, because only the body knows its own names, and the rest are anchored
//! against the names of the module.

use mlkc_hir_def::{
    BinaryOp, BodyBuilder, Expr, ExprId, ItemTree, Literal, Name, Namespace, Pat, PatId,
    PathAnchor, PathData, UnaryOp,
};
use mlkc_intern::Interned;
use mlkc_rowan::AstNode;
use mlkc_span::Span;
use mlkc_syntax::{
    BinExpr, CallExpr, Expr as ExprSyntax, FunDecl, LetExpr, Literal as LiteralSyntax,
    Pat as PatSyntax, Path as PathSyntax, PathExpr, SyntaxKind, SyntaxToken, TextRange, TextSize,
    UnaryExpr, inner_string_text,
};
use mlkc_vfs::FileId;

use crate::{
    LoweredBody, LoweringDiag, LoweringError, decl, pat, path, source_map::BodySourceMap,
    syntax::span,
};

/// Lowers the body of `decl`, a function of the module `tree` describes, or nothing if the
/// declaration declares no body.
pub(crate) fn lower(tree: &ItemTree, decl: &FunDecl) -> Option<LoweredBody> {
    // A function that declares no body of its own has none: what it is is its signature, and a
    // caller that gets nothing does not have to tell the absence of a body from a missing
    // expression inside one.
    let body = decl.body()?;

    let mut lowering = BodyLowering {
        tree,
        builder: BodyBuilder::new(),
        bindings: Vec::new(),
        diagnostics: Vec::new(),
        source_map: BodySourceMap::default(),
    };

    lowering.parameters(decl);

    // A body whose expression is not there is what the parser reported; the parameters of the
    // function are read, and the body holds a missing expression.
    let root = lowering.optional(body.expr().ok());
    lowering.builder.set_root(root);

    let BodyLowering {
        builder,
        diagnostics,
        source_map,
        ..
    } = lowering;

    Some(LoweredBody {
        body: builder.finish(),
        diagnostics,
        source_map,
    })
}

/// The lowering of one body.
struct BodyLowering<'a> {
    /// The surface of the module the body is in, which a path of the body is anchored against.
    tree: &'a ItemTree,
    builder: BodyBuilder,
    /// The names the body binds, the innermost last, with the pattern that binds each of them.
    bindings: Vec<(Name, PatId)>,
    diagnostics: Vec<LoweringDiag>,
    /// Where each node of the body is written, read as the node is made.
    source_map: BodySourceMap,
}

impl BodyLowering<'_> {
    /// Lowers the parameters of the function into patterns of its body.
    ///
    /// A parameter is a pattern and a type in the signature of the function, and a binding of
    /// its body: the pattern is what the body receives the argument as, and the signature is
    /// what a caller reads. A pattern that binds no name --- the wildcard, or one the parser
    /// could not read --- binds nothing, and the argument is not reachable from the body.
    fn parameters(&mut self, decl: &FunDecl) {
        for parameter in decl::parameters(decl) {
            let lowered = decl::pattern(&parameter);
            let names = pat::bindings(&lowered);
            let pat = self.builder.alloc_pat(lowered);

            if let Some(pattern) = parameter.as_ref().and_then(|it| it.pat().ok()) {
                self.source_map
                    .set_pat(pat, pattern.syntax().text_trimmed_range());
            }

            self.builder.push_param(pat);
            self.bindings
                .extend(names.into_iter().map(|name| (name, pat)));
        }
    }

    /// Lowers one expression.
    ///
    /// The range of the syntax is read as the node is made: this is the one place that knows
    /// which expression a node of the syntax became, which is what a host marks a buffer by.
    fn expr(&mut self, expr: &ExprSyntax) -> ExprId {
        let id = match expr {
            ExprSyntax::Literal(literal) => self.literal(literal),
            ExprSyntax::PathExpr(path) => self.path_expr(path),
            ExprSyntax::CallExpr(call) => self.call(call),
            ExprSyntax::UnaryExpr(unary) => self.unary_expr(unary),
            ExprSyntax::BinExpr(binary) => self.binary(binary),
            ExprSyntax::LetExpr(let_expr) => self.let_expr(let_expr),
            // A parenthesized expression is the expression it holds: how the source is
            // grouped is the parser's business, and what it hands over is a tree already.
            ExprSyntax::ParenExpr(paren) => self.optional(paren.expr().ok()),
            ExprSyntax::BogusExpr(_) => self.missing(),
        };

        self.source_map
            .set_expr(id, expr.syntax().text_trimmed_range());

        id
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

    /// Lowers an expression that names something by a path.
    fn path_expr(&mut self, expr: &PathExpr) -> ExprId {
        // A path that is not there is what a broken declaration holds: the expression is one
        // the body cannot read, and the parse is what reported the mistake.
        let Ok(path) = expr.path() else {
            return self.missing();
        };

        let written = path.syntax().text_trimmed_range();
        let data = self.path_data(&path);
        let id = self.builder.alloc_path(data);

        self.source_map.set_path(id, written);

        self.builder.alloc_expr(Expr::Path(id))
    }

    /// The path of an expression, anchored to what the body can tell of it.
    ///
    /// A path of one name is a name of the body if the body binds one --- a parameter or
    /// a `let` is what a bare name denotes --- and otherwise a name of the module, read where
    /// a value belongs. A path of several names is a path of the module: what its root denotes
    /// is what the names after it are names inside, and nothing a body binds has names inside
    /// it. A path rooted at the project is none of a body's business either: what it names is
    /// read inside the project.
    fn path_data(&mut self, path: &PathSyntax) -> PathData {
        let mut data = path::data(path, self.file(), &mut self.diagnostics);

        // What the root of the path denotes is already decided when it is the project, and a
        // root that is not a name is what a broken path is read with.
        if matches!(data.anchor, PathAnchor::Project) {
            return data;
        }

        let Some(name) = data.root.name() else {
            return data;
        };

        let anchor = match data.segments.as_slice() {
            [] => self.anchor(name),
            _ => self.tree.scope().anchor(name, Namespace::Module),
        };
        data.anchor = anchor;

        data
    }

    /// The value a string literal holds, with the escapes of the language decoded.
    ///
    /// An escape the language has no meaning for is what a reader is told about, and what the
    /// module wrote stands for the value there: a literal that holds one is read as far as it
    /// can be read, and the rest of it is its value.
    fn string_value(&mut self, token: &SyntaxToken) -> String {
        // What a literal holds is the text between its quotes, which the syntax of the
        // language reads; the range of that text is where the escapes are in the file.
        let body = inner_string_text(token);
        let range = body.source_range(token.text_trimmed_range());

        let mut value = String::with_capacity(body.len().into());
        let mut characters = body.char_indices();

        while let Some((at, character)) = characters.next() {
            if character != '\\' {
                value.push(character);
                continue;
            }

            // A literal the lexer could not close ends at a backslash: it is not an escape
            // of anything, and it is kept as the text it is.
            let Some((_, escaped)) = characters.next() else {
                value.push(character);
                break;
            };

            match escape(escaped) {
                Some(character) => value.push(character),
                None => {
                    // What the module wrote stands for the value where the language has no
                    // escape: the text keeps the sequence, and a reader is told about it.
                    let sequence = format!("\\{escaped}");
                    let start = range.start() + offset(at);
                    let error = LoweringError::UnknownEscape {
                        escape: sequence.clone(),
                    };
                    let diagnostic = LoweringDiag::new(
                        error,
                        Span::new(self.file(), TextRange::at(start, TextSize::of(&sequence))),
                    );

                    self.diagnostics.push(diagnostic);
                    value.push_str(&sequence);
                },
            }
        }

        value
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

    /// Lowers a sign written in front of an expression.
    fn unary_expr(&mut self, unary: &UnaryExpr) -> ExprId {
        let operator = unary
            .operator_token()
            .ok()
            .and_then(|token| unary_operator(token.kind()));

        let Some(op) = operator else {
            // A unary expression is built around the sign that was read, so a tree without
            // one is not a tree the parser makes: a reader of the HIR is handed a missing
            // expression rather than a sign the source does not have.
            return self.missing();
        };

        let operand = self.optional(unary.operand().ok());

        self.builder.alloc_expr(Expr::Unary { op, operand })
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

        self.source_map
            .set_pat(id, pat.syntax().text_trimmed_range());

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
                        let error = LoweringError::IntegerLiteralTooLarge {
                            literal: token.text_trimmed().to_owned(),
                        };
                        let diagnostic = LoweringDiag::new(error, span(self.file(), int.syntax()));
                        self.diagnostics.push(diagnostic);

                        return self.missing();
                    },
                }
            },
            LiteralSyntax::StringLiteral(string) => {
                let Some(token) = string.value_token().ok() else {
                    return self.missing();
                };

                let value = self.string_value(&token);

                Expr::Literal(Literal::Str(Interned::new_str(&value)))
            },
        };

        self.builder.alloc_expr(expr)
    }

    /// What a name in the body denotes: a binding if the body has one, and otherwise what the
    /// module declares.
    ///
    /// The bindings come first, and the innermost of them: a `let` shadows a parameter of the
    /// same name, which is the order the bindings are held in. A name that is not a binding of
    /// the body is read where a value belongs: a path of an expression names a value, and what
    /// the value is is worked out later.
    fn anchor(&self, name: &Name) -> PathAnchor {
        match self.binding(name) {
            Some(pat) => PathAnchor::Binding(pat),
            None => self.tree.scope().anchor(name, Namespace::Value),
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

/// The sign a token is, if the language has one.
fn unary_operator(kind: SyntaxKind) -> Option<UnaryOp> {
    let operator = match kind {
        SyntaxKind::MINUS => UnaryOp::Neg,
        SyntaxKind::PLUS => UnaryOp::Pos,
        _ => return None,
    };

    Some(operator)
}

/// The character an escape stands for, if the language has an escape of it.
///
/// The escapes are the ones a string cannot hold as they are: the quote that would close it,
/// the backslash that opens an escape, and the line breaks and the tab that a source is read
/// in rather than written in.
fn escape(character: char) -> Option<char> {
    let escaped = match character {
        '\\' => '\\',
        '"' => '"',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '0' => '\0',
        _ => return None,
    };

    Some(escaped)
}

/// A byte offset of a file, as the size the tree reads positions in.
///
/// A file is at most `u32::MAX` bytes, which is the size the tree holds it in, so an offset
/// it holds fits here; one that does not is an offset no file has.
fn offset(at: usize) -> TextSize {
    TextSize::try_from(at).unwrap_or(TextSize::from(u32::MAX))
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

#[cfg(test)]
mod tests {
    use mlkc_hir_def::{ItemLocLike, ModuleId, Name, Namespace, PathAnchor, Prelude};
    use mlkc_rowan::AstNode;
    use mlkc_syntax::ModuleRoot;
    use mlkc_vfs::FileId;

    use super::*;

    /// A declaration that declares no body, and one that declares a body.
    const SOURCE: &str = "\
@extern
fun add(left: Int, right: Int): Int

fun main(): Int = 1
";

    /// The declaration of the function `name`, found the way a caller that holds an item tree
    /// and the syntax it was lowered from finds it.
    fn declaration(root: &ModuleRoot, lowered: &crate::LoweredModule, name: &str) -> FunDecl {
        let anchor = lowered
            .item_tree
            .scope()
            .anchor(&Name::new(name), Namespace::Value);
        let PathAnchor::Item(item) = anchor else {
            panic!("the module to declare an entity named `{name}`");
        };
        let position = lowered
            .item_tree
            .syntax_loc(item.item)
            .expect("the entity to have a position");
        let syntax = crate::syntax_at(root, position).expect("the declaration to be there");

        FunDecl::cast(syntax).expect("a function declaration")
    }

    #[test]
    fn a_declaration_that_declares_no_body_has_no_body() {
        let parsed = mlkc_parser::parse(SOURCE);
        let root = parsed.tree::<ModuleRoot>();
        let lowered = crate::lower_module(ModuleId(FileId::from_raw(0)), &root, &Prelude::none());

        // The work list holds the declaration of a body.
        assert_eq!(lowered.bodies.len(), 1);
        assert_eq!(
            lowered.bodies[0].owner.item().name(),
            Some(&Name::new("main")),
        );

        // The declaration that declares none is an entity of the module all the same, and it
        // has no body to lower: a caller is told what is there rather than handed a body that
        // is not.
        let add = declaration(&root, &lowered, "add");
        assert!(crate::lower_body(&lowered.item_tree, &add).is_none());
        assert_eq!(lowered.item_tree.scope().len(), 2);

        // The declaration that declares one has it, and it is the body of the expression.
        let main = declaration(&root, &lowered, "main");
        let body = crate::lower_body(&lowered.item_tree, &main).expect("a body");
        assert!(body.diagnostics.is_empty());
        assert_eq!(body.body[body.body.root()], Expr::Literal(Literal::Int(1)),);
    }
}
