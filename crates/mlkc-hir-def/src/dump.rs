//! A reading of the HIR by a person, and by a diff.
//!
//! A dump holds one line per node: the id of the node, and its shape. A change to one node
//! is therefore a change to one line, and a snapshot of a dump is read as a diff --- which
//! is what a spec test of a lowering needs, since the HIR it produces is a graph of arenas,
//! and `Debug` of a graph prints positions a reader cannot follow.
//!
//! Names are printed as the names they are, not as the symbols behind them: `fun main` is
//! what a reader sees at both ends of a snapshot, and a name that is missing is printed as
//! `<missing>`. A path is printed as it was written, followed by what its base resolved to,
//! so a dump shows resolution as well as shape.
//!
//! A dump is a reading of one revision and nothing more: the ids it prints are the positions
//! of the arenas of that revision, and a name is the only thing in it that denotes the same
//! entity in the next one.

use std::fmt::{self, Write as _};

use crate::{
    body::{Body, Expr, Literal, Pat},
    id::{BodyEntityLoc, EntityLoc, LocalConstId, LocalFunctionId, ModuleId, arena_index},
    item_data::{Attributes, EntityData, Signature, Visibility},
    item_tree::ItemTree,
    path::{PathAnchor, PathData, PathSegmentData},
    type_ref::TypeRef,
};

/// The indentation of one level of a dump.
const INDENT: &str = "  ";

/// What a name that is not there is printed as.
const MISSING: &str = "<missing>";

/// A reading of the item tree of a module.
///
/// The module is headed by the file it is read from and by the path it declares itself as,
/// and the entities follow in the order the module declares them, each of them headed by its
/// name and by the position it is at in the syntax, which is what a reader compares between
/// revisions.
pub fn item_tree(tree: &ItemTree) -> String {
    let module = tree.module();
    let mut dump = Dump::new();

    match tree.path() {
        Some(path) => dump.line(format!("MODULE #{} {path}", module_index(module))),
        None => dump.line(format!("MODULE #{}", module_index(module))),
    }

    dump.blank();

    dump.section("ITEM TREE", |dump| {
        for (loc, id) in tree.entities() {
            let entity = tree.entity(id);
            dump.line(format!("{loc:?}  @{}", entity.syntax()));

            dump.depth += 1;
            entity_data(dump, entity.data(), module);
            dump.depth -= 1;
        }
    });

    dump.render()
}

/// A reading of one body, headed by the name of the entity that owns it.
///
/// The owner is what a body is remembered by, since a body itself has no name,
/// and its module is what the paths of the body resolved against.
pub fn body(owner: &BodyEntityLoc, body: &Body) -> String {
    let module = owner.module();
    let mut dump = Dump::new();

    dump.line(format!(
        "BODY {:?} in module #{}",
        owner.item(),
        module_index(module),
    ));
    dump.blank();

    dump.line(format!("root {}", expr_ref(body.root())));

    dump.section("params", |dump| {
        for pat in body.params() {
            dump.line(pat_ref(*pat));
        }
    });

    dump.section("exprs", |dump| {
        for (id, expr) in body.exprs().iter() {
            dump.line(format!("{}  {}", expr_ref(id), expr_text(expr)));
        }
    });

    dump.section("pats", |dump| {
        for (id, pat) in body.pats().iter() {
            dump.line(format!("{}  {}", pat_ref(id), pat_text(pat)));
        }
    });

    dump.section("paths", |dump| {
        for (id, path) in body.paths().iter() {
            dump.line(format!("{}  {}", path_ref(id), path_resolved(path, module),));
        }
    });

    dump.section("functions", |dump| {
        for (id, data) in body.local_functions().iter() {
            let root = body.local_function_root(LocalFunctionId(id));
            dump.line(format!(
                "{}  {:?}  root {}",
                local_function_ref(id),
                data.name,
                root_text(root),
            ));

            dump.depth += 1;
            signature(dump, &data.signature, module);
            dump.depth -= 1;
        }
    });

    dump.section("consts", |dump| {
        for (id, data) in body.local_consts().iter() {
            let root = body.local_const_root(LocalConstId(id));
            dump.line(format!(
                "{}  {:?}  root {}",
                local_const_ref(id),
                data.name,
                root_text(root),
            ));

            dump.depth += 1;
            if let Some(ty) = &data.ty {
                dump.line(format!("ty: {}", type_ref(ty, module)));
            }
            dump.depth -= 1;
        }
    });

    dump.render()
}

