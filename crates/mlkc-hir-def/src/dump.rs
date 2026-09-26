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
//!
//! The words are read twice: as text, which is what a snapshot of a dump holds, and as a tree
//! of lines ([`Node`]), which is what a host that folds what it is not reading and marks
//! a buffer by a line reads. A line of either reading says the same thing, and a line of
//! a tree that stands for a node of the HIR carries it ([`Target`]) so that a host can find
//! the syntax the node was read from.

use std::fmt::{self, Write as _};

use crate::{
    body::{Body, Expr, ExprId, Literal, Pat, PatId},
    id::{BodyEntityLoc, EntityLoc, ItemLoc, LocalConstId, LocalFunctionId, ModuleId, arena_index},
    item_data::{Attributes, EntityData, Signature, Visibility},
    item_tree::{Entity, ItemTree},
    path::{PathAnchor, PathData, PathId, PathSegmentData},
    type_ref::TypeRef,
};

/// The indentation of one level of a dump.
const INDENT: &str = "  ";

/// What a name that is not there is printed as.
const MISSING: &str = "<missing>";

/// What a line of a reading is about, since a line is what a host marks a buffer with.
///
/// A line that stands for no node of the HIR --- a section header, a field of a declaration ---
/// is about nothing, and a host marks nothing for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// An entity of the module: a function, a class, an import.
    Item(ItemLoc),
    /// A type a declaration writes: the type of a parameter, or the type of a result.
    ///
    /// A type is a value of the HIR rather than a node of it, and where it is written is where
    /// the declaration writes it: the entity it is written in, and which of its types it is.
    Type {
        /// The entity the type is written in.
        item: ItemLoc,
        /// Which type of that declaration it is.
        place: TypePlace,
    },
    /// An expression of a body.
    Expr(ExprId),
    /// A pattern of a body.
    Pat(PatId),
    /// A path of a body.
    Path(PathId),
}

/// A type a declaration writes, named by where the declaration writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypePlace {
    /// The type of a parameter, at the index the declaration writes it at.
    ///
    /// A signature holds a parameter for every parameter the declaration wrote, the ones that
    /// broke included, so the index of a signature is the index of the declaration.
    Parameter(usize),
    /// The type the declaration writes for its result.
    Result,
}

/// What a line of a reading is, which is what a host paints it by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// The module itself, headed by the path it declares.
    Module,
    /// A body, headed by the entity that owns it.
    Body,
    /// A named part of a reading: `ITEM TREE`, `root`.
    Section,
    /// An entity of the module, or one declared inside a body.
    Item,
    /// One of the things an entity is: its visibility, its signature, the path of an import.
    Field,
    /// An expression of a body.
    Expr,
    /// A pattern of a body.
    Pat,
    /// A path of a body.
    Path,
}

/// One part of the text of a line.
///
/// A line says one thing unless a part of it says something a host paints differently from the
/// rest: the type a signature writes is a path, and a path reads the way a path reads wherever
/// it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The words of this part.
    pub text: String,
    /// What the words are, when they are not what the line is.
    pub kind: Option<NodeKind>,
}

impl Part {
    /// A part that says what its line says.
    fn plain(text: impl fmt::Display) -> Self {
        Self {
            text: text.to_string(),
            kind: None,
        }
    }

    /// A part that says something of its own kind.
    fn of(text: impl fmt::Display, kind: NodeKind) -> Self {
        Self {
            text: text.to_string(),
            kind: Some(kind),
        }
    }
}

/// A line of a reading, and the lines under it.
///
/// The text of a reading is these lines in order, each of them indented by the level it is at.
/// A host that shows a reading as a tree folds what it is not reading, and marks the source
/// a line stands for: [`Node::target`] is a node of the HIR, and the syntax it was read from
/// is what a file is marked by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// What the line says, as the parts a host paints.
    pub parts: Vec<Part>,
    /// What the line is, which is what a host paints it by.
    pub kind: NodeKind,
    /// The node of the HIR the line is about, if it is about one.
    pub target: Option<Target>,
    /// The node the line's own node resolves to, when it is a name that resolved to something:
    /// the entity or the import a path names, the binding a name of a body stands for.
    ///
    /// A line says what a node is, and a node that names another is what a reader follows when
    /// they ask where a name comes from: a host marks both ends of that walk, and the syntax
    /// of the target is a place in the same file it reads the line of.
    pub resolves: Option<Target>,
    /// The lines under it, in the order they are read.
    pub children: Vec<Node>,
}

