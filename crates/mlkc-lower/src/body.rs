//! The inside of a function: its parameters, the names it binds, and its expressions.
//!
//! A body is lowered from the declaration it is written in and nothing else, together with
//! the names of the module it is in: the paths of a body that name a binding of the body are
//! anchored here, because only the body knows its own names, and the rest are anchored
//! against the names of the module.

use mlkc_hir_def::{
    Attributes, BinaryOp, BodyBuilder, Expr, ExprId, IfArm, ItemKind, ItemTree, LambdaParam,
    Literal, LocalDefId, LocalFunctionData, Name, Namespace, Pat, PatId, PathAnchor, PathData,
    UnaryOp,
};
use mlkc_intern::Interned;
use mlkc_rowan::AstNode;
use mlkc_span::Span;
use mlkc_syntax::{
    AnyParameter, BinExpr, CallExpr, Expr as ExprSyntax, FieldExpr, FunDecl, IfArm as IfArmSyntax,
    IfExpr as IfExprSyntax, LambdaExpr as LambdaExprSyntax, LetExpr, Literal as LiteralSyntax,
    LocalExpr as LocalExprSyntax, ModuleItem, Pat as PatSyntax, Path as PathSyntax, PathExpr,
    PipeExpr, PlaceholderExpr, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, UfcsCall,
    UnaryExpr, inner_string_text,
};
use mlkc_vfs::FileId;

use crate::{
    LoweredBody, LoweringDiag, LoweringError, decl, pat, path,
    source_map::BodySourceMap,
    syntax::{name, span},
    ty,
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
        names: Vec::new(),
        outer: None,
        pipes: 0,
        pipe: None,
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
        mut diagnostics,
        source_map,
        ..
    } = lowering;

    // The names the types written in the body are rooted at: a type is read where a type belongs,
    // so a name of the module, an import of it, or a project is what one may be rooted at, and
    // a name that is none of them is what the module is told about ([`path::unresolved`]).
    let mut names = Vec::new();
    path::written_names(body.syntax(), &mut names);
    diagnostics.extend(path::unresolved(&names, tree, tree.module().0));

    // A reader reads a body from its top, and what lowering found is read the same way: the
    // rules are checked in the order the expressions are walked, and the mistakes are ordered by
    // where they are written rather than by when they were found.
    diagnostics.sort_by_key(|diagnostic| diagnostic.span().range.start());

    Some(LoweredBody {
        body: builder.finish(tree.scope()),
        diagnostics,
        source_map,
    })
}

