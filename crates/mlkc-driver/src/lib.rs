//! The driver of the compiler.
//!
//! The driver is the component that owns the inputs and the memoized passes,
//! and the only one that decides what has to be recomputed.
//! The pipeline it drives is the compiler pipeline;
//! today that pipeline is the parser,
//! so the table holds the values that are derived from the text of a file:
//! the parse, the diagnostics a host renders, and the line index positions are read with.
//! The units that follow — the item tree, the interface, the checked bodies —
//! are more slots in the same table rather than a different design.
//!
//! Three rules are visible in the code.
//!
//! - **Inputs are pushed, values are pulled**.
//!   The driver never opens a file, never reads a clock, never writes anywhere:
//!   a host hands it bytes, and asks it for values.
//! - **The key of a slot is the identity of what the pass read**.
//!   Everything here is a function of the text of one file,
//!   so the identity of that text — its [`FileVersion`] — is the whole key,
//!   while the text itself is handed to the pass by reference.
//! - **A pass is a function of its input**.
//!   [`Parse`] is what [`mlkc_parser::parse`] returned, and nothing more:
//!   the value and the diagnostics of the pass, stored as they came.
//!
//! # Concurrency
//!
//! A pull takes `&mut self`, because it may compute, so the driver is not shared:
//! one thread owns it and asks it for values.
//! What that thread hands out are `Arc`s, which are immutable and safe to read anywhere,
//! so a host answers the requests that only read from the values it already pulled,
//! and sends the ones that need computing to the thread that owns the driver.
//!
//! Nothing read takes a lock on the driver, so a long pull cannot block a reader;
//! and a pull a host abandons leaves the table as valid as it found it,
//! because a slot is written only when its value is complete.

use std::sync::Arc;

use mlkc_diagnostics::Diagnostic;
use mlkc_line_index::LineIndex;
use mlkc_parser_core::{AnyParse, diagnostic::ParseDiagnostic};
use mlkc_rowan::AstNode;
use mlkc_syntax::{ModuleRoot, SyntaxNode};
use mlkc_vfs::{ChangedFile, FileId, FileState, FileVersion, Vfs, VfsPath};
use rustc_hash::FxHashMap;

/// The driver: the only mutable component, and the owner of the memo table.
#[derive(Default)]
pub struct Driver {
    /// The state of every file a host has pushed.
    vfs: Vfs,
    /// The parse of every file that has been parsed.
    parses: FxHashMap<FileId, TextSlot<Parse>>,
    /// The diagnostics of every file that has been asked for them.
    diagnostics: FxHashMap<FileId, TextSlot<[Diagnostic]>>,
    /// The line index of every file whose positions have been read.
    line_indices: FxHashMap<FileId, TextSlot<LineIndex>>,
}

/// The shape of a slot whose input is the text of one file.
///
/// The version is the whole key: everything the driver derives from a file
/// is a function of the text of that file, so a slot whose version is still the version
/// of the file cannot have been built from anything else.
struct TextSlot<T: ?Sized> {
    /// The version of the contents the value was built from.
    version: FileVersion,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<T>,
}

/// The value of the parse slot: what the parser returned, whole.
///
/// The tree is immutable and shared, so whoever holds it
/// holds the text it was parsed from, whatever the file holds now.
pub struct Parse {
    parse: AnyParse,
}

impl Parse {
    /// Runs the pass.
    ///
    /// This is the only place where the driver touches the parser:
    /// the value is stored as the parser produced it,
    /// which is what makes the slot's key the input of the pass.
    fn of(source: &str) -> Self {
        Self {
            parse: mlkc_parser::parse(source),
        }
    }

    /// The concrete syntax tree: lossless, and never absent, however broken the input.
    pub fn syntax(&self) -> SyntaxNode {
        self.parse.syntax()
    }

    /// The typed view over the tree,
    /// or `None` when the parse did not find a module root — a tree is not a module by itself.
    pub fn module_root(&self) -> Option<ModuleRoot> {
        ModuleRoot::cast(self.syntax())
    }

