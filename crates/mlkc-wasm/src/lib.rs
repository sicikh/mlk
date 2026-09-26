//! The driver, for a browser.
//!
//! This is the wasm host of [`mlkc_driver`]: the same driver the CLI drives,
//! with the text of a buffer pushed into it and the trees pulled out.
//! Nothing here knows about a file system —
//! in a browser there is none, and the driver never wanted one:
//! an editor pushes the text it holds, and reads back what the pipeline made of it.
//!
//! The boundary is deliberately thin: this module converts values and nothing else,
//! so the browser and the CLI cannot drift apart in what they ask the driver to do.
//!
//! The trees cross it in the shape the syntax tree itself defines:
//! a node is its kind, its range and its children, a token is its kind, its range and its text.
//! A host walks that as deeply as it likes — folds it, prints it, jumps from it to the text —
//! and no conversion here decides what a tree is.

use mlkc_driver::{Driver, Lowered};
use mlkc_hir_def::dump;
use mlkc_line_index::LineIndex;
use mlkc_syntax::{ModuleRoot, SyntaxNode, TextRange};
use mlkc_vfs::VfsPath;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// A driver a browser can own.
#[wasm_bindgen]
pub struct WasmDriver {
    driver: Driver,
}

#[wasm_bindgen]
impl WasmDriver {
    /// A driver that knows nothing: the editor pushes what it wants compiled.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            driver: Driver::new(),
        }
    }

    /// Feeds the text of a file into the driver; `null` or `undefined` means the file is gone.
    ///
    /// Returns whether the contents changed.
    /// Pushing the same text again changes nothing and rebuilds nothing,
    /// which is what makes "analyze the buffer after every keystroke" affordable.
    ///
    /// The name a host sees is `setText`: wasm-bindgen keeps the Rust name otherwise,
    /// and the editor around this module is written in JavaScript.
    #[wasm_bindgen(js_name = setText)]
    pub fn set_text(&mut self, path: &str, text: Option<String>) -> bool {
        self.driver.set_file_text(path_of(path), text)
    }

    /// Everything a host reads from the parse of one file:
    /// the concrete syntax tree, the typed view over it, the HIR, and the diagnostics.
    ///
    /// The trees are values a host navigates, not text it re-parses:
    /// a node is its kind, its range and its children, a token is its kind, its range and its text.
    ///
    /// Throws when the driver holds no text for the file:
    /// an editor pushes the buffer before it asks about it.
    pub fn analyze(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.analysis(path)?)
    }
}

impl WasmDriver {
    /// Everything the editor shows about one buffer.
    fn analysis(&mut self, path: &str) -> Result<Analysis, JsValue> {
        let file = self.file(path)?;
        let parse = self
            .driver
            .parse(file)
            .ok_or_else(|| failure(&format!("{path} is not text the driver can parse")))?;
        let lowered = self.driver.lower(file);
        let diagnostics = self
            .driver
            .diagnostics(file)
            .ok_or_else(|| failure(&format!("{path} has no diagnostics to read")))?;
        let index = self
            .driver
            .line_index(file)
            .ok_or_else(|| failure(&format!("{path} has no lines to read")))?;

        Ok(Analysis {
            cst: parse.syntax(),
            ast: parse.module_root(),
            hir: lowered.map(|lowered| Hir::of(&lowered)),
            diagnostics: diagnostics
                .iter()
                .map(|it| Diagnostic::of(it, &index))
                .collect(),
        })
    }

    /// The id the driver knows a path under, or a thrown error when it holds nothing for it.
    fn file(&self, path: &str) -> Result<mlkc_vfs::FileId, JsValue> {
        self.driver
            .file_id(&path_of(path))
            .ok_or_else(|| failure(&format!("no file was pushed at {path}")))
    }
}

impl Default for WasmDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything the editor shows about one buffer.
///
/// The shape of the trees is the one the syntax tree serializes itself into,
/// and the one of the diagnostics is the one an editor marks a buffer with;
/// the types of the editor mirror both, and that is the only place the two sides meet.
#[derive(Serialize)]
struct Analysis {
    /// The concrete syntax tree: lossless, tokens and trivia included.
    cst: SyntaxNode,

    /// The typed view over the same tree,
    /// or `null` when the parse did not find a module root.
    ast: Option<ModuleRoot>,

    /// The HIR of the module, or `null` when there is nothing to lower.
    hir: Option<Hir>,

    /// What the parser and the lowering reported, in the shape an editor marks the buffer with.
    diagnostics: Vec<Diagnostic>,
}

