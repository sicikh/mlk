//! The driver, for a browser.
//!
//! This is the wasm host of [`mlkc_driver`]: the same driver the CLI drives, with the text of a
//! buffer pushed into it and the values of the pipeline pulled out. Nothing here knows about a
//! file system --- in a browser there is none, and the driver never wanted one.
//!
//! The boundary is the driver's own: one method per value a host may ask for --- the concrete
//! tree, the typed view, the HIR, the types of a module, and the diagnostics --- and a pull
//! computes what it answers and nothing else. A host that shows the trees only when a person
//! opens them asks for them only then, and the pipeline under the boundary recomputes nothing
//! it already holds. No method bundles the others: what a host does not ask for is not built,
//! and a host that speaks another protocol --- an editor, over LSP --- asks for the same pulls.
//!
//! The standard library is the one thing a host does not push: it is part of the compiler, and
//! a browser has nowhere to read it from, so a host asks the driver for it
//! ([`WasmDriver::use_std`]) and is handed the files it is made of. The library is the
//! compiler's, so the editor shows it as a buffer and writes in none of it.
//!
//! The trees cross the boundary in the shape the syntax tree itself defines:
//! a node is its kind, its range and its children, a token is its kind, its range and its text.
//! A host walks that as deeply as it likes --- folds it, prints it, jumps from it to the text ---
//! and no conversion here decides what a tree is.

use std::sync::Arc;

use mlkc_driver::{Driver, Lowered, Parse};
use mlkc_hir_def::{ItemLoc, ItemLocLike, ModuleId, dump};
use mlkc_hir_ty::Ty;
use mlkc_line_index::LineIndex;
use mlkc_syntax::{ModuleRoot, SyntaxNode, TextRange};
use mlkc_vfs::{FileId, VfsPath};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// The clock of the page, which is the clock a driver in a browser is measured by.
///
/// A browser has no time of its own inside wasm, and this is the time it does have: the
/// milliseconds `performance.now()` counts, which is what a look at the driver costs in the
/// end. A host that is not a browser --- the tests of this crate --- leaves the driver without
/// a clock, and reads the work of the passes rather than the time they took.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

/// A driver a browser can own.
#[wasm_bindgen]
pub struct WasmDriver {
    driver: Driver,
}

/// A driver the page times: a browser has no clock inside wasm, so it hands the driver the one
/// it has (`performance.now`).
#[cfg(target_arch = "wasm32")]
fn timed_driver() -> Driver {
    let mut driver = Driver::new();

    driver.set_clock(now);

    driver
}

/// A driver that is not timed: the module was not built for a browser, which is what the tests
/// of this crate are, and nothing there measures a pass.
#[cfg(not(target_arch = "wasm32"))]
fn timed_driver() -> Driver {
    Driver::new()
}