    /// The diagnostics the parse produced, in the shape the parser knows them.
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        self.parse.diagnostics()
    }

    /// Whether the parse reported an error.
    pub fn has_errors(&self) -> bool {
        self.parse.has_errors()
    }
}

impl std::fmt::Debug for Parse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parse")
            .field("diagnostics", &self.parse.diagnostics())
            .finish_non_exhaustive()
    }
}

impl Driver {
    /// A driver that knows nothing: the host pushes what it wants compiled.
    pub fn new() -> Self {
        Self::default()
    }

    // Inputs: the driver never reaches for any of this.

    /// Feeds the contents of a file into the driver; `None` means the file is gone.
    ///
    /// Returns whether the contents changed,
    /// and pushing the same contents again changes nothing.
    pub fn set_file_contents(&mut self, path: VfsPath, contents: Option<Vec<u8>>) -> bool {
        self.vfs.set_file_contents(path, contents)
    }

    /// A convenience for a host that already holds text:
    /// an editor, a WASM shim, a test.
    pub fn set_file_text(&mut self, path: VfsPath, text: Option<String>) -> bool {
        self.vfs.set_file_text(path, text)
    }

    // Reads: no computation, and none of them takes `&mut self`.

    /// The id of a path the driver knows, if the file is there.
    pub fn file_id(&self, path: &VfsPath) -> Option<FileId> {
        self.vfs.file_id(path).map(|(file, _)| file)
    }

    /// The contents of a file, or `None` if it is missing, unreadable, or excluded.
    pub fn file_text(&self, file: FileId) -> Option<Arc<str>> {
        self.vfs.file_text(file)
    }

    /// The version of the contents of a file, which is the identity of its state.
    pub fn file_version(&self, file: FileId) -> FileVersion {
        self.vfs.file_version(file)
    }

    /// What the driver knows about a file: read or not, text or not, there or not.
    pub fn file_state(&self, file: FileId) -> FileState {
        self.vfs.file_state(file)
    }

    /// The path a file was interned under.
    pub fn file_path(&self, file: FileId) -> &VfsPath {
        self.vfs.file_path(file)
    }

    // Pulls: they may compute, and they never answer from an invalid slot.

    /// The parse of `file`, computed when the slot is missing or stale.
    ///
    /// `None` means the driver has no text for the file:
    /// it was never pushed, it is gone, or it is not text at all.
    /// [`Driver::file_state`] tells which of those it is.
    pub fn parse(&mut self, file: FileId) -> Option<Arc<Parse>> {
        let version = self.file_version(file);
        let text = self.file_text(file);

        // There is nothing to back-date here: the parse is a function of the text,
        // and the tree of different text is a different tree.
        // The stages where a recomputation can end up equal to the retained value —
        // the item tree, the interface — are the ones that follow.
        Self::text_derived(&mut self.parses, file, version, || {
            text.map(|text| Arc::new(Parse::of(&text)))
        })
    }