/// The HIR of one module, as a host reads it.
#[derive(Serialize)]
struct Hir {
    /// The lines of the HIR, in the order a reader reads them: the module and its items,
    /// and then a body per entity that owns one.
    nodes: Vec<HirNode>,
}

impl Hir {
    /// Reads the HIR of a lowered module the way a host reads it.
    ///
    /// A line of the reading is about a node of the HIR ([`dump::Target`]), and what a host
    /// marks a buffer by is a range of it: the line of the item tree is read against the
    /// syntax of the module, and the line of a body against the places the lowering of that
    /// body recorded.
    fn of(lowered: &Lowered) -> Self {
        let mut nodes = Vec::new();

        for node in dump::item_tree_nodes(lowered.item_tree()) {
            nodes.push(HirNode::of(&node, &|target| surface_range(lowered, target)));
        }

        for body in lowered.bodies() {
            let reading = dump::body_nodes(body.owner(), &body.body().body);
            let places = &body.body().source_map;

            nodes.push(HirNode::of(&reading, &|target| {
                match target {
                    dump::Target::Expr(expr) => places.expr(*expr),
                    dump::Target::Pat(pat) => places.pat(*pat),
                    dump::Target::Path(path) => places.path(*path),
                    // What a body names can be a place outside it: the entity of the module.
                    dump::Target::Item(item) => lowered.item_range(item),
                    // A body writes no type: its annotations are the ones of the signature it is
                    // a body of, which the surface of the module is read for.
                    dump::Target::Type { .. } => None,
                }
            }));
        }

        Self { nodes }
    }
}

/// One line of the HIR, and the lines under it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HirNode {
    /// The text of the line: `fun main  @2.0`, `param: Int -> use Int`.
    text: String,

    /// What the line says, as the parts a host paints: a line is one part unless a part of it
    /// says something the host paints differently from the rest, which is what the type
    /// a signature writes does --- a type written as a path reads the way a path reads.
    parts: Vec<HirPart>,

    /// What the line is, which is what a host paints it by.
    kind: &'static str,

    /// The part of the buffer the line is about, in bytes, or nothing where the line is about
    /// something a module did not write: a section header, a field of a declaration.
    range: Option<[u32; 2]>,

    /// The part of the buffer what the line resolves to is written at, when the line is about
    /// a path that named something: the declaration a name comes from, the binding it stands
    /// for. Nothing for a line that names nothing, and for a name of another module, which is
    /// written in a file this one is not marked against.
    resolves: Option<[u32; 2]>,

    /// The lines under it, which a host folds.
    children: Vec<HirNode>,
}

/// One part of the text of a line.
#[derive(Serialize)]
struct HirPart {
    /// What this part of the line says.
    text: String,

    /// What it is, when it is not what the line is; `null` for the part of a line that says
    /// what the line says.
    kind: Option<&'static str>,
}

impl HirNode {
    /// Reads one line of a reading, and everything under it.
    ///
    /// `range` is where the module says a node of the HIR is written, and it is the one thing
    /// the reading of the HIR does not know: a line says what a node is, and the driver is
    /// what says where it was read from.
    fn of(node: &dump::Node, range: &impl Fn(&dump::Target) -> Option<TextRange>) -> Self {
        Self {
            text: node.text(),
            parts: node
                .parts
                .iter()
                .map(|part| {
                    HirPart {
                        text: part.text.clone(),
                        kind: part.kind.map(kind_of),
                    }
                })
                .collect(),
            kind: kind_of(node.kind),
            range: node.target.as_ref().and_then(range).map(covered),
            resolves: node.resolves.as_ref().and_then(range).map(covered),
            children: node
                .children
                .iter()
                .map(|child| Self::of(child, range))
                .collect(),
        }
    }
}

/// Where what a line of the surface of a module is about is written: an entity of the module,
/// or a type written in the declaration of one.
fn surface_range(lowered: &Lowered, target: &dump::Target) -> Option<TextRange> {
    match target {
        dump::Target::Item(item) => lowered.item_range(item),
        dump::Target::Type { item, place } => lowered.type_range(item, *place),
        _ => None,
    }
}

/// What a line of a reading is, as a host reads it.
fn kind_of(kind: dump::NodeKind) -> &'static str {
    match kind {
        dump::NodeKind::Module => "module",
        dump::NodeKind::Body => "body",
        dump::NodeKind::Section => "section",
        dump::NodeKind::Item => "item",
        dump::NodeKind::Field => "field",
        dump::NodeKind::Expr => "expr",
        dump::NodeKind::Pat => "pat",
        dump::NodeKind::Path => "path",
    }
}