impl Node {
    /// A line that says one thing.
    fn line(text: impl fmt::Display, kind: NodeKind) -> Self {
        Self {
            parts: vec![Part::plain(text)],
            kind,
            target: None,
            resolves: None,
            children: Vec::new(),
        }
    }

    /// A line that says one thing, and stands for a node of the HIR.
    fn marked(text: impl fmt::Display, kind: NodeKind, target: Target) -> Self {
        let mut line = Self::line(text, kind);
        line.target = Some(target);

        line
    }

    /// The text of the line, which is what a reading writes out.
    pub fn text(&self) -> String {
        self.parts.iter().map(|part| part.text.as_str()).collect()
    }
}

/// A reading of the item tree of a module.
///
/// The module is headed by the file it is read from and by the path it declares itself as,
/// and the entities follow in the order the module declares them, each of them headed by its
/// name and by the position it is at in the syntax, which is what a reader compares between
/// revisions.
pub fn item_tree(tree: &ItemTree) -> String {
    let module = tree.module();
    let mut dump = Dump::new();

    dump.line(module_line(tree, module));
    dump.blank();

    dump.section("ITEM TREE", |dump| {
        for (loc, id) in tree.entities() {
            let entity = tree.entity(id);
            dump.line(entity_line(&loc, entity));

            dump.depth += 1;
            dump.lines(&entity_data_lines(entity.data(), module, &loc));
            dump.depth -= 1;
        }
    });

    dump.render()
}

/// The line a reading of a module is headed by: the module, and the path it declares itself as.
fn module_line(tree: &ItemTree, module: ModuleId) -> String {
    match tree.path() {
        Some(path) => format!("MODULE #{} {path}", module_index(module)),
        None => format!("MODULE #{}", module_index(module)),
    }
}