/// The lines of the data of one entity.
fn entity_data(dump: &mut Dump, data: &EntityData, module: ModuleId) {
    attributes(dump, data.attributes());

    match data {
        EntityData::Function(data) => {
            visibility(dump, data.visibility);
            signature(dump, &data.signature, module);
        },
        EntityData::Class(data) => visibility(dump, data.visibility),
        EntityData::Value(data) => visibility(dump, data.visibility),
        EntityData::Const(data) => {
            visibility(dump, data.visibility);
            if let Some(ty) = &data.ty {
                dump.line(format!("ty: {}", type_ref(ty, module)));
            }
        },
        EntityData::Impl(data) => {
            if let Some(class) = &data.class {
                dump.line(format!("class: {}", type_ref(class, module)));
            }
            if let Some(ty) = &data.ty {
                dump.line(format!("ty: {}", type_ref(ty, module)));
            }
        },
        EntityData::Use(data) => {
            dump.line(format!("path: {}", *data.path));
            if let Some(alias) = &data.alias {
                dump.line(format!("alias: {alias:?}"));
            }
            visibility(dump, data.visibility);
        },
    }
}

/// The lines of a signature, parameters first.
///
/// A parameter is read as what its signature says of it, which is its type: what the
/// parameter binds is the pattern of the body, and the body of the function is where a dump
/// reads it.
fn signature(dump: &mut Dump, signature: &Signature, module: ModuleId) {
    for param in &signature.params {
        match &param.ty {
            Some(ty) => dump.line(format!("param: {}", type_ref(ty, module))),
            None => dump.line("param".to_owned()),
        }
    }

    if let Some(ret) = &signature.ret {
        dump.line(format!("ret: {}", type_ref(ret, module)));
    }
}

/// One line of the attributes of a declaration, if it carries any.
///
/// The line holds the attributes as the module writes them, since what a reader compares is
/// the source and the HIR it becomes.
fn attributes(dump: &mut Dump, attributes: Option<&Attributes>) {
    let Some(attributes) = attributes else {
        return;
    };

    if attributes.is_none() {
        return;
    }

    let mut words = Vec::new();

    if attributes.builtin {
        words.push("@builtin");
    }

    if attributes.external {
        words.push("@extern");
    }

    dump.line(format!("attributes: {}", words.join(" ")));
}

/// One line of the visibility of an entity.
fn visibility(dump: &mut Dump, visibility: Visibility) {
    let word = if visibility.is_public() {
        "public"
    } else {
        "private"
    };

    dump.line(format!("visibility: {word}"));
}

/// A type reference as it was written, and what its paths denote.
fn type_ref(ty: &TypeRef, module: ModuleId) -> String {
    match ty {
        TypeRef::Missing => MISSING.to_owned(),
        TypeRef::Infer => "_".to_owned(),
        TypeRef::Path(path) => path_resolved(path, module),
    }
}

/// A path as it was written, and what its root denotes.
fn path_resolved(path: &PathData, module: ModuleId) -> String {
    format!(
        "{} -> {}",
        path_text(path, module),
        anchor_text(&path.anchor, module),
    )
}

/// A path as it was written, with the arguments written at it.
fn path_text(path: &PathData, module: ModuleId) -> String {
    let mut text = String::new();

    write!(text, "{}", path.root).expect("writing to a string to never fail");
    arguments_text(&mut text, &path.root_args, module);

    for segment in &path.segments {
        text.push('.');
        segment_text(&mut text, segment, module);
    }

    text
}

/// One segment of a path, with the arguments written at it.
fn segment_text(text: &mut String, segment: &PathSegmentData, module: ModuleId) {
    write!(text, "{:?}", segment.name).expect("writing to a string to never fail");
    arguments_text(text, &segment.args, module);
}

/// The type arguments written at one name of a path.
fn arguments_text(text: &mut String, args: &[TypeRef], module: ModuleId) {
    if args.is_empty() {
        return;
    }

    text.push('[');

    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            text.push_str(", ");
        }

        text.push_str(&type_ref(arg, module));
    }

    text.push(']');
}

/// What the base of a path denotes.
///
/// A target in the module the dump is of is written as the entity itself;
/// one in another module carries the module it is in, since both are readable and one of them
/// is surprising.
fn anchor_text(anchor: &PathAnchor, module: ModuleId) -> String {
    match anchor {
        PathAnchor::Item(loc) => item_text(loc, module),
        PathAnchor::Use(loc) => format!("{loc:?}"),
        PathAnchor::Local(id) => format!("local {id:?}"),
        PathAnchor::TypeVar(var) => {
            format!("typevar {}[{}]", item_text(&var.owner, module), var.index)
        },
        PathAnchor::Binding(pat) => format!("binding {}", pat_ref(*pat)),
        // A meaning rather than a name: what the keyword names is this very project.
        PathAnchor::Project => "the project".to_owned(),
        PathAnchor::Unresolved => "unresolved".to_owned(),
    }
}

