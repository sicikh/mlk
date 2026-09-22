//! The driver, for a browser.
//!
//! This is the wasm host of [`mlkc_driver`]: the same driver the CLI drives,
//! with the text of a buffer pushed into it and the trees pulled out ([ADR-0008]).
//! Nothing here knows about a file system —
//! in a browser there is none, and the driver never wanted one:
//! an editor pushes the text it holds, and reads back what the pipeline made of it.
//!
//! The boundary is deliberately thin: this module converts values and nothing else,
//! so the browser and the CLI cannot drift apart in what they ask the driver to do.
//!
//! The trees cross it in the shape the syntax tree itself defines ([ADR-0002]):
//! a node is its kind, its range and its children, a token is its kind, its range and its text.
//! A host walks that as deeply as it likes — folds it, prints it, jumps from it to the text —
//! and no conversion here decides what a tree is.
//!
//! [ADR-0002]: https://github.com/sicikh/mlk/blob/main/docs/adr/0002-lossless-syntax-tree.md
//! [ADR-0008]: https://github.com/sicikh/mlk/blob/main/docs/adr/0008-compiler-driver.md

use mlkc_driver::Driver;
use mlkc_line_index::LineIndex;
use mlkc_syntax::{ModuleRoot, SyntaxNode};
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
    /// Pushing the same text again changes nothing and rebuilds nothing ([ADR-0007]),
    /// which is what makes "analyze the buffer after every keystroke" affordable.
    ///
    /// The name a host sees is `setText`: wasm-bindgen keeps the Rust name otherwise,
    /// and the editor around this module is written in JavaScript ([ADR-0008]).
    ///
    /// [ADR-0007]: https://github.com/sicikh/mlk/blob/main/docs/adr/0007-vfs-file-state.md
    /// [ADR-0008]: https://github.com/sicikh/mlk/blob/main/docs/adr/0008-compiler-driver.md
    #[wasm_bindgen(js_name = setText)]
    pub fn set_text(&mut self, path: &str, text: Option<String>) -> bool {
        self.driver.set_file_text(path_of(path), text)
    }

    /// Everything a host reads from the parse of one file:
    /// the concrete syntax tree, the typed view over it, and the diagnostics of the parse.
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
/// The shape of the trees is the one the syntax tree serializes itself into ([ADR-0002]),
/// and the one of the diagnostics is the one an editor marks a buffer with;
/// the types of the editor mirror both, and that is the only place the two sides meet.
#[derive(Serialize)]
struct Analysis {
    /// The concrete syntax tree: lossless, tokens and trivia included ([ADR-0002]).
    cst: SyntaxNode,

    /// The typed view over the same tree ([ADR-0002]),
    /// or `null` when the parse did not find a module root.
    ast: Option<ModuleRoot>,

    /// What the parser reported, in the shape an editor marks the buffer with.
    diagnostics: Vec<Diagnostic>,
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
}