/// The lowering of one body.
struct BodyLowering<'a> {
    /// The surface of the module the body is in, which a path of the body is anchored against.
    tree: &'a ItemTree,
    builder: BodyBuilder,
    /// The names the body declares, the innermost last, with what each of them denotes: a
    /// binding the body made, or a function declared inside it.
    names: Vec<(Name, PathAnchor)>,
    /// The environment of the function declared in a `local` that is being lowered, if one is:
    /// the height the names of the function itself start at, and the name of the function.
    ///
    /// A function declared inside a body is given its own parameters only: a binding below the
    /// height is a name of the body that declares the function, and the body of the function may
    /// not read one.
    outer: Option<(usize, Name)>,
    /// How many pipelines the body has lowered, which is what numbers the bindings of them.
    pipes: usize,
    /// The value the innermost pipeline passes, if one is being read: the name the binding of
    /// it is under, and the pattern that binds it. A `_` among the arguments of a step is that
    /// binding, and a `_` written anywhere else is not a value.
    pipe: Option<(Name, PatId)>,
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
            self.names.extend(
                names
                    .into_iter()
                    .map(|name| (name, PathAnchor::Binding(pat))),
            );
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
            ExprSyntax::UfcsCall(call) => self.ufcs_call(call),
            ExprSyntax::FieldExpr(field) => self.field_expr(field),
            ExprSyntax::UnaryExpr(unary) => self.unary_expr(unary),
            ExprSyntax::BinExpr(binary) => self.binary(binary),
            ExprSyntax::PipeExpr(pipe) => self.pipe_expr(pipe),
            ExprSyntax::PlaceholderExpr(place) => self.placeholder(place),
            ExprSyntax::IfExpr(if_expr) => self.if_expr(if_expr),
            ExprSyntax::LetExpr(let_expr) => self.let_expr(let_expr),
            ExprSyntax::LambdaExpr(lambda) => self.lambda_expr(lambda),
            ExprSyntax::LocalExpr(local) => self.local_expr(local),
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

        self.path_of(&path)
    }

    /// Lowers a path into the expression that names what the path names.
    ///
    /// A path is what a value is named by, and the callee of a call written with a dot is
    /// a path outside an expression: a caller that holds one reads it here, and is handed an
    /// expression written where the path is.
    fn path_of(&mut self, path: &PathSyntax) -> ExprId {
        let written = path.syntax().text_trimmed_range();
        let data = self.path_data(path);
        let id = self.builder.intern_path(data);

        let expr = self.builder.alloc_expr(Expr::Path(id));
        self.source_map.set_expr(expr, written);

        expr
    }

    /// The path of an expression, anchored to what the body can tell of it.
    ///
    /// A path of one name is a name of the body if the body binds one --- a parameter or
    /// a `let` is what a bare name denotes --- and otherwise a name of the module, read where
    /// a value belongs. A path of several names is a path of the module: what its root denotes
    /// is what the names after it are names inside, and nothing a body binds has names inside
    /// it. A path rooted at a project is none of a body's business either: what the keyword
    /// names is the project the module is written in, and a name is read against what the
    /// module may name, which is the scope of the module as much as the body's own bindings.
    ///
    /// A path the body can give no meaning to at all is what the body reports: a name that is
    /// no binding of it, no name of the module, and no project the module may name is a name
    /// nothing denotes ([`BodyLowering::report_unresolved`]).
    fn path_data(&mut self, path: &PathSyntax) -> PathData {
        let mut data = path::data(path, self.file(), &mut self.diagnostics);

        // What the root of the path denotes is already decided when it is the project, and a
        // root that is not a name is what a broken path is read with.
        if matches!(data.anchor, PathAnchor::Project(_)) {
            return data;
        }

        let Some(name) = data.root.name() else {
            return data;
        };

        let anchor = match data.segments.as_slice() {
            [] => self.anchor(path, name),
            _ => self.tree.scope().anchor(name, Namespace::Module),
        };
        data.anchor = anchor;

        if matches!(data.anchor, PathAnchor::Unresolved) {
            self.report_unresolved(path, name);
        }

        data
    }

    /// Reports the name a path is rooted at when nothing the body knows denotes it.
    ///
    /// A name an expression is written with is a binding of the body, a name of the module, or
    /// the name of a project the module may name ([`path::names_a_name`]). A name the module
    /// knows in another namespace is a mistake of its own, which the stage that holds the scopes
    /// reads, and so is the name of a binding a path goes on after: what the names after a name
    /// denote is a name *inside* what it denotes, which the language has yet to have. What is
    /// reported here is a name that is not there at all, which the module alone decides
    /// ([ADR-0004]).
    ///
    /// [adr-0004]: ../../docs/adr/0004-module-system.md
    fn report_unresolved(&mut self, path: &PathSyntax, name: &Name) {
        if self.declares(name) || path::names_a_name(self.tree, name) {
            return;
        }

        // A path the parser could not read has no name where it is rooted, and the parse is what
        // a reader is told about it.
        let Some(written) = path::root_name(path) else {
            return;
        };

        let error = LoweringError::UnresolvedName { name: name.clone() };
        let diagnostic = LoweringDiag::new(error, span(self.file(), &written.node));

        self.diagnostics.push(diagnostic);
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

    /// Lowers a call: the callee, and the arguments in the order they are written.
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

    /// Lowers a call written with the receiver before the callee: `receiver.function(a)`.
    ///
    /// The spelling means the call `function(receiver, a)`: the receiver is the first
    /// argument, and the callee is the path written after the dot. The dot is a spelling and
    /// stops here — what the HIR holds is the call it means — while the receiver and the
    /// arguments keep the places they are written at.
    fn ufcs_call(&mut self, call: &UfcsCall) -> ExprId {
        let receiver = self.optional(call.receiver().ok());
        let callee = match call.callee() {
            Ok(path) => self.path_of(&path),
            Err(_) => self.missing(),
        };

        let mut args = vec![receiver];
        args.extend(
            call.arguments()
                .syntax()
                .children()
                .filter_map(ExprSyntax::cast)
                .map(|arg| self.expr(&arg)),
        );

        self.builder.alloc_expr(Expr::Call { callee, args })
    }

    /// Lowers a field read: the value, and the name of the field.
    ///
    /// The field is not resolved here: what fields the type of the value has is what the type
    /// is, and the name is kept as the module wrote it, for the stage that holds the types.
    fn field_expr(&mut self, expr: &FieldExpr) -> ExprId {
        let receiver = self.optional(expr.receiver().ok());
        let field = name(expr.field());

        self.builder.alloc_expr(Expr::Field { receiver, field })
    }

    /// Lowers a pipeline: the value, the binding the lowering makes up for it, and the call
    /// the step is.
    ///
    /// What the spelling means is `let v = x in f(a, v)`: the value is bound before the call,
    /// and every `_` among the arguments of the call is the binding. The binding is a node the
    /// lowering makes up --- under a name no module can write --- and what it binds is the
    /// value, which is written on the left, so that is the piece of the text it reads as.
    fn pipe_expr(&mut self, pipe: &PipeExpr) -> ExprId {
        let value = self.optional(pipe.lhs().ok());

        let name = self.pipeline_name();
        let pat = self.builder.alloc_pat(Pat::Bind(name.clone()));

        if let Some(written) = self.source_map.expr(value) {
            self.source_map.set_pat(pat, written);
        }

        // The step is read with the binding in scope: a `_` among the arguments of its call is
        // the value the pipeline passes, and a step written inside the arguments of another
        // one binds its own value. The binding is a name of the body the step is written in, and
        // it is held with the other names of the body for as long as the step is read.
        let outer = self.pipe.replace((name.clone(), pat));
        let mark = self.names.len();
        self.names.push((name, PathAnchor::Binding(pat)));

        let step = self.optional(pipe.step().ok());

        self.names.truncate(mark);
        self.pipe = outer;

        self.builder.alloc_expr(Expr::Let {
            pat,
            expr: value,
            body: step,
        })
    }

    /// The name of the binding of the next pipeline: a name no module can write.
    ///
    /// A binding the lowering makes up is not a name a reader wrote, and a name a module could
    /// write is a name it could also use. The binding of a pipe is under the namespace of the
    /// compiler instead --- `<mlkc@pipeline-0>` --- and what a name is made of is letters,
    /// digits, `_` and `-`: the brackets, the `@` and the number are what say that the lowering
    /// wrote the name, and which pipeline of the body the binding belongs to.
    fn pipeline_name(&mut self) -> Name {
        let name = Name::new(&format!("<mlkc@pipeline-{}>", self.pipes));

        self.pipes += 1;

        name
    }

    /// Lowers a `_`: a reference to the value the innermost pipeline passes.
    ///
    /// A place is where that value goes, and what it is is the binding the step bound: a path
    /// of one name --- the name the lowering gave the binding --- anchored to the pattern that
    /// binds it. The place itself says nothing else, and the expression it becomes reads as the
    /// `_` it stands for, which is what a host marks for it.
    ///
    /// The binding a place stands for is a binding of the body, and a function declared in
    /// a `local` may not read one: a `_` in the body of one that names the value of a pipeline
    /// the enclosing body wrote is reported like any other outer name
    /// ([`BodyLowering::report_outer_binding`]).
    fn placeholder(&mut self, place: &PlaceholderExpr) -> ExprId {
        // A `_` is a value only among the arguments of a step, and one written anywhere else
        // is what the parser reported: a tree that holds one has no binding for it to be.
        let Some((name, pat)) = self.pipe.clone() else {
            return self.missing();
        };

        let index = self
            .names
            .iter()
            .position(|(_, anchor)| matches!(anchor, PathAnchor::Binding(bound) if *bound == pat));

        if let Some((height, function)) = self.outer.clone()
            && index.is_some_and(|index| index < height)
        {
            let written = span(self.file(), place.syntax());
            self.report_outer_binding(written, &function, None);
        }

        let path = self
            .builder
            .intern_path(PathData::ident(name, PathAnchor::Binding(pat)));

        self.builder.alloc_expr(Expr::Path(path))
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

    /// Lowers an `if`: the condition, the expression it selects, the `elif` arms written after
    /// it, and the expression selected when no condition holds.
    ///
    /// Nothing of the spelling is kept but the choices: every part is an expression, and what
    /// a value is chosen by is a question about types, which the HIR does not answer.
    fn if_expr(&mut self, if_expr: &IfExprSyntax) -> ExprId {
        let cond = self.optional(if_expr.condition().ok());
        let then_ = self.optional(if_expr.then_branch().ok());
        let arms = if_expr
            .arms()
            .syntax()
            .children()
            .filter_map(IfArmSyntax::cast)
            .map(|arm| {
                IfArm {
                    cond: self.optional(arm.condition().ok()),
                    body: self.optional(arm.body().ok()),
                }
            })
            .collect();
        let otherwise = if_expr
            .else_branch()
            .map(|branch| self.optional(branch.expr().ok()));

        self.builder.alloc_expr(Expr::If {
            cond,
            then_,
            arms,
            otherwise,
        })
    }

    /// Lowers a `let`: the pattern it binds, the expression it is bound to, and the body the
    /// binding is visible in.
    fn let_expr(&mut self, let_expr: &LetExpr) -> ExprId {
        // The pattern is lowered before the expression it is bound to, and the names it binds
        // are added to the body only for the expression they are visible in:
        // a binding is not one of itself.
        let (pat, names) = self.pattern(let_expr.pat().ok());
        let expr = self.optional(let_expr.expr().ok());

        let mark = self.names.len();
        self.names.extend(
            names
                .into_iter()
                .map(|name| (name, PathAnchor::Binding(pat))),
        );
        let body = self.optional(let_expr.body().ok());
        self.names.truncate(mark);

        self.builder.alloc_expr(Expr::Let { pat, expr, body })
    }

    /// Lowers a lambda: the parameters it binds, the body it computes, and the bindings of the
    /// enclosing body its body reads --- what the lambda captures.
    ///
    /// A parameter is the parameter of a function declaration: a pattern, and the type it takes
    /// where one is written. The parameters are the names the body binds: they are pushed for
    /// the body the way the pattern of a `let` is pushed for the expression after it, and taken
    /// back after it, so that a name of the enclosing body stays what it is outside the lambda.
    /// What the body reads of the enclosing body is what the lambda carries as its environment,
    /// which is what [`BodyBuilder::captures`] works out ([ADR-0018]).
    ///
    /// [ADR-0018]: ../../docs/adr/0018-values-as-words.md
    fn lambda_expr(&mut self, lambda: &LambdaExprSyntax) -> ExprId {
        let file = self.file();
        let mut params = Vec::new();
        let mut bound = Vec::new();

        for parameter in lambda.parameters().syntax().children() {
            // A parameter the parser could not read at all is one of them, and `None` stands
            // for it, as it does among the parameters of a function declaration: the arity is
            // what the module wrote, and the pattern of the broken one is not there.
            let parameter = match AnyParameter::cast(parameter) {
                Some(AnyParameter::Parameter(parameter)) => Some(parameter),
                Some(AnyParameter::BogusParameter(_)) | None => None,
            };

            let (pat, names) = self.pattern(parameter.as_ref().and_then(|it| it.pat().ok()));
            let ty = parameter.as_ref().and_then(|parameter| {
                parameter.type_annotation().map(|annotation| {
                    ty::type_ref(annotation.ty().ok(), file, &mut self.diagnostics)
                })
            });

            params.push(LambdaParam { pat, ty });
            bound.extend(
                names
                    .into_iter()
                    .map(|name| (name, PathAnchor::Binding(pat))),
            );
        }

        let mark = self.names.len();
        self.names.extend(bound);

        let body = self.optional(lambda.body().ok());

        self.names.truncate(mark);

        let pats: Vec<PatId> = params.iter().map(|param| param.pat).collect();
        let captures = self.builder.captures(body, &pats);

        self.builder.alloc_expr(Expr::Lambda {
            params,
            body,
            captures,
        })
    }

    /// Lowers a `local`: the items it declares, and the expression they are visible in.
    ///
    /// The items are the items of a module written where a value belongs ([`ModuleItem`]):
    /// a function declared here is an entity of the enclosing body ([ADR-0010]), and what it is
    /// made of --- its signature, the patterns of its parameters, its root --- is read from the
    /// body, with the expressions of it living in the arenas of the body that wrote it
    /// ([ADR-0003]).
    ///
    /// Every name is added before a body is lowered: an item sees the items written next to it
    /// and itself, which is what a function that calls itself is written with.
    ///
    /// A function declared here is given its own parameters only: the environment of the item is
    /// the height its own names start at, and a path of its body that names a binding below it is
    /// what the body may not read ([`BodyLowering::report_outer_binding`]).
    ///
    /// [ADR-0003]: ../../docs/adr/0003-id-based-ir.md
    /// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
    fn local_expr(&mut self, local: &LocalExprSyntax) -> ExprId {
        let file = self.file();
        let mark = self.names.len();
        let mut declared = Vec::new();
        let mut items = Vec::new();

        // The items are read before the body of any of them: a name is visible in the whole of
        // the `local`, and not only after the item that declares it.
        for node in local.items().syntax().children() {
            let Some(item) = ModuleItem::cast(node) else {
                continue;
            };

            // The items are the items of a module, and the language does not hold all of them
            // inside a body yet: a function is what a `local` declares today, and the rest is
            // what a reader is told about.
            let decl = match item {
                ModuleItem::FunDecl(decl) => decl,
                ModuleItem::TypeDecl(decl) => {
                    self.report_unread_local_item(ItemKind::Class, decl.syntax());
                    continue;
                },
                ModuleItem::UseDecl(decl) => {
                    self.report_unread_local_item(ItemKind::Use, decl.syntax());
                    continue;
                },
                // An item the parser could not read is what the parse reported, and the HIR
                // holds what the module does say: a broken item says nothing.
                ModuleItem::BogusDecl(_) => continue,
            };

            let name = name(decl.name());

            // The attributes and the visibility of a declaration are read on the items of
            // a module, and a function declared inside a body carries none the language reads.
            self.local_function_modifiers(&decl, &name);

            let signature = decl::signature(&decl, file, &mut self.diagnostics);
            let mut params = Vec::new();
            let mut bound = Vec::new();

            for parameter in decl::parameters(&decl) {
                let (pat, names) = self.pattern(parameter.as_ref().and_then(|it| it.pat().ok()));
                params.push(pat);
                bound.extend(
                    names
                        .into_iter()
                        .map(|name| (name, PathAnchor::Binding(pat))),
                );
            }

            // A function declared inside a body is not implemented elsewhere, the way a function
            // of a module may be: a `local` is where its body is, and one without a body is
            // a declaration nothing implements.
            if decl.body().is_none() {
                let error = LoweringError::LocalFunctionWithoutBody { name: name.clone() };
                let diagnostic = LoweringDiag::new(error, span(file, decl.syntax()));
                self.diagnostics.push(diagnostic);
            }

            let id = self.builder.declare_local_function(LocalFunctionData {
                name: name.clone(),
                signature,
                params,
            });

            self.names
                .push((name.clone(), PathAnchor::Local(LocalDefId::Function(id))));
            declared.push((id, decl, bound, name));
            items.push(LocalDefId::Function(id));
        }

        // The body of an item is lowered with the parameters of the item in scope, and with the
        // environment of the item fixed: a binding below the height where its own names start is
        // a name of the enclosing body, which the body of the item may not read.
        for (id, decl, bound, name) in declared {
            let height = self.names.len();
            let outer = self.outer.replace((height, name));

            self.names.extend(bound);
            let root = self.optional(decl.body().and_then(|body| body.expr().ok()));
            self.names.truncate(height);

            self.outer = outer;
            self.builder.set_local_function_root(id, root);
        }

        let body = self.optional(local.body().ok());
        self.names.truncate(mark);

        self.builder.alloc_expr(Expr::Local { items, body })
    }

    /// Reports an item of a `local` the language does not hold inside a body yet.
    fn report_unread_local_item(&mut self, kind: ItemKind, node: &SyntaxNode) {
        let error = LoweringError::LocalItemNotRead { kind };
        let diagnostic = LoweringDiag::new(error, span(self.file(), node));

        self.diagnostics.push(diagnostic);
    }

    /// Reports the attributes and the visibility a function declared inside a body carries.
    ///
    /// What an attribute of the language says --- what implements an entity, where a program
    /// begins --- is about an item of a module, and what `pub` reaches is a module as well:
    /// a function declared inside a body carries neither.
    fn local_function_modifiers(&mut self, decl: &FunDecl, function: &Name) {
        let file = self.file();
        let mut known = Attributes::default();

        for attribute in decl::attributes(&decl.attributes()) {
            let attribute_name = name(attribute.name());

            // An attribute the language has is one that belongs to an item of a module; one it
            // has not is a name nothing knows, wherever it is written.
            let error = if known.insert(&attribute_name) {
                LoweringError::LocalFunctionAttribute {
                    function: function.clone(),
                    attribute: attribute_name,
                }
            } else {
                LoweringError::UnknownAttribute {
                    name: attribute_name,
                }
            };

            let diagnostic = LoweringDiag::new(error, span(file, attribute.syntax()));
            self.diagnostics.push(diagnostic);
        }

        if let Some(token) = decl.visibility_token() {
            let error = LoweringError::LocalFunctionVisibility {
                function: function.clone(),
            };
            let diagnostic = LoweringDiag::new(error, Span::new(file, token.text_trimmed_range()));

            self.diagnostics.push(diagnostic);
        }
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
            LiteralSyntax::BoolLiteral(boolean) => {
                let value = match boolean.value().map(|token| token.kind()) {
                    Ok(SyntaxKind::TRUE_KW) => true,
                    Ok(SyntaxKind::FALSE_KW) => false,
                    // A truth value is built around the word it is written with, so a tree
                    // without one is not a tree the parser makes: a reader of the HIR is handed
                    // an expression that is not there rather than a value the source does not
                    // have.
                    _ => return self.missing(),
                };

                Expr::Literal(Literal::Bool(value))
            },
        };

        self.builder.alloc_expr(expr)
    }

    /// What a name in the body denotes: a binding or a function the body declares if it has one,
    /// and otherwise what the module declares.
    ///
    /// The names of the body come first, and the innermost of them: a `let` shadows a parameter
    /// of the same name, and a function declared in a `local` is shadowed by the binding of
    /// a `let` written inside it, which is the order the names are held in. A name that is not
    /// one of the body is read where a value belongs: a path of an expression names a value, and
    /// what the value is is worked out later.
    ///
    /// A name that is a binding below the height of the function declared in a `local` that is
    /// being lowered is a name of the enclosing body: a function declared inside a body is given
    /// its own parameters only, and reading one is what it may not do
    /// ([`BodyLowering::report_outer_binding`]).
    fn anchor(&mut self, path: &PathSyntax, name: &Name) -> PathAnchor {
        let found = self
            .names
            .iter()
            .enumerate()
            .rev()
            .find(|(_, (declared, _))| declared == name)
            .map(|(index, (_, anchor))| (index, anchor.clone()));

        let Some((index, anchor)) = found else {
            return self.tree.scope().anchor(name, Namespace::Value);
        };

        if let Some((height, function)) = self.outer.clone()
            && index < height
            && matches!(anchor, PathAnchor::Binding(_))
            && let Some(written) = path::root_name(path)
        {
            let written = span(self.file(), &written.node);
            self.report_outer_binding(written, &function, Some(name.clone()));
        }

        anchor
    }

    /// Reports a name of the enclosing body read inside a function declared in a `local`.
    ///
    /// A function declared inside a body is given its own parameters only: what the body that
    /// declares it binds --- its parameters, its `let`s, the values a lambda around it carries,
    /// and the value a pipeline of it passes --- is not part of it, and a name that reads one is
    /// a mistake.
    fn report_outer_binding(&mut self, written: Span, function: &Name, name: Option<Name>) {
        let error = LoweringError::LocalFunctionReadsOuterBinding {
            function: function.clone(),
            name,
        };
        let diagnostic = LoweringDiag::new(error, written);

        self.diagnostics.push(diagnostic);
    }

    /// Whether the body declares a name: a binding, or a function of a `local`.
    fn declares(&self, name: &Name) -> bool {
        self.names.iter().any(|(declared, _)| declared == name)
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
    use mlkc_vfs::{FileId, RelPathBuf};

    use super::{Expr, FunDecl, Literal};

    /// The path of the file these tests read a module from, which is what the module is called
    /// by: the modules here stand in no project, and a module of none is called by its name.
    fn relative() -> RelPathBuf {
        RelPathBuf::try_from("main.mlk").expect("a relative path")
    }

    /// A declaration that declares no body, and one that declares a body.
    const SOURCE: &str = "\
#[extern]
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
        let lowered = crate::lower_module(
            ModuleId(FileId::from_raw(0)),
            &root,
            &Prelude::none(),
            &[],
            relative().as_path(),
        );

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

    #[test]
    fn a_name_nothing_denotes_is_reported_where_the_expression_writes_it() {
        let source = "\
fun declared(value: Int): Int =
    value

fun main(): Int =
    let bound = declared(1) in
    bound + nope(2)
";
        let parsed = mlkc_parser::parse(source);
        let root = parsed.tree::<ModuleRoot>();
        let lowered = crate::lower_module(
            ModuleId(FileId::from_raw(0)),
            &root,
            Prelude::standard(),
            &[],
            relative().as_path(),
        );

        let main = declaration(&root, &lowered, "main");
        let body = crate::lower_body(&lowered.item_tree, &main).expect("a body");

        // What an expression may be rooted at is a binding of the body, a name of the module,
        // and a project the module may name; a name that is none of them is reported where it is
        // written, and the body is the only stage that reads it.
        let messages: Vec<String> = body
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.error().message())
            .collect();

        assert_eq!(messages, [
            "the name `nope` is not a name of this module, and names no project",
        ]);
    }

    #[test]
    fn a_name_the_body_knows_is_not_a_name_nothing_denotes() {
        let source = "\
#[builtin]
type Point

fun declared(value: Int): Int =
    value

fun main(): Int =
    let bound = declared(1) in
    bound::get() + declared::get() + Point::make()
";
        let parsed = mlkc_parser::parse(source);
        let root = parsed.tree::<ModuleRoot>();
        let lowered = crate::lower_module(
            ModuleId(FileId::from_raw(0)),
            &root,
            Prelude::standard(),
            &[],
            relative().as_path(),
        );

        let main = declaration(&root, &lowered, "main");
        let body = crate::lower_body(&lowered.item_tree, &main).expect("a body");

        // A binding the body made, a name of the module written where a path root is not read,
        // and a name of another namespace are names the body knows: what the names after one of
        // them denote is the language's business, and not a name that is not there.
        assert!(body.diagnostics.is_empty(), "{:?}", body.diagnostics);
    }

    #[test]
    fn a_type_a_body_writes_is_anchored_and_a_name_it_cannot_name_is_reported() {
        let source = "\
#[builtin]
type Point

#[extern]
fun declared(value: Point): Point

fun main(): Point =
    declared[Point](1) + declared[Missing](1)
";
        let parsed = mlkc_parser::parse(source);
        let root = parsed.tree::<ModuleRoot>();
        let lowered = crate::lower_module(
            ModuleId(FileId::from_raw(0)),
            &root,
            Prelude::standard(),
            &[],
            relative().as_path(),
        );

        let main = declaration(&root, &lowered, "main");
        let body = crate::lower_body(&lowered.item_tree, &main).expect("a body");

        // A type written in a body is a type like any other: a name of the module is what its
        // root is read as, and a name the module knows nothing of is reported where it is
        // written, as the name of an expression is.
        let dumped = mlkc_hir_def::dump::body(&lowered.bodies[0].owner, &body.body);

        assert!(dumped.contains("declared[Point -> type Point]"), "{dumped}");
        assert!(
            dumped.contains("declared[Missing -> unresolved]"),
            "{dumped}"
        );

        let messages: Vec<String> = body
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.error().message())
            .collect();

        assert_eq!(messages, [
            "the name `Missing` is not a name of this module, and names no project",
        ]);
    }
}