/// An entity of the project, named by its module if the module is not the one being dumped.
fn item_text(loc: &EntityLoc, module: ModuleId) -> String {
    if loc.module == module {
        format!("{:?}", loc.item)
    } else {
        format!("{:?} in module #{}", loc.item, module_index(loc.module))
    }
}

/// The shape of one expression, as the ids it is made of.
fn expr_text(expr: &Expr) -> String {
    match expr {
        Expr::Missing => "missing".to_owned(),
        Expr::Path(id) => format!("path {}", path_ref(*id)),
        Expr::Literal(Literal::Int(value)) => format!("literal {value}"),
        Expr::Literal(Literal::Str(value)) => {
            let value: &str = value;
            format!("literal {value:?}")
        },
        Expr::Call { callee, args } => {
            let args: Vec<String> = args.iter().map(|id| expr_ref(*id)).collect();
            format!("call {} ({})", expr_ref(*callee), args.join(", "))
        },
        Expr::Binary { lhs, op, rhs } => {
            format!("binary {} {op} {}", expr_ref(*lhs), expr_ref(*rhs))
        },
        Expr::Unary { op, operand } => format!("unary {op} {}", expr_ref(*operand)),
        Expr::Seq { first, then } => format!("seq {} {}", expr_ref(*first), expr_ref(*then)),
        Expr::Let { pat, expr, body } => {
            format!(
                "let {} = {} in {}",
                pat_ref(*pat),
                expr_ref(*expr),
                expr_ref(*body),
            )
        },
    }
}

/// The shape of one pattern.
fn pat_text(pat: &Pat) -> String {
    match pat {
        Pat::Missing => "missing".to_owned(),
        Pat::Wildcard => "_".to_owned(),
        Pat::Bind(name) => format!("bind {name:?}"),
    }
}

/// The name of an expression or a pattern in the arena of a body.
fn expr_ref(id: crate::body::ExprId) -> String {
    format!("expr#{}", arena_index(id))
}

/// The name of a pattern in the arena of a body.
fn pat_ref(id: crate::body::PatId) -> String {
    format!("pat#{}", arena_index(id))
}

/// The name of a path in the arena of a body.
fn path_ref(id: crate::path::PathId) -> String {
    format!("path#{}", arena_index(id))
}

/// The name of a function declared inside a body.
fn local_function_ref(id: mlkc_la_arena::Idx<crate::body::LocalFunctionData>) -> String {
    format!("fun#{}", arena_index(id))
}

/// The name of a constant declared inside a body.
fn local_const_ref(id: mlkc_la_arena::Idx<crate::body::LocalConstData>) -> String {
    format!("const#{}", arena_index(id))
}

/// The root of an entity declared inside a body, which every one of them has.
fn root_text(root: Option<crate::body::ExprId>) -> String {
    match root {
        Some(id) => expr_ref(id),
        None => "<none>".to_owned(),
    }
}

/// The number a module is known by in a dump.
fn module_index(module: ModuleId) -> u32 {
    module.0.index()
}

/// The text of a dump, written one line at a time.
#[derive(Default)]
struct Dump {
    lines: Vec<String>,
    depth: usize,
}

impl Dump {
    fn new() -> Self {
        Self::default()
    }

    /// A line at the current indentation.
    fn line(&mut self, text: impl fmt::Display) {
        let text = text.to_string();
        let line = self.indented(&text);

        self.lines.push(line);
    }

    /// An empty line, which separates the parts of a dump.
    fn blank(&mut self) {
        self.lines.push(String::new());
    }

    /// A named part, holding the lines the closure writes one level deeper.
    ///
    /// A part that holds nothing is left out, so that a dump shows what is there
    /// and a reader is not made to skip over what is not.
    fn section(&mut self, header: impl fmt::Display, lines: impl FnOnce(&mut Self)) {
        let start = self.lines.len();

        self.depth += 1;
        lines(self);
        self.depth -= 1;

        if self.lines.len() == start {
            return;
        }

        let header = self.indented(&header.to_string());
        self.lines.insert(start, header);
    }