#[wasm_bindgen]
impl WasmDriver {
    /// A driver that knows nothing: a host pushes the buffers it holds, and asks for the
    /// standard library of the language when it wants it ([`WasmDriver::use_std`]).
    ///
    /// The driver measures the passes it runs by the clock of the page
    /// ([`mlkc_driver::Driver::set_clock`]).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            driver: timed_driver(),
        }
    }

    /// Records the standard library of the language in the driver, and hands over the files it
    /// is made of: the path each is known by, and its text.
    ///
    /// The library is part of the compiler, and a browser has nowhere to read it from, so a
    /// host asks the driver for it rather than pushing text of its own. What comes back is what
    /// a host shows: the library is the compiler's, and nothing in it is a person's to write.
    ///
    /// The name a host sees is `useStd`: wasm-bindgen keeps the Rust name otherwise,
    /// and the editor around this module is written in JavaScript.
    #[wasm_bindgen(js_name = useStd)]
    pub fn use_std(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.register_library())
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

    /// The concrete syntax tree of a buffer: lossless, tokens and trivia included.
    ///
    /// The tree is a value a host navigates, not text it re-parses:
    /// a node is its kind, its range and its children, a token is its kind, its range and its text.
    ///
    /// Throws when the driver holds no text for the file:
    /// an editor pushes the buffer before it asks about it.
    pub fn cst(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.cst_of(path)?)
    }

    /// The typed view over the same tree, or `null` when the parse found no module root.
    ///
    /// Its shape is the one the typed tree gives itself: a field of the compiler's AST is a key
    /// here, a list is an array, and a token is a node of the concrete tree.
    pub fn ast(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.ast_of(path)?)
    }

    /// The HIR of the module in the buffer, or `null` when there is nothing to lower.
    ///
    /// The lines are read the way a person reads the HIR --- the module and its items, and then
    /// a body per entity that owns one --- and each line says where the node it is about is
    /// written, which is what an editor marks the buffer by.
    pub fn hir(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.hir_of(path)?)
    }

    /// What checking the types of the buffer left, or `null` when there is nothing to check.
    ///
    /// The types are read where a person reads them: the type an entity of the module was
    /// resolved to, and the type every node of every body was checked to.
    pub fn types(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.types_of(path)?)
    }

    /// What the stages of the pipeline reported, in the order they reported it.
    ///
    /// The diagnostics are what an editor marks the buffer with: the level, the kind, and the
    /// places they point at, each of them with the line and the column it sits at.
    pub fn diagnostics(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.diagnostics_of(path)?)
    }

    /// What the driver did since this was last asked, by pass and by unit: what it handed over
    /// from a slot, what it read again, what it kept although it read again, what it dropped,
    /// and how long the passes it ran took.
    ///
    /// Reading the counters is what clears them, so a host that wants to know what one pull
    /// cost asks for them right after it: what comes back is that pull's work, and the time it
    /// took is the host's own clock around the call. A unit is what a pass is a value of: the
    /// path of a file or a module, the name of a project, and the name of a body and where it
    /// is written.
    ///
    /// The name a host sees is `stats`: wasm-bindgen keeps the Rust name otherwise.
    #[wasm_bindgen(js_name = stats)]
    pub fn stats(&mut self) -> Result<JsValue, JsValue> {
        to_js(&Stats::of(&mut self.driver))
    }
}

impl WasmDriver {
    /// Records the standard library, and hands over the files it is made of.
    fn register_library(&mut self) -> Vec<StdFile> {
        self.driver
            .use_std()
            .into_iter()
            .map(|file| {
                StdFile {
                    path: file.path.to_string(),
                    text: file.text,
                }
            })
            .collect()
    }

    /// The concrete syntax tree of a buffer.
    fn cst_of(&mut self, path: &str) -> Result<SyntaxNode, JsValue> {
        Ok(self.parse_of(path)?.syntax())
    }

    /// The typed view over the same tree.
    fn ast_of(&mut self, path: &str) -> Result<Option<ModuleRoot>, JsValue> {
        Ok(self.parse_of(path)?.module_root())
    }

    /// The HIR of the module in the buffer.
    fn hir_of(&mut self, path: &str) -> Result<Option<Hir>, JsValue> {
        let file = self.file(path)?;

        Ok(self.driver.lower(file).map(|lowered| Hir::of(&lowered)))
    }

    /// What checking the types of the buffer left.
    fn types_of(&mut self, path: &str) -> Result<Option<Types>, JsValue> {
        let file = self.file(path)?;

        let Some(lowered) = self.driver.lower(file) else {
            return Ok(None);
        };

        Ok(Types::of(&mut self.driver, ModuleId(file), &lowered))
    }

    /// What the stages of the pipeline reported.
    fn diagnostics_of(&mut self, path: &str) -> Result<Vec<Diagnostic>, JsValue> {
        let file = self.file(path)?;
        let diagnostics = self
            .driver
            .diagnostics(file)
            .ok_or_else(|| failure(&format!("{path} has no diagnostics to read")))?;
        let index = self
            .driver
            .line_index(file)
            .ok_or_else(|| failure(&format!("{path} has no lines to read")))?;

        Ok(diagnostics
            .iter()
            .map(|it| Diagnostic::of(it, &index))
            .collect())
    }

    /// The parse of a buffer, or a thrown error when the driver holds no text for it.
    fn parse_of(&mut self, path: &str) -> Result<Arc<Parse>, JsValue> {
        let file = self.file(path)?;

        self.driver
            .parse(file)
            .ok_or_else(|| failure(&format!("{path} is not text the driver can parse")))
    }