    /// The diagnostics of the parse of `file`, in the shape a host renders.
    ///
    /// The conversion is a value of its own, not work done per call:
    /// it is computed once per version of the file and shared as an `Arc`.
    ///
    /// `None` when the file has no parse: see [`Driver::parse`].
    pub fn diagnostics(&mut self, file: FileId) -> Option<Arc<[Diagnostic]>> {
        let Some(parse) = self.parse(file) else {
            self.diagnostics.remove(&file);
            return None;
        };

        let version = self.file_version(file);

        Self::text_derived(&mut self.diagnostics, file, version, || {
            let rendered = parse
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.to_diagnostic(file))
                .collect::<Vec<_>>();

            Some(Arc::from(rendered))
        })
    }

    /// The line index of `file`: where its lines start and end.
    ///
    /// A person and a protocol read a position as a line and a column, a diagnostic points
    /// at a byte range, and the index is the mapping between the two.
    /// It is derived from the text like everything else here,
    /// so nothing computes it before somebody asks.
    pub fn line_index(&mut self, file: FileId) -> Option<Arc<LineIndex>> {
        let version = self.file_version(file);
        let text = self.file_text(file);

        Self::text_derived(&mut self.line_indices, file, version, || {
            text.map(|text| Arc::new(LineIndex::new(&text)))
        })
    }

    /// The net effect of the pushes a host made since its last call.
    ///
    /// The driver does not need this: a slot decides its own validity by comparing versions,
    /// and the driver knows which files were pushed.
    /// A host does, to know which documents to publish again —
    /// a file that appeared and disappeared between two calls is not one of them.
    pub fn take_changes(&mut self) -> Vec<ChangedFile> {
        self.vfs.take_changes().into_values().collect()
    }

    /// The value of a slot keyed by the version of a file,
    /// computed when the slot is missing or stale and remembered otherwise.
    fn text_derived<T: ?Sized>(
        slots: &mut FxHashMap<FileId, TextSlot<T>>,
        file: FileId,
        version: FileVersion,
        build: impl FnOnce() -> Option<Arc<T>>,
    ) -> Option<Arc<T>> {
        if let Some(slot) = slots.get(&file)
            && slot.version == version
        {
            return Some(slot.value.clone());
        }

        let Some(value) = build() else {
            // There is no input left to describe, so the slot goes.
            // The next push of this file builds it again from nothing.
            slots.remove(&file);
            return None;
        };

        slots.insert(file, TextSlot {
            version,
            value: value.clone(),
        });

        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use mlkc_diagnostics::{Category, Level};
    use mlkc_line_index::LineCol;
    use mlkc_rowan::AstNodeList;
    use mlkc_text_size::TextLen;
    use mlkc_vfs::Change;

    use super::*;

    /// A module of the language, written the way a person writes one.
    const MODULE: &str =
        "fun main(): Unit =\n    let x = 42 * 2 - 10 in\n    println-int(x + 20)\n";

    /// The same module with the `in` of the `let` missing.
    const BROKEN: &str = "fun main(): Unit =\n    let x = 1\n";

    /// A path in the virtual file system: a test has no file system.
    fn path(name: &str) -> VfsPath {
        VfsPath::new_virtual_path(format!("/{name}"))
    }

    /// A driver that holds one file, and the id of that file.
    fn driver_with(name: &str, text: &str) -> (Driver, FileId) {
        let mut driver = Driver::new();
        let path = path(name);

        driver.set_file_text(path.clone(), Some(text.to_string()));

        let file = driver.file_id(&path).expect("the file to have an id");

        (driver, file)
    }

    #[test]
    fn the_parse_holds_the_source_it_was_parsed_from() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);

        let parse = driver.parse(file).expect("the file to be parsed");

        assert_eq!(parse.syntax().to_string(), MODULE);
        assert!(!parse.has_errors());
    }

    #[test]
    fn the_typed_view_names_the_items_of_the_module() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);

        let parse = driver.parse(file).expect("the file to be parsed");
        let root = parse
            .module_root()
            .expect("the root of a module to be a module");

        assert_eq!(root.items().len(), 1);
    }

    #[test]
    fn a_second_pull_is_the_value_the_first_one_returned() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);

        let first = driver.parse(file).expect("the file to be parsed");
        let second = driver.parse(file).expect("the file to be parsed");

        assert!(Arc::ptr_eq(&first, &second), "the slot was built twice");
    }

    #[test]
    fn pushing_the_same_text_again_changes_nothing() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let first = driver.parse(file).expect("the file to be parsed");

        let changed = driver.set_file_text(path("main.mlk"), Some(MODULE.to_string()));

        assert!(!changed, "the contents were the same");
        assert!(Arc::ptr_eq(
            &first,
            &driver.parse(file).expect("the file to be parsed")
        ));
    }

    #[test]
    fn a_changed_file_is_parsed_again_and_the_old_value_keeps_its_text() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let before = driver.parse(file).expect("the file to be parsed");

        driver.set_file_text(path("main.mlk"), Some(BROKEN.to_string()));
        let after = driver.parse(file).expect("the file to be parsed");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not rebuilt");
        assert_eq!(after.syntax().to_string(), BROKEN);
        assert_eq!(
            before.syntax().to_string(),
            MODULE,
            "a value is immutable, so it keeps the text it was parsed from"
        );
    }

    #[test]
    fn a_deleted_file_has_no_parse() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);

        driver.set_file_text(path("main.mlk"), None);

        assert_eq!(driver.file_state(file), FileState::Deleted);
        assert!(driver.parse(file).is_none());
    }

    #[test]
    fn a_file_that_is_not_text_has_no_parse() {
        let mut driver = Driver::new();
        let path = path("binary.mlk");

        driver.set_file_contents(path.clone(), Some(vec![0xFF, 0xFE]));
        let file = driver.file_id(&path).expect("the file to have an id");

        assert_eq!(driver.file_state(file), FileState::Unreadable);
        assert!(driver.parse(file).is_none());
    }

    #[test]
    fn the_diagnostics_of_a_broken_module_travel_with_the_parse() {
        let (mut driver, file) = driver_with("main.mlk", BROKEN);

        let parse = driver.parse(file).expect("the file to be parsed");

        assert!(
            parse.has_errors(),
            "a missing `in` is a mistake the parser reports"
        );

        let diagnostics = driver.diagnostics(file).expect("the file to be parsed");
        let diagnostic = diagnostics.first().expect("the parse to report something");

        assert_eq!(diagnostic.level, Level::Error);
        assert_eq!(diagnostic.category, Category::Parser);
        assert_eq!(diagnostic.labels.first().expect("a label").span.file, file);
    }

    #[test]
    fn the_diagnostics_are_converted_once() {
        let (mut driver, file) = driver_with("main.mlk", BROKEN);

        let first = driver.diagnostics(file).expect("the file to be parsed");
        let second = driver.diagnostics(file).expect("the file to be parsed");

        assert!(Arc::ptr_eq(&first, &second), "the conversion ran twice");
    }

    #[test]
    fn the_diagnostics_follow_the_text_and_the_old_ones_stay_as_they_were() {
        let (mut driver, file) = driver_with("main.mlk", BROKEN);
        let broken = driver.diagnostics(file).expect("the file to be parsed");

        assert!(!broken.is_empty());

        driver.set_file_text(path("main.mlk"), Some(MODULE.to_string()));
        let fixed = driver.diagnostics(file).expect("the file to be parsed");

        assert!(fixed.is_empty(), "the module parses cleanly now");
        assert!(
            !broken.is_empty(),
            "a value does not change under the one that holds it"
        );
    }

    #[test]
    fn the_line_index_follows_the_text() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let index = driver.line_index(file).expect("the file to have an index");

        assert!(Arc::ptr_eq(
            &index,
            &driver.line_index(file).expect("the file to have an index")
        ));
        assert_eq!(
            index.line_count(),
            4,
            "three lines of source and the line the trailing break makes"
        );
        assert_eq!(
            &MODULE[index.line_range(2).expect("a third line")],
            "    println-int(x + 20)"
        );
        assert_eq!(index.line_col(MODULE.text_len()), LineCol {
            line: 3,
            col: 0
        });

        driver.set_file_text(path("main.mlk"), Some("x\n".to_string()));
        let rebuilt = driver.line_index(file).expect("the file to have an index");

        assert!(!Arc::ptr_eq(&index, &rebuilt), "the index was not rebuilt");
        assert_eq!(rebuilt.line_count(), 2);
    }

    #[test]
    fn take_changes_reports_the_net_effect_since_the_last_call() {
        let mut driver = Driver::new();
        let path = path("main.mlk");
        let others = ["one", "two"];

        driver.set_file_text(path.clone(), Some(others[0].to_string()));

        let created = driver.take_changes();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].change, Change::Create);
        assert!(driver.take_changes().is_empty(), "a drain drains");

        driver.set_file_text(path.clone(), Some(others[0].to_string()));
        assert!(
            driver.take_changes().is_empty(),
            "the same contents are not a change"
        );

        driver.set_file_text(path.clone(), Some(others[1].to_string()));
        assert_eq!(driver.take_changes()[0].change, Change::Modify);

        driver.set_file_text(path, None);
        assert_eq!(driver.take_changes()[0].change, Change::Delete);
    }
}