    /// The text of one line, indented to the current depth.
    fn indented(&self, text: &str) -> String {
        let mut line = String::new();

        for _ in 0..self.depth {
            line.push_str(INDENT);
        }

        write!(line, "{text}").expect("writing to a string to never fail");

        line
    }

    /// The dump, as the text of a file.
    fn render(self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');

        text
    }
}

#[cfg(test)]
mod tests {
    use mlkc_intern::Interned;
    use mlkc_vfs::FileId;

    use super::*;
    use crate::{
        Name,
        body::{BinaryOp, BodyBuilder, LocalConstData, LocalFunctionData, Pat},
        id::{BodyLoc, FunctionLoc, ItemKind, ItemLoc},
        item_data::{Attributes, ClassData, FunctionData, ImplData, ParamData},
        item_tree::{ItemSyntaxLoc, ItemTreeBuilder},
        path::{PathRoot, PlainPath, PlainPathId},
    };

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn entity(name: &str, kind: ItemKind) -> EntityLoc {
        EntityLoc {
            module: module(),
            item: ItemLoc::new(kind, Some(Name::new(name)), 0),
        }
    }

    fn owner(name: &str) -> BodyEntityLoc {
        BodyEntityLoc {
            module: module(),
            item: BodyLoc::Function(
                FunctionLoc::try_from(ItemLoc::new(ItemKind::Function, Some(Name::new(name)), 0))
                    .expect("a function"),
            ),
        }
    }

    /// A type written as a path of one segment.
    fn type_path(name: &str, anchor: PathAnchor) -> TypeRef {
        TypeRef::Path(PathData::ident(Name::new(name), anchor))
    }

    #[test]
    fn an_item_tree_reads_as_the_module_declares_it() {
        let mut builder = ItemTreeBuilder::new(module());

        builder.declare(
            Some(Name::new("Int")),
            EntityData::Class(ClassData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(0),
        );
        builder.declare(
            Some(Name::new("main")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Private,
                signature: Signature {
                    params: vec![ParamData {
                        ty: Some(type_path("Int", PathAnchor::Unresolved)),
                    }],
                    ret: Some(type_path("Unit", PathAnchor::Unresolved)),
                },
            }),
            ItemSyntaxLoc::root().child(1),
        );

        let tree = builder.finish();

        // The path of `value` resolved against the names of the module, and the one of the
        // return type did not: the dump is where that difference is read.
        assert_eq!(
            crate::dump::item_tree(&tree),
            "\
MODULE #0

ITEM TREE
  type Int  @0
    visibility: public
  fun main  @1
    visibility: private
    param: Int -> type Int
    ret: Unit -> unresolved
"
        );
    }