/// The line one entity of a module is headed by: what it is, and where it is written.
///
/// The position is what a reader compares between two revisions, since the entities of a
/// module are held in the order it declares them.
fn entity_line(loc: &ItemLoc, entity: &Entity) -> String {
    format!("{loc:?}  @{}", entity.syntax())
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
            dump.lines(&signature_lines(&data.signature, module, None));
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

/// The lines of the item tree of a module, as a tree a host reads.
///
/// This is the reading [`item_tree`] prints, with what an entity is made of under the entity
/// rather than after it, and without the blank line a text holds: a host folds a line and the
/// lines under it, and a line that says nothing is not one of them.
pub fn item_tree_nodes(tree: &ItemTree) -> Vec<Node> {
    let module = tree.module();
    let mut nodes = vec![module_node(tree, module)];

    let items: Vec<Node> = tree
        .entities()
        .map(|(loc, id)| {
            let entity = tree.entity(id);
            let mut item = Node::marked(
                entity_line(&loc, entity),
                NodeKind::Item,
                Target::Item(loc.clone()),
            );
            item.children = entity_data_lines(entity.data(), module, &loc);

            item
        })
        .collect();

    if !items.is_empty() {
        let mut section = Node::line("ITEM TREE", NodeKind::Section);
        section.children = items;

        nodes.push(section);
    }

    nodes
}

/// The line a reading of a module is headed by.
fn module_node(tree: &ItemTree, module: ModuleId) -> Node {
    Node::line(module_line(tree, module), NodeKind::Module)
}

/// The lines of one body, as a tree a host reads.
///
/// The reading [`body`] prints is a line per node of the body, in the order the arenas hold
/// them, which is what a diff of two revisions is read for. A host that folds what it is not
/// reading wants the other shape, and a body is a graph of ids: an expression under the
/// expression that is made of it, and a pattern or a path under the expression it is written
/// at, so that what a person folds is a part of the body rather than a list of its nodes.
pub fn body_nodes(owner: &BodyEntityLoc, body: &Body) -> Node {
    let module = owner.module();
    let mut children = Vec::new();

    if !body.params().is_empty() {
        let mut params = Node::line("params", NodeKind::Section);
        params.children = body.params().iter().map(|id| pat_node(body, *id)).collect();

        children.push(params);
    }

    let mut root = Node::line("root", NodeKind::Section);
    root.children = vec![expr_node(body, module, body.root())];

    children.push(root);

    for (id, data) in body.local_functions().iter() {
        let mut inner = signature_lines(&data.signature, module, None);
        inner.push(root_node(
            body,
            module,
            body.local_function_root(LocalFunctionId(id)),
        ));

        let mut node = Node::line(
            format!("{}  {:?}", local_function_ref(id), data.name),
            NodeKind::Item,
        );
        node.children = inner;

        children.push(node);
    }

    for (id, data) in body.local_consts().iter() {
        let mut inner = Vec::new();

        if let Some(ty) = &data.ty {
            inner.push(type_line("ty", Some(ty), None, module));
        }

        inner.push(root_node(
            body,
            module,
            body.local_const_root(LocalConstId(id)),
        ));

        let mut node = Node::line(
            format!("{}  {:?}", local_const_ref(id), data.name),
            NodeKind::Item,
        );
        node.children = inner;

        children.push(node);
    }

    let mut node = Node::marked(
        format!(
            "BODY {:?} in module #{}",
            owner.item(),
            module_index(module)
        ),
        NodeKind::Body,
        Target::Item(owner.item().clone().into()),
    );
    node.children = children;

    node
}

/// The root of an entity declared inside a body, and the expressions it is made of.
fn root_node(body: &Body, module: ModuleId, root: Option<ExprId>) -> Node {
    let mut node = Node::line(format!("root {}", root_text(root)), NodeKind::Section);
    node.target = root.map(Target::Expr);
    node.children = root
        .map(|id| vec![expr_node(body, module, id)])
        .unwrap_or_default();

    node
}

/// One expression of a body, with what it is made of under it.
fn expr_node(body: &Body, module: ModuleId, id: ExprId) -> Node {
    let mut children = Vec::new();

    match &body[id] {
        Expr::Missing | Expr::Literal(_) => {},
        Expr::Path(path) => children.push(path_node(body, module, *path)),
        Expr::Call { callee, args } => {
            children.push(expr_node(body, module, *callee));
            children.extend(args.iter().map(|arg| expr_node(body, module, *arg)));
        },
        Expr::Binary { lhs, rhs, .. } => {
            children.push(expr_node(body, module, *lhs));
            children.push(expr_node(body, module, *rhs));
        },
        Expr::Unary { operand, .. } => children.push(expr_node(body, module, *operand)),
        Expr::Seq { first, then } => {
            children.push(expr_node(body, module, *first));
            children.push(expr_node(body, module, *then));
        },
        Expr::Let {
            pat,
            expr,
            body: inner,
        } => {
            children.push(pat_node(body, *pat));
            children.push(expr_node(body, module, *expr));
            children.push(expr_node(body, module, *inner));
        },
    }

    let mut node = Node::marked(
        format!("{}  {}", expr_ref(id), expr_text(&body[id])),
        NodeKind::Expr,
        Target::Expr(id),
    );
    node.children = children;

    node
}

/// One pattern of a body.
fn pat_node(body: &Body, id: PatId) -> Node {
    Node::marked(
        format!("{}  {}", pat_ref(id), pat_text(&body[id])),
        NodeKind::Pat,
        Target::Pat(id),
    )
}

/// One path of a body: the path, what its root denotes, and what the path names.
fn path_node(body: &Body, module: ModuleId, id: PathId) -> Node {
    let mut node = Node::marked(
        format!("{}  {}", path_ref(id), path_resolved(&body[id], module)),
        NodeKind::Path,
        Target::Path(id),
    );
    node.resolves = path_target(body, id);

    node
}

/// What a path names, when it names something the HIR points at.
fn path_target(body: &Body, id: PathId) -> Option<Target> {
    anchor_target(&body[id].anchor)
}

/// What a type names, when it is a path that resolved to something.
fn type_target(ty: &TypeRef) -> Option<Target> {
    match ty {
        TypeRef::Path(path) => anchor_target(&path.anchor),
        TypeRef::Missing | TypeRef::Infer => None,
    }
}

/// What a path that resolved to something points at.
///
/// A path resolved to an entity of the module, to an entry of its import table, or to
/// a binding of the body: what a reader is told about besides the path itself is where the
/// name comes from. A path rooted at the project, one that resolved to an entity of another
/// module, and one that resolved to nothing point at no place in this file.
fn anchor_target(anchor: &PathAnchor) -> Option<Target> {
    match anchor {
        PathAnchor::Item(entity) => Some(Target::Item(entity.item.clone())),
        PathAnchor::Use(import) => Some(Target::Item(ItemLoc::Use(import.clone()))),
        PathAnchor::Binding(pat) => Some(Target::Pat(*pat)),
        PathAnchor::Local(_)
        | PathAnchor::TypeVar(_)
        | PathAnchor::Project
        | PathAnchor::Unresolved => None,
    }
}

/// One line of what an entity is.
fn field_node(text: impl fmt::Display) -> Node {
    Node::line(text, NodeKind::Field)
}

/// The lines of the data of one entity.
fn entity_data_lines(data: &EntityData, module: ModuleId, item: &ItemLoc) -> Vec<Node> {
    let mut lines = Vec::new();
    lines.extend(attributes_line(data.attributes()));

    match data {
        EntityData::Function(data) => {
            lines.push(visibility_line(data.visibility));
            lines.extend(signature_lines(&data.signature, module, Some(item)));
        },
        EntityData::Class(data) => lines.push(visibility_line(data.visibility)),
        EntityData::Value(data) => lines.push(visibility_line(data.visibility)),
        EntityData::Const(data) => {
            lines.push(visibility_line(data.visibility));
            if let Some(ty) = &data.ty {
                lines.push(type_line("ty", Some(ty), None, module));
            }
        },
        EntityData::Impl(data) => {
            if let Some(class) = &data.class {
                lines.push(type_line("class", Some(class), None, module));
            }
            if let Some(ty) = &data.ty {
                lines.push(type_line("ty", Some(ty), None, module));
            }
        },
        EntityData::Use(data) => {
            lines.push(field_node(format!("path: {}", *data.path)));
            if let Some(alias) = &data.alias {
                lines.push(field_node(format!("alias: {alias:?}")));
            }
            lines.push(visibility_line(data.visibility));
        },
    }

    lines
}

/// The lines of a signature, parameters first.
///
/// A parameter is read as what its signature says of it, which is its type: what the
/// parameter binds is the pattern of the body, and the body of the function is where a dump
/// reads it.
///
/// A line of a signature is about a type the declaration wrote, and a type is a place of that
/// declaration: `item` is the entity the signature is of, and a signature a reader holds
/// without one --- the one of an entity declared inside a body --- points at nothing.
fn signature_lines(signature: &Signature, module: ModuleId, item: Option<&ItemLoc>) -> Vec<Node> {
    let mut lines = Vec::new();

    for (index, param) in signature.params.iter().enumerate() {
        let written = item.map(|item| (item, TypePlace::Parameter(index)));

        lines.push(type_line("param", param.ty.as_ref(), written, module));
    }

    if let Some(ret) = &signature.ret {
        let written = item.map(|item| (item, TypePlace::Result));

        lines.push(type_line("ret", Some(ret), written, module));
    }

    lines
}

/// One line that reads a type: the type as it was written, and what its paths name.
///
/// A type written as a path is read the way a path is read wherever it is written, and a type
/// that names nothing --- the `_` of an inferred type, a type that is not there --- is read as
/// what it is.
///
/// `written` says which type of a declaration the line reads, when the line is about one a
/// declaration writes: a signature a reader holds without a declaration --- the one of an
/// entity declared inside a body --- and a type the language has no declaration for yet have
/// no place to point at.
fn type_line(
    label: &str,
    ty: Option<&TypeRef>,
    written: Option<(&ItemLoc, TypePlace)>,
    module: ModuleId,
) -> Node {
    let mut parts = vec![Part::plain(label)];

    if let Some(ty) = ty {
        parts.push(Part::plain(": "));
        parts.push(type_part(ty, module));
    }

    Node {
        parts,
        kind: NodeKind::Field,
        target: match (written, ty) {
            (Some((item, place)), Some(_)) => {
                Some(Target::Type {
                    item: item.clone(),
                    place,
                })
            },
            _ => None,
        },
        resolves: ty.and_then(type_target),
        children: Vec::new(),
    }
}

/// A type as it is read: as a path, when it is one, and as what it is otherwise.
fn type_part(ty: &TypeRef, module: ModuleId) -> Part {
    match ty {
        TypeRef::Path(path) => Part::of(path_resolved(path, module), NodeKind::Path),
        TypeRef::Missing | TypeRef::Infer => Part::plain(type_ref(ty, module)),
    }
}

/// One line of the attributes of a declaration, if it carries any.
///
/// The line holds the attributes as the module writes them, since what a reader compares is
/// the source and the HIR it becomes.
fn attributes_line(attributes: Option<&Attributes>) -> Option<Node> {
    let attributes = attributes?;

    if attributes.is_none() {
        return None;
    }

    let mut words = Vec::new();

    if attributes.builtin {
        words.push("@builtin");
    }

    if attributes.external {
        words.push("@extern");
    }

    Some(field_node(format!("attributes: {}", words.join(" "))))
}

/// One line of the visibility of an entity.
fn visibility_line(visibility: Visibility) -> Node {
    let word = if visibility.is_public() {
        "public"
    } else {
        "private"
    };

    field_node(format!("visibility: {word}"))
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

    /// Lines of a reading that was made apart from this one.
    fn lines(&mut self, lines: &[Node]) {
        for line in lines {
            self.line(line.text());
        }
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
        id::{BodyLoc, FunctionLoc, ItemKind, ItemLoc, UseLoc},
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

    #[test]
    fn the_tree_of_an_item_tree_holds_what_an_entity_is() {
        let mut builder = ItemTreeBuilder::new(module());
        builder.set_path(PlainPathId::new(PlainPath::from_root(PathRoot::Project, [
            Name::new("main-module"),
        ])));
        builder.declare(
            Some(Name::new("Unit")),
            EntityData::Class(ClassData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(0),
        );

        let tree = builder.finish();
        let nodes = crate::dump::item_tree_nodes(&tree);

        assert_eq!(nodes[0].text(), "MODULE #0 project.main-module");
        assert_eq!(nodes[0].kind, NodeKind::Module);
        assert_eq!(nodes[0].target, None);

        let items = &nodes[1];

        assert_eq!(items.text(), "ITEM TREE");
        assert_eq!(items.kind, NodeKind::Section);

        let unit = &items.children[0];

        assert_eq!(unit.text(), "type Unit  @0");
        assert_eq!(unit.kind, NodeKind::Item);
        assert_eq!(
            unit.target,
            Some(Target::Item(ItemLoc::new(
                ItemKind::Class,
                Some(Name::new("Unit")),
                0,
            ))),
            "a line about an entity stands for the entity"
        );
        assert_eq!(unit.children.len(), 1);
        assert_eq!(unit.children[0].text(), "visibility: public");
        assert_eq!(unit.children[0].kind, NodeKind::Field);
        assert_eq!(unit.children[0].target, None, "a field is about nothing");
    }

    #[test]
    fn the_tree_of_a_body_holds_what_an_expression_is_made_of() {
        let mut builder = BodyBuilder::new();

        let one = builder.alloc_expr(Expr::Literal(Literal::Int(1)));
        let two = builder.alloc_expr(Expr::Literal(Literal::Int(2)));
        let sum = builder.alloc_expr(Expr::Binary {
            lhs: one,
            op: BinaryOp::Add,
            rhs: two,
        });
        builder.set_root(sum);

        let owner = owner("f");
        let node = crate::dump::body_nodes(&owner, &builder.finish());

        assert_eq!(node.text(), "BODY fun f in module #0");
        assert_eq!(node.kind, NodeKind::Body);
        assert_eq!(
            node.target,
            Some(Target::Item(owner.item().clone().into())),
            "a body stands for the declaration it is written in"
        );
        assert_eq!(
            tree_text(&node),
            "\
BODY fun f in module #0
  root
    expr#2  binary expr#0 + expr#1
      expr#0  literal 1
      expr#1  literal 2
"
        );
    }

    #[test]
    fn a_line_of_a_signature_is_about_a_type_of_the_declaration() {
        let function =
            FunctionLoc::try_from(ItemLoc::new(ItemKind::Function, Some(Name::new("f")), 0))
                .expect("a function");
        let imported = UseLoc::try_from(ItemLoc::new(ItemKind::Use, Some(Name::new("Int")), 0))
            .expect("a use");

        let data = EntityData::Function(FunctionData {
            attributes: Attributes::default(),
            visibility: Visibility::Private,
            signature: Signature {
                params: vec![ParamData {
                    ty: Some(type_path("Int", PathAnchor::Use(imported.clone()))),
                }],
                ret: Some(TypeRef::Infer),
            },
        });

        let lines = entity_data_lines(&data, module(), &ItemLoc::Function(function.clone()));

        let param = &lines[1];

        assert_eq!(param.text(), "param: Int -> use Int");
        assert_eq!(
            param.parts,
            vec![
                Part::plain("param"),
                Part::plain(": "),
                Part::of("Int -> use Int", NodeKind::Path),
            ],
            "the type of a parameter reads as a path, and the label of the line does not"
        );
        assert_eq!(
            param.target,
            Some(Target::Type {
                item: ItemLoc::Function(function),
                place: TypePlace::Parameter(0),
            }),
            "a line of a signature is about the type the declaration wrote"
        );
        assert_eq!(
            param.resolves,
            Some(Target::Item(ItemLoc::Use(imported))),
            "and the type names what the import brought in"
        );

        let ret = &lines[2];

        assert_eq!(ret.text(), "ret: _");
        assert_eq!(
            ret.resolves, None,
            "a type that names nothing resolves to nothing"
        );
    }

    #[test]
    fn a_path_of_a_body_says_what_it_resolved_to() {
        let mut builder = BodyBuilder::new();

        let entity = EntityLoc {
            module: module(),
            item: ItemLoc::new(ItemKind::Function, Some(Name::new("f")), 0),
        };
        let pat = builder.alloc_pat(Pat::Bind(Name::new("x")));
        let bound = builder.alloc_path(PathData::ident(Name::new("x"), PathAnchor::Binding(pat)));
        let named = builder.alloc_path(PathData::ident(
            Name::new("f"),
            PathAnchor::Item(entity.clone()),
        ));

        let callee = builder.alloc_expr(Expr::Path(named));
        let argument = builder.alloc_expr(Expr::Path(bound));
        let call = builder.alloc_expr(Expr::Call {
            callee,
            args: vec![argument],
        });
        builder.set_root(call);

        let node = crate::dump::body_nodes(&owner("f"), &builder.finish());
        let call = &node.children[0].children[0];
        // An expression reads one level above the path it is written as: the line of the
        // expression says which expression it is, and the line of the path below it says
        // what the path names.
        let callee = &call.children[0].children[0];
        let argument = &call.children[1].children[0];

        assert_eq!(callee.target, Some(Target::Path(named)));
        assert_eq!(
            callee.resolves,
            Some(Target::Item(entity.item)),
            "a path that names an entity of the module points at the entity"
        );
        assert_eq!(argument.target, Some(Target::Path(bound)));
        assert_eq!(
            argument.resolves,
            Some(Target::Pat(pat)),
            "a path that names a binding points at the binding"
        );
    }

    /// The lines of a tree of a reading, indented by what they are under.
    fn tree_text(node: &Node) -> String {
        let mut text = String::new();

        write_tree(&mut text, node, 0);

        text
    }

    fn write_tree(text: &mut String, node: &Node, depth: usize) {
        for _ in 0..depth {
            text.push_str("  ");
        }

        writeln!(text, "{}", node.text()).expect("writing to a string to never fail");

        for child in &node.children {
            write_tree(text, child, depth + 1);
        }
    }
}