    /// The id the driver knows a path under, or a thrown error when it holds nothing for it.
    fn file(&self, path: &str) -> Result<FileId, JsValue> {
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

/// One file of the standard library, as a host reads it.
#[derive(Serialize)]
struct StdFile {
    /// The path the driver knows the file by, which is the one a host shows.
    path: String,

    /// The source of it.
    text: &'static str,
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

/// What checking the types of a module left, as a host reads it.
///
/// Types are values of the compiler, and what a host shows of them is what a person reads at
/// a place: the type an entity of the module was resolved to, and the type every node of every
/// body was checked to.
#[derive(Serialize)]
struct Types {
    /// The types of the entities of the module, in the order it declares them.
    surface: Vec<SurfaceType>,

    /// The checked bodies, in the order the module declares them.
    bodies: Vec<BodyTypes>,
}

/// What the driver did since a host last read the counters, as it reads it.
///
/// The counters are the driver's account of its own incrementality: what it handed over from
/// a slot, what it had to read again, what it kept although it read again, what went, and how
/// long the passes it ran took. A row is of one pass and one unit, and a host that wants the
/// totals of a pass --- or of a pull --- adds the rows up.
#[derive(Serialize)]
struct Stats {
    /// The counters of every pass and unit, in the order a module is read in.
    rows: Vec<StatsRow>,
}

impl Stats {
    /// The counters the driver holds, and a clean slate for the next read of them.
    fn of(driver: &mut mlkc_driver::Driver) -> Self {
        let taken = driver.take_stats();

        Self {
            rows: taken
                .iter()
                .map(|(pass, unit, tally)| {
                    StatsRow {
                        pass: pass.name().to_owned(),
                        unit: driver.unit_name(unit),
                        hits: tally.hits,
                        misses: tally.misses,
                        stales: tally.stales,
                        kept: tally.kept,
                        dropped: tally.dropped,
                        took: tally.took.as_secs_f64() * 1000.0,
                    }
                })
                .collect(),
        }
    }
}

/// What one pass did for one unit since the counters were last read.
#[derive(Serialize)]
struct StatsRow {
    /// The pass, by the name it is known by: `parse`, `interface`, `check`.
    pass: String,

    /// The unit the pass was asked for: the path of a file or a module, the name of a project,
    /// or the name of a body and where it is written.
    unit: String,

    /// How often the value was there: the slot was keyed by what it was built from.
    hits: u32,

    /// How often the pass ran and the driver held nothing to hand over.
    misses: u32,

    /// How often the pass ran although a value was held, because what the value was built from
    /// had changed.
    stales: u32,

    /// How often a pass that ran read the same as the value the driver held, which is the value
    /// it kept.
    kept: u32,

    /// How many values went, with the input they were built from.
    dropped: u32,

    /// How long the pass spent running for this unit, in milliseconds: zero for a pass that
    /// never ran, and zero for a host that gave the driver no clock.
    took: f64,
}

/// One entity of the surface of a module and the type it was resolved to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SurfaceType {
    /// What the entity is: `fun main`, `type Point`.
    name: String,

    /// The type it was resolved to, as it reads: `() -> Int`.
    ty: String,

    /// Where the declaration is written, in bytes, or nothing where it is written nowhere.
    range: Option<[u32; 2]>,
}

/// The types of the nodes of one body.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BodyTypes {
    /// The entity the body belongs to: `fun main`.
    owner: String,

    /// Where the declaration that owns the body is written, in bytes.
    range: Option<[u32; 2]>,

    /// The types of the nodes of the body, in the order they are written in.
    nodes: Vec<TypedNode>,
}

/// One node of a body and the type it was checked to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TypedNode {
    /// What the node is: `expr` or `pat`.
    kind: &'static str,

    /// What a node that is written nowhere is called by: `expr #3`.
    label: String,

    /// The part of the buffer the node covers, in bytes, or nothing when nothing was written.
    range: Option<[u32; 2]>,

    /// The type, as it reads: `Int`, `(Int) -> Bool`, `{error}`.
    ty: String,

    /// Whether the type is the type of a mistake, which a host paints as one.
    error: bool,
}