/// The part of a buffer a range covers, in the bytes a host counts.
fn covered(range: TextRange) -> [u32; 2] {
    [u32::from(range.start()), u32::from(range.end())]
}

/// A diagnostic as an editor reads it: what to say, and where to point.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    /// `error`, `warning`, `note`, or `help`.
    level: String,

    /// `parser` for everything the parser reports.
    category: String,

    /// The code of the kind of mistake within its category.
    code: String,

    /// The message, without the location: the location is in the labels.
    message: String,

    /// Where the diagnostic points, the primary one first.
    labels: Vec<Label>,

    /// What the diagnostic has to add: a hint, an alternative, a note.
    notes: Vec<String>,
}

impl Diagnostic {
    /// Renders a diagnostic for a host.
    ///
    /// The line and the column are counted from zero and in bytes,
    /// because that is what a byte offset turns into without reading the text again.
    /// An editor that speaks another unit — CodeMirror counts UTF-16 code units —
    /// converts the column of the line it already has.
    fn of(diagnostic: &mlkc_diagnostics::Diagnostic, index: &LineIndex) -> Self {
        Self {
            level: diagnostic.level.as_str().to_string(),
            category: diagnostic.category.as_str().to_string(),
            code: diagnostic.code.to_string(),
            message: diagnostic.message.clone(),
            labels: diagnostic
                .labels
                .iter()
                .map(|label| {
                    Label {
                        start: u32::from(label.span.range.start()),
                        end: u32::from(label.span.range.end()),
                        line: index.line_col(label.span.range.start()).line,
                        column: index.line_col(label.span.range.start()).col,
                        primary: label.primary,
                        message: label.message.clone(),
                    }
                })
                .collect(),
            notes: diagnostic.notes.clone(),
        }
    }
}

/// One place a diagnostic points at.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Label {
    /// The start of the range, in bytes into the file.
    start: u32,

    /// The end of the range, in bytes into the file.
    end: u32,

    /// The line the range starts on, counted from zero.
    line: u32,

    /// The column of that line, counted in bytes from zero.
    column: u32,

    /// Whether this is the place the diagnostic is about, rather than a place it mentions.
    primary: bool,

    /// What the label says: often empty for the primary one, since the message is above it.
    message: String,
}

/// The path of a buffer, as the driver spells paths.
///
/// A browser has no file system, so the path is a name the editor invents,
/// and a virtual one: `/main.mlk` for the buffer the editor holds.
fn path_of(path: &str) -> VfsPath {
    let path = path.strip_prefix('/').unwrap_or(path);

    VfsPath::new_virtual_path(format!("/{path}"))
}

/// A failure that crosses the boundary as an exception.
fn failure(message: &str) -> JsValue {
    JsValue::from_str(message)
}

