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
//! [ADR-0008]: https://github.com/sicikh/mlk/blob/main/docs/adr/0008-compiler-driver.md

use mlkc_driver::Driver;
use mlkc_line_index::LineIndex;
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
    /// the dump of the concrete syntax tree, the dump of its typed view,
    /// and the diagnostics of the parse.
    ///
    /// Throws when the driver holds no text for the file:
    /// an editor pushes the buffer before it asks about it.
    pub fn analyze(&mut self, path: &str) -> Result<JsValue, JsValue> {
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

        let analysis = Analysis {
            cst: format!("{:#?}", parse.syntax()),
            ast: parse.module_root().map(|root| format!("{root:#?}")),
            diagnostics: diagnostics
                .iter()
                .map(|it| Diagnostic::of(it, &index))
                .collect(),
        };

        serde_wasm_bindgen::to_value(&analysis)
            .map_err(|error| failure(&format!("failed to cross the boundary: {error}")))
    }
}

impl WasmDriver {
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
/// The shape is mirrored by the types of the editor,
/// which is the one place where the two sides of the boundary have to be kept in step.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Analysis {
    /// The concrete syntax tree: lossless, tokens and trivia included ([ADR-0002]).
    ///
    /// [ADR-0002]: https://github.com/sicikh/mlk/blob/main/docs/adr/0002-lossless-syntax-tree.md
    cst: String,

    /// The typed view over the tree ([ADR-0002]),
    /// or `null` when the parse did not find a module root.
    ast: Option<String>,

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
                .map(|label| Label {
                    start: u32::from(label.span.range.start()),
                    end: u32::from(label.span.range.end()),
                    line: index.line_col(label.span.range.start()).line,
                    column: index.line_col(label.span.range.start()).col,
                    primary: label.primary,
                    message: label.message.clone(),
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
        let file = driver.file("/main.mlk").expect("the file to be known");

        let parse = driver.driver.parse(file).expect("the file to be parsed");
        let analysis = Analysis {
            cst: format!("{:#?}", parse.syntax()),
            ast: parse.module_root().map(|root| format!("{root:#?}")),
            diagnostics: driver
                .driver
                .diagnostics(file)
                .expect("the file to be parsed")
                .iter()
                .map(|it| Diagnostic::of(it, &driver.driver.line_index(file).unwrap()))
                .collect(),
        };

        assert!(analysis.cst.contains("MODULE_ROOT"));
        assert!(analysis.ast.expect("a module root").contains("ModuleRoot"));
        assert!(analysis.diagnostics.is_empty(), "the module parses cleanly");
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