impl Types {
    /// Reads the types of a module and of its bodies the way a host reads them.
    ///
    /// The driver is what resolved the surface and checked the bodies; what the reading adds is
    /// where each of them was written, which is what the HIR of the module holds --- the range
    /// of an entity, and the source map of a body.
    fn of(driver: &mut Driver, module: ModuleId, lowered: &Lowered) -> Option<Self> {
        let types = driver.module_types(module)?;
        let mut surface = Vec::new();

        for (entity, ty) in types.iter() {
            surface.push(SurfaceType {
                name: entity_name(&entity.item),
                ty: ty.to_string(),
                range: lowered.item_range(&entity.item).map(covered),
            });
        }

        let mut bodies = Vec::new();

        for body in lowered.bodies() {
            let Some(checked) = driver.check(body.owner()) else {
                continue;
            };

            let places = &body.body().source_map;
            let mut nodes: Vec<(u32, u32, TypedNode)> = Vec::new();

            for (id, ty) in checked.expr_types() {
                nodes.push(node("expr", id.into_raw().into_u32(), places.expr(id), ty));
            }

            for (id, ty) in checked.pat_types() {
                nodes.push(node("pat", id.into_raw().into_u32(), places.pat(id), ty));
            }

            // The nodes are read in the order they are written in: a node nothing was read
            // from has no place, and is read after the ones that have one.
            nodes.sort_by_key(|(at, id, _)| (*at, *id));

            let owner = ItemLoc::from(body.owner().item.clone());

            bodies.push(BodyTypes {
                owner: entity_name(&owner),
                range: lowered.item_range(&owner).map(covered),
                nodes: nodes.into_iter().map(|(_, _, node)| node).collect(),
            });
        }

        Some(Self { surface, bodies })
    }
}

/// One node of a body, as a host reads it.
///
/// `at` is where the node is written, and the two numbers a host sorts it by are its place in
/// the buffer --- or the end of it, for a node nothing was read from --- and the position of
/// the node in the body of the compiler, which tells two nodes in one place apart.
fn node(kind: &'static str, id: u32, range: Option<TextRange>, ty: &Ty) -> (u32, u32, TypedNode) {
    let at = range.map_or(u32::MAX, |range| u32::from(range.start()));

    (at, id, TypedNode {
        kind,
        label: format!("{kind} #{}", id - 1),
        range: range.map(covered),
        ty: ty.to_string(),
        error: ty.is_error(),
    })
}

/// The name of an entity of the surface of a module, as a host reads it: `fun main`.
///
/// The name is the one the entity was declared under, and the kind is what the language
/// declares it with. An entity with no name of its own is read by its place among its kind,
/// which is what the HIR dump reads it by as well.
fn entity_name(item: &ItemLoc) -> String {
    match item.name() {
        Some(name) => format!("{} {name}", item.kind().keyword()),
        None => format!("{item:?}"),
    }
}