    #[test]
    fn a_module_reads_as_the_path_it_declares_and_what_its_entities_carry() {
        let mut builder = ItemTreeBuilder::new(module());
        builder.set_path(PlainPathId::new(PlainPath::from_root(PathRoot::Project, [
            Name::new("main-module"),
        ])));

        builder.declare(
            Some(Name::new("Unit")),
            EntityData::Class(ClassData {
                attributes: Attributes {
                    builtin: true,
                    external: false,
                },
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(0),
        );
        builder.declare(
            Some(Name::new("println-int")),
            EntityData::Function(FunctionData {
                attributes: Attributes {
                    builtin: false,
                    external: true,
                },
                visibility: Visibility::Private,
                signature: Signature {
                    params: vec![ParamData {
                        ty: Some(type_path("Int", PathAnchor::Unresolved)),
                    }],
                    ret: Some(type_path("Unit", PathAnchor::Unresolved)),
                },
            }),
            ItemSyntaxLoc::root().child(1),
        );

        let tree = builder.finish();

        assert_eq!(
            crate::dump::item_tree(&tree),
            "\
MODULE #0 project.main-module

ITEM TREE
  type Unit  @0
    attributes: @builtin
    visibility: public
  fun println-int  @1
    attributes: @extern
    visibility: private
    param: Int -> unresolved
    ret: Unit -> type Unit
"
        );
    }

    #[test]
    fn a_name_that_is_not_there_is_read_as_a_word() {
        let mut builder = ItemTreeBuilder::new(module());

        // An entity with no name of its own, and two whose names the syntax does not have:
        // the second of the two is not the same entity as the first, and a dump says so.
        builder.declare(
            None,
            EntityData::Impl(ImplData {
                class: None,
                ty: None,
            }),
            ItemSyntaxLoc::root().child(0),
        );

        for slot in 1..3 {
            builder.declare(
                Some(Name::missing()),
                EntityData::Function(FunctionData {
                    attributes: Attributes::default(),
                    visibility: Visibility::Private,
                    signature: Signature::default(),
                }),
                ItemSyntaxLoc::root().child(slot),
            );
        }

        let tree = builder.finish();

        assert_eq!(
            crate::dump::item_tree(&tree),
            "\
MODULE #0

ITEM TREE
  impl <anon>#0  @0
  fun <missing>  @1
    visibility: private
  fun <missing>#1  @2
    visibility: private
"
        );
    }

    #[test]
    fn a_body_reads_as_the_ids_its_nodes_are_made_of() {
        let mut builder = BodyBuilder::new();

        // `let _ = g(42, unknown) in g(42, unknown)`.
        let callee = builder.alloc_path(PathData::ident(
            Name::new("g"),
            PathAnchor::Item(entity("g", ItemKind::Function)),
        ));
        let callee = builder.alloc_expr(Expr::Path(callee));
        let value = builder.alloc_expr(Expr::Literal(Literal::Int(42)));
        let unknown = builder.alloc_path(PathData::ident(
            Name::new("unknown"),
            PathAnchor::Unresolved,
        ));
        let unknown = builder.alloc_expr(Expr::Path(unknown));
        let call = builder.alloc_expr(Expr::Call {
            callee,
            args: vec![value, unknown],
        });

        let param = builder.alloc_pat(Pat::Bind(Name::new("x")));
        builder.push_param(param);

        let ignored = builder.alloc_pat(Pat::Wildcard);
        let root = builder.alloc_expr(Expr::Let {
            pat: ignored,
            expr: call,
            body: call,
        });
        builder.set_root(root);

        let helper = builder.declare_local_function(LocalFunctionData {
            name: Name::new("helper"),
            signature: Signature {
                params: vec![ParamData {
                    ty: Some(type_path(
                        "Int",
                        PathAnchor::Item(entity("Int", ItemKind::Class)),
                    )),
                }],
                ret: Some(TypeRef::Infer),
            },
        });
        let literal = builder.alloc_expr(Expr::Literal(Literal::Int(7)));
        builder.set_local_function_root(helper, literal);

        let body = builder.finish();

        assert_eq!(
            crate::dump::body(&owner("main"), &body),
            "\
BODY fun main in module #0

root expr#4
params
  pat#0
exprs
  expr#0  path path#0
  expr#1  literal 42
  expr#2  path path#1
  expr#3  call expr#0 (expr#1, expr#2)
  expr#4  let pat#1 = expr#3 in expr#3
  expr#5  literal 7
pats
  pat#0  bind x
  pat#1  _
paths
  path#0  g -> fun g
  path#1  unknown -> unresolved
functions
  fun#0  helper  root expr#5
    param: Int -> type Int
    ret: _
"
        );
    }

    #[test]
    fn the_nodes_of_a_body_are_read_one_line_each() {
        let mut builder = BodyBuilder::new();

        let missing = builder.alloc_expr(Expr::Missing);
        let text = builder.alloc_expr(Expr::Literal(Literal::Str(Interned::new_str("say \"hi\""))));
        let sum = builder.alloc_expr(Expr::Binary {
            lhs: missing,
            op: BinaryOp::Add,
            rhs: text,
        });
        let seq = builder.alloc_expr(Expr::Seq {
            first: sum,
            then: text,
        });
        builder.set_root(seq);

        let body = builder.finish();

        assert_eq!(
            crate::dump::body(&owner("f"), &body),
            "\
BODY fun f in module #0

root expr#3
exprs
  expr#0  missing
  expr#1  literal \"say \\\"hi\\\"\"
  expr#2  binary expr#0 + expr#1
  expr#3  seq expr#2 expr#1
"
        );
    }

    #[test]
    fn the_consts_of_a_body_are_read_with_their_roots() {
        let mut builder = BodyBuilder::new();

        let constant = builder.declare_local_const(LocalConstData {
            name: Name::new("pi"),
            ty: Some(type_path("Float", PathAnchor::Unresolved)),
        });
        let literal = builder.alloc_expr(Expr::Literal(Literal::Int(3)));
        builder.set_local_const_root(constant, literal);
        builder.set_root(literal);

        let body = builder.finish();

        assert_eq!(
            crate::dump::body(&owner("pi"), &body),
            "\
BODY fun pi in module #0

root expr#0
exprs
  expr#0  literal 3
consts
  const#0  pi  root expr#0
    ty: Float -> unresolved
"
        );
    }
}