/// Hands a value to a JavaScript host the way a host reads it.
///
/// The trees of the syntax are maps, and a host reads a map as an object, not as a `Map`;
/// an absent value is `null`, not `undefined`, so that what a host sees
/// does not depend on where it looks.
fn to_js(value: &impl Serialize) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_maps_as_objects(true)
        .serialize_missing_as_null(true);

    value
        .serialize(&serializer)
        .map_err(|error| failure(&format!("failed to cross the boundary: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wasm module is compiled for the host in tests,
    /// so everything but the conversions is exercised where it is cheap to inspect.
    #[test]
    fn a_path_becomes_the_path_of_a_virtual_file() {
        assert_eq!(path_of("main.mlk"), path_of("/main.mlk"));
        assert_eq!(
            path_of("/main.mlk"),
            VfsPath::new_virtual_path("/main.mlk".to_string())
        );
    }

    #[test]
    fn a_pushed_buffer_analyzes_into_trees_and_diagnostics() {
        let mut driver = WasmDriver::new();

        assert!(driver.set_text("/main.mlk", Some("fun main(): Unit =\n    x\n".to_string())));
        let analysis = driver.analysis("/main.mlk").expect("the file to analyze");
        let json = serde_json::to_value(&analysis).expect("the analysis to serialize");

        let cst = &json["cst"];

        assert_eq!(cst["kind"], "MODULE_ROOT");
        assert!(
            cst["text_range"].is_array(),
            "a host reads the range of a node as a pair of offsets"
        );
        assert!(cst["children"].is_array(), "a host walks the children");
        assert_eq!(
            token_text(cst, "FUN_KW").as_deref(),
            Some("fun "),
            "a token carries the trivia that follows it"
        );
        assert_eq!(token_text(cst, "IDENT").as_deref(), Some("main"));
        assert!(
            token_text(cst, "WHITESPACE").is_none(),
            "trivia is not a child: it belongs to the token it follows"
        );

        let ast = &json["ast"];
        let decl = &ast["fields"]["items"]["items"][0];

        assert_eq!(ast["kind"], "ModuleRoot", "a node says what it is");
        assert_eq!(ast["fields"]["items"]["kind"], "ModuleItemList");
        assert_eq!(
            decl["kind"], "FunDecl",
            "a union serializes as the node it holds"
        );
        assert_eq!(
            decl["fields"]["fun_token"]["Ok"]["kind"], "FUN_KW",
            "a required field holds a token, or nothing"
        );
        assert!(
            ast["fields"]["bom_token"].is_null(),
            "an optional field that is missing is null"
        );
        assert!(analysis.diagnostics.is_empty(), "the module parses cleanly");
    }

    #[test]
    fn the_hir_reads_as_a_tree_a_host_folds_and_marks_the_buffer_by() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "fun main(): Unit =\n    let x = 1 in\n    x\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let analysis = driver.analysis("/main.mlk").expect("the file to analyze");
        let json = serde_json::to_value(&analysis).expect("the analysis to serialize");
        let nodes = json["hir"]["nodes"]
            .as_array()
            .expect("the HIR to hold lines");

        // The module is read first, and it is a line about nothing a host can mark.
        assert_eq!(nodes[0]["text"], "MODULE #0");
        assert_eq!(nodes[0]["kind"], "module");
        assert!(nodes[0]["range"].is_null());

        // The surface of the module follows, one line per entity and one per thing it is.
        let item = &nodes[1]["children"][0];

        assert_eq!(nodes[1]["text"], "ITEM TREE");
        assert_eq!(nodes[1]["kind"], "section");
        assert_eq!(item["text"], "fun main  @2.0");
        assert_eq!(item["kind"], "item");
        assert_eq!(covered(source, &item["range"]), source.trim_end());
        assert_eq!(item["children"][0]["text"], "visibility: private");
        assert_eq!(item["children"][1]["text"], "ret: Unit -> unresolved");
        assert!(
            item["children"][0]["range"].is_null(),
            "a field of a declaration is about nothing a host marks"
        );

        // And then the body of the function, as the tree of what it holds.
        let body = &nodes[2];

        assert_eq!(body["text"], "BODY fun main in module #0");
        assert_eq!(body["kind"], "body");
        assert_eq!(covered(source, &body["range"]), source.trim_end());

        let root = &body["children"][0];
        let declaration = &root["children"][0];

        assert_eq!(root["text"], "root");
        assert_eq!(declaration["text"], "expr#2  let pat#0 = expr#0 in expr#1");
        assert_eq!(
            covered(source, &declaration["range"]),
            "let x = 1 in\n    x"
        );

        let pat = &declaration["children"][0];
        let literal = &declaration["children"][1];

        assert_eq!(pat["text"], "pat#0  bind x");
        assert_eq!(covered(source, &pat["range"]), "x");
        assert_eq!(literal["text"], "expr#0  literal 1");
        assert_eq!(covered(source, &literal["range"]), "1");

        // A path says what it names as well as what it is: the `x` of the body is the binding
        // the `let` introduced, and a host marks both ends of that walk.
        let path = &declaration["children"][2]["children"][0];

        assert_eq!(path["text"], "path#0  x -> binding pat#0");
        assert_eq!(covered(source, &path["range"]), "x");
        assert_eq!(covered(source, &path["resolves"]), "x");
        assert_ne!(
            path["range"], path["resolves"],
            "the path and the binding it names are two places in the source"
        );
    }

    #[test]
    fn a_path_of_a_signature_marks_the_type_and_what_it_names() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "use std.core.Int\n\nfun main(value: Int): Int =\n    value\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let analysis = driver.analysis("/main.mlk").expect("the file to analyze");
        let json = serde_json::to_value(&analysis).expect("the analysis to serialize");
        let items = json["hir"]["nodes"][1]["children"]
            .as_array()
            .expect("the items of the module");
        let item = items
            .iter()
            .find(|it| {
                it["text"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("fun main"))
            })
            .expect("the module to declare a function called `main`");
        let param = &item["children"][1];
        let result = &item["children"][2];

        assert_eq!(param["text"], "param: Int -> use Int");
        assert_eq!(
            param["parts"],
            serde_json::json!([
                { "text": "param", "kind": null },
                { "text": ": ", "kind": null },
                { "text": "Int -> use Int", "kind": "path" },
            ]),
            "the type of a parameter is a part of its own, and it reads as a path"
        );
        assert_eq!(covered(source, &param["range"]), "Int");
        assert_eq!(
            covered(source, &param["resolves"]),
            "use std.core.Int",
            "a type a signature writes marks what its name comes from"
        );

        // The two types are written at two places of the same declaration, and each line
        // points at its own.
        assert_eq!(result["text"], "ret: Int -> use Int");
        assert_eq!(covered(source, &result["range"]), "Int");
        assert_ne!(param["range"], result["range"]);
    }

    #[test]
    fn a_path_that_names_an_import_points_at_the_import() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "use std.core.Int\n\nfun main(): Int =\n    Int\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let analysis = driver.analysis("/main.mlk").expect("the file to analyze");
        let json = serde_json::to_value(&analysis).expect("the analysis to serialize");
        let body = &json["hir"]["nodes"][2];
        let path = &body["children"][0]["children"][0]["children"][0];

        assert_eq!(path["text"], "path#0  Int -> use Int");
        assert_eq!(covered(source, &path["range"]), "Int");
        assert_eq!(
            covered(source, &path["resolves"]),
            "use std.core.Int",
            "a name an import brought in leads to the import"
        );
    }

    #[test]
    fn a_lowering_mistake_crosses_the_boundary_as_a_diagnostic() {
        let mut driver = WasmDriver::new();

        driver.set_text(
            "/main.mlk",
            Some("fun f(): Unit =\n    1\n\nfun f(): Unit =\n    2\n".to_string()),
        );
        let analysis = driver.analysis("/main.mlk").expect("the file to analyze");
        let json = serde_json::to_value(&analysis).expect("the analysis to serialize");
        let diagnostics = json["diagnostics"].as_array().expect("diagnostics");

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "lowering");
        assert_eq!(diagnostics[0]["code"], "01");
        assert_eq!(diagnostics[0]["level"], "error");
    }

    /// The text of the first token of a kind, wherever in a serialized tree it sits.
    fn token_text(node: &serde_json::Value, kind: &str) -> Option<String> {
        if node["kind"] == kind {
            return node["text"].as_str().map(str::to_string);
        }

        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| token_text(child, kind))
    }

    #[test]
    fn a_mistake_becomes_a_diagnostic_a_host_can_point_at() {
        let mut driver = WasmDriver::new();

        driver.set_text(
            "/main.mlk",
            Some("fun main(): Unit =\n    let x = 1\n".to_string()),
        );
        let file = driver.file("/main.mlk").expect("the file to be known");
        let diagnostics = driver
            .driver
            .diagnostics(file)
            .expect("the file to be parsed");
        let index = driver
            .driver
            .line_index(file)
            .expect("the file to have lines");
        let diagnostic = Diagnostic::of(&diagnostics[0], &index);

        assert_eq!(diagnostic.level, "error");
        assert_eq!(diagnostic.category, "parser");

        let label = diagnostic
            .labels
            .first()
            .expect("a label to point somewhere");

        assert!(label.primary);
        assert_eq!(
            label.line, 2,
            "the missing `in` is reported at the end of the file"
        );
        assert!(label.end >= label.start);
    }

    #[test]
    fn a_node_the_grammar_has_no_room_for_serializes_what_it_holds() {
        let mut driver = WasmDriver::new();

        driver.set_text(
            "/main.mlk",
            Some("fun main(): Unit =\n    x\nabc\n".to_string()),
        );
        let analysis = driver.analysis("/main.mlk").expect("the file to analyze");
        let json = serde_json::to_value(&analysis).expect("the analysis to serialize");
        let bogus = bogus_node(&json["ast"]).expect("the mistake to leave a node of no kind");

        assert!(
            bogus["items"].is_array(),
            "what such a node holds is a list of elements, as it is for any list"
        );
        assert!(
            bogus.get("syntax").is_none(),
            "the syntax a node wraps is its own business, not something a host reads"
        );
    }

    /// The first value of a serialized tree whose kind is one the grammar has no room for.
    fn bogus_node(value: &serde_json::Value) -> Option<&serde_json::Value> {
        if let Some(kind) = value["kind"].as_str()
            && kind.starts_with("Bogus")
        {
            return Some(value);
        }

        match value {
            serde_json::Value::Array(items) => items.iter().find_map(bogus_node),
            serde_json::Value::Object(entries) => entries.values().find_map(bogus_node),
            _ => None,
        }
    }
}