/// A diagnostic as an editor reads it: what to say, and where to point.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    /// `error`, `warning`, `note`, or `help`.
    level: String,

    /// `parser` for everything the parser reports.
    category: String,

    /// The two digits of the category, in the order the pipeline runs its stages: `02` for the
    /// parser. A code a person reads is these digits and then the kind's.
    category_code: String,

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
            category_code: diagnostic.category.as_code().to_string(),
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

    /// What the label says: the part of the message that belongs to the place it marks.
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
    fn the_library_crosses_the_boundary_as_the_files_it_is() {
        let mut driver = WasmDriver::new();
        let files = driver.register_library();
        let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();

        assert_eq!(paths, ["/std/core.mlk", "/std/prelude.mlk"]);
        assert!(
            files[0].text.contains("module project::core"),
            "a host is handed the source, not a path to read"
        );

        // The driver holds them, so a host that shows a file of the library gets its trees.
        for file in files {
            let diagnostics = driver
                .diagnostics_of(&file.path)
                .expect("a file of the library to analyze");

            assert!(
                diagnostics.is_empty(),
                "{}: {}",
                file.path,
                serde_json::to_string(&diagnostics).unwrap_or_default(),
            );
        }
    }

    #[test]
    fn a_pushed_buffer_crosses_the_boundary_as_trees_and_diagnostics() {
        let mut driver = WasmDriver::new();

        // The names of the language are names the library declares: without it, `Unit` is a
        // name the module cannot resolve.
        driver.register_library();

        assert!(driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    let x = 1 in\n    x\n".to_string())
        ));
        let cst = serde_json::to_value(driver.cst_of("/main.mlk").expect("the file to parse"))
            .expect("the tree to serialize");

        assert_eq!(cst["kind"], "MODULE_ROOT");
        assert!(
            cst["text_range"].is_array(),
            "a host reads the range of a node as a pair of offsets"
        );
        assert!(cst["children"].is_array(), "a host walks the children");
        assert_eq!(
            token_text(&cst, "FUN_KW").as_deref(),
            Some("fun "),
            "a token carries the trivia that follows it"
        );
        assert_eq!(token_text(&cst, "IDENT").as_deref(), Some("main"));
        assert!(
            token_text(&cst, "WHITESPACE").is_none(),
            "trivia is not a child: it belongs to the token it follows"
        );

        let ast = serde_json::to_value(driver.ast_of("/main.mlk").expect("the file to parse"))
            .expect("the tree to serialize");
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
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");

        assert!(
            diagnostics.is_empty(),
            "the module is one the language accepts: {}",
            serde_json::to_string(&diagnostics).unwrap_or_default(),
        );
    }

    #[test]
    fn what_the_driver_did_crosses_the_boundary_as_counters() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    let x = 1 in\n    x\n".to_string()),
        );

        let _ = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let taken = Stats::of(&mut driver.driver);

        // The first read of a buffer is work: nothing it needs is held yet. The hits are the
        // consultations of the values the read made and asked for again --- the library is
        // walked more than once by one pull --- and what a first read cannot have is a stale
        // read or a drop.
        let parse = row(&taken, "parse", "/main.mlk");

        assert!(parse.misses > 0, "the parse of a new buffer to be a miss");
        assert_eq!(parse.took, 0.0, "no clock was given: nothing is timed");
        assert!(
            taken
                .rows
                .iter()
                .all(|it| it.stales == 0 && it.dropped == 0),
            "a first read of a buffer to read nothing again",
        );

        // The second read of it is not: what came back is what the driver held.
        let _ = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let taken = Stats::of(&mut driver.driver);
        let parse = row(&taken, "parse", "/main.mlk");

        assert_eq!(parse.misses, 0);
        assert!(parse.hits > 0, "a second read of a buffer to be a hit");
        assert!(
            taken
                .rows
                .iter()
                .all(|it| it.stales == 0 && it.dropped == 0),
            "a second read of a buffer to read nothing again",
        );

        // The check of the buffer is a value of a body, and the row says whose: the name of
        // the body and the module it is written in.
        let checked: Vec<&str> = taken
            .rows
            .iter()
            .filter(|it| it.pass == "check")
            .map(|it| it.unit.as_str())
            .collect();

        assert_eq!(checked, ["/main.mlk: main"]);
    }

    #[test]
    fn a_pass_the_driver_ran_crosses_the_boundary_timed() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    let x = 1 in\n    x\n".to_string()),
        );

        // A host that has a clock of its own hands it over, and what the passes it runs cost is
        // read off it: a parse is not so fast that a machine's clock reports it as nothing.
        driver.driver.set_clock(mlkc_driver::system_clock());

        let _ = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let taken = Stats::of(&mut driver.driver);
        let parse = row(&taken, "parse", "/main.mlk");

        assert!(parse.misses > 0, "the parse of a new buffer to be a miss");
        assert!(
            parse.took > 0.0,
            "the time of the parse to cross the boundary in milliseconds, not {}",
            parse.took,
        );
    }

    /// The counters of one pass for one unit, by the names a host reads them by.
    fn row<'a>(stats: &'a Stats, pass: &str, unit: &str) -> &'a StatsRow {
        stats
            .rows
            .iter()
            .find(|it| it.pass == pass && it.unit == unit)
            .unwrap_or_else(|| panic!("a row for {pass} of {unit}"))
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
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let nodes = json["nodes"].as_array().expect("the HIR to hold lines");

        // The module is read first, and it is a line about nothing a host can mark: the file
        // stands in no project, and the module is called by the name of its file.
        assert_eq!(nodes[0]["text"], "MODULE #0 project::main");
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
        assert_eq!(
            item["children"][1]["text"], "ret: Unit -> use Unit",
            "the type of the result is a name the prelude brings in"
        );
        assert!(
            item["children"][0]["range"].is_null(),
            "a field of a declaration is about nothing a host marks"
        );

        // And then the names the module is given without writing them: a prelude import is
        // read like any other entity, and being written nowhere, it is a line with no range.
        let prelude = &nodes[1]["children"][2];

        assert_eq!(prelude["text"], "use Unit  prelude");
        assert_eq!(prelude["kind"], "item");
        assert!(prelude["range"].is_null());
        assert_eq!(prelude["children"][0]["text"], "path: std::prelude::Unit");

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
    fn a_name_written_twice_marks_each_place_it_is_written_at() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "fun main(): Int =\n    let x = 42 in\n    x + x\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let nodes = json["nodes"].as_array().expect("the HIR to hold lines");

        let declaration = &nodes[2]["children"][0]["children"][0];

        assert_eq!(declaration["text"], "expr#4  let pat#0 = expr#0 in expr#3");

        let sum = &declaration["children"][2];

        assert_eq!(sum["text"], "expr#3  binary expr#1 + expr#2");

        // Both names are one path of the body — the body holds the path once — and the line under
        // each of them marks the name it is written as: what a reader points at is the place
        // they are reading, and not the place the path happens to be written first.
        let left = &sum["children"][0]["children"][0];
        let right = &sum["children"][1]["children"][0];

        assert_eq!(left["text"], "path#0  x -> binding pat#0");
        assert_eq!(right["text"], left["text"]);
        assert_eq!(left["range"], sum["children"][0]["range"]);
        assert_eq!(right["range"], sum["children"][1]["range"]);
        assert_eq!(covered(source, &left["range"]), "x");
        assert_eq!(
            right["range"][0].as_u64().unwrap() as usize,
            source.rfind('x').expect("a second name in the sum"),
            "the second name is marked where it is written"
        );
        // And both of them name the binding the `let` introduced.
        assert_eq!(covered(source, &left["resolves"]), "x");
        assert_eq!(left["resolves"], right["resolves"]);
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

        let source = "use std::core::Int\n\nfun main(value: Int): Int =\n    value\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let items = json["nodes"][1]["children"]
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
            "use std::core::Int",
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

        let source = "use std::core::Int\n\nfun main(): Int =\n    Int\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let body = &json["nodes"][2];
        let path = &body["children"][0]["children"][0]["children"][0];

        assert_eq!(path["text"], "path#0  Int -> use Int");
        assert_eq!(covered(source, &path["range"]), "Int");
        assert_eq!(
            covered(source, &path["resolves"]),
            "use std::core::Int",
            "a name an import brought in leads to the import"
        );
    }

    #[test]
    fn a_lowering_mistake_crosses_the_boundary_as_a_diagnostic() {
        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text(
            "/main.mlk",
            Some(
                "fun f(): Unit =\n    g()\n\nfun g(): Unit =\n    g()\n\nfun f(): Unit =\n    g()\n"
                    .to_string(),
            ),
        );
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let diagnostics = serde_json::to_value(&diagnostics).expect("the diagnostics to serialize");
        let diagnostics = diagnostics.as_array().expect("diagnostics");

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "lowering");
        assert_eq!(diagnostics[0]["code"], "01");
        assert_eq!(diagnostics[0]["level"], "error");
    }

    #[test]
    fn a_type_mistake_crosses_the_boundary_as_a_diagnostic() {
        const SOURCE: &str = "fun main(): Unit =\n    \"text\"\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let json = serde_json::to_value(&diagnostics).expect("the diagnostics to serialize");
        let diagnostics = json.as_array().expect("diagnostics");

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "typechecker");
        assert_eq!(
            diagnostics[0]["categoryCode"], "05",
            "the type checker is the fifth stage of the pipeline"
        );
        assert_eq!(diagnostics[0]["code"], "09");

        // The place of the label is the expression the check reported, and what it says is what
        // an editor shows under the caret.
        let label = &diagnostics[0]["labels"][0];
        let start = SOURCE.find("\"text\"").expect("the literal to be written") as u64;

        assert_eq!(label["primary"], true);
        assert_eq!(label["start"], start);
        assert_eq!(label["end"], start + "\"text\"".len() as u64);
        assert_eq!(label["message"], "expected `Unit`, found `String`");
    }

    #[test]
    fn a_literal_out_of_the_range_of_int_crosses_the_boundary_as_a_mistake() {
        const SOURCE: &str = "fun big(): Int =\n    1099511627776\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let json = serde_json::to_value(&diagnostics).expect("the diagnostics to serialize");
        let diagnostics = json.as_array().expect("diagnostics");

        // The range of a literal is the meaning of the type it is written with, so the check is
        // what reports it ([ADR-0018]).
        //
        // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "typechecker");
        assert_eq!(diagnostics[0]["categoryCode"], "05");
        assert_eq!(diagnostics[0]["code"], "13");
        assert_eq!(
            diagnostics[0]["message"],
            "the integer literal `1099511627776` is outside the 31-bit range of `Int`"
        );

        // The place of the label is the literal, and what it says is what an editor shows under
        // the caret.
        let label = &diagnostics[0]["labels"][0];
        let start = SOURCE
            .find("1099511627776")
            .expect("the literal to be written") as u64;

        assert_eq!(label["primary"], true);
        assert_eq!(label["start"], start);
        assert_eq!(label["end"], start + "1099511627776".len() as u64);
        assert_eq!(label["message"], "outside the range of `Int`");
    }

    #[test]
    fn the_types_of_a_buffer_cross_the_boundary_as_places_and_types() {
        const SOURCE: &str = "fun main(): Int =\n    let x = 1 in\n    x\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let types = driver
            .types_of("/main.mlk")
            .expect("the file to be checked")
            .expect("the module to have types");
        let json = serde_json::to_value(&types).expect("the types to serialize");
        let types = &json;

        // The surface of the module: what its readers read of it.
        assert_eq!(types["surface"][0]["name"], "fun main");
        assert_eq!(types["surface"][0]["ty"], "() -> Int");

        let body = &types["bodies"][0];
        assert_eq!(body["owner"], "fun main");
        let (from, to) = range(body);
        assert_eq!(
            &SOURCE[from..to],
            SOURCE.trim_end(),
            "the declaration of the body is the whole function it was read from",
        );

        // Every node of the body, in the order it is written in: the `let`, the pattern it
        // binds, the value, and the name the body gives back.
        let nodes = body["nodes"].as_array().expect("the nodes of the body");
        let rows: Vec<(&str, &str, &str)> = nodes
            .iter()
            .map(|it| {
                let (from, to) = range(it);

                (
                    it["kind"].as_str().expect("a kind"),
                    it["ty"].as_str().expect("a type"),
                    &SOURCE[from..to],
                )
            })
            .collect();

        assert_eq!(rows.len(), 4, "{rows:?}");
        assert!(rows.iter().all(|it| it.1 == "Int"), "{rows:?}");
        assert_eq!(rows[0].0, "expr");
        assert!(rows[0].2.starts_with("let x = 1"), "{:?}", rows[0]);
        assert_eq!(rows[1].0, "pat");
        assert_eq!(rows[3], ("expr", "Int", "x"));
    }

    /// The place a serialized row points at, in the bytes a host counts.
    fn range(row: &serde_json::Value) -> (usize, usize) {
        let at = row["range"].as_array().expect("a range to be a pair");

        (
            at[0].as_u64().expect("a start") as usize,
            at[1].as_u64().expect("an end") as usize,
        )
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
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let diagnostic = diagnostics
            .into_iter()
            .next()
            .expect("the parse to report something");

        assert_eq!(diagnostic.level, "error");
        assert_eq!(diagnostic.category, "parser");
        assert_eq!(
            diagnostic.category_code, "02",
            "the parser is the second stage of the pipeline"
        );
        assert_eq!(diagnostic.code, "01");

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
        let ast = driver
            .ast_of("/main.mlk")
            .expect("the file to parse")
            .expect("the module root to be found");
        let json = serde_json::to_value(&ast).expect("the tree to serialize");
        let bogus = bogus_node(&json).expect("the mistake to leave a node of no kind");

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
