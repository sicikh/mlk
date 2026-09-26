//! The driver of the compiler.
//!
//! The driver is the component that owns the inputs and the memoized passes,
//! and the only one that decides what has to be recomputed.
//! The pipeline it drives is the compiler pipeline;
//! today that pipeline is the parser and the lowering of a module into the HIR,
//! so the table holds the values that are derived from the text of a file:
//! the parse, the HIR, the diagnostics a host renders, and the line index positions are read
//! with. The units that follow — the interface, the checked bodies —
//! are more slots in the same table rather than a different design.
//!
//! Three rules are visible in the code.
//!
//! - **Inputs are pushed, values are pulled**.
//!   The driver never opens a file, never reads a clock, never writes anywhere:
//!   a host hands it bytes, and asks it for values.
//! - **The key of a slot is the identity of what the pass read**.
//!   Everything here is a function of the text of one file --- and, for the HIR, of the
//!   prelude the file is compiled with --- so the identity of that text, its [`FileVersion`],
//!   is almost the whole key, while the text itself is handed to the pass by reference.
//! - **A pass is a function of its input**.
//!   [`Parse`] is what [`mlkc_parser::parse`] returned, and nothing more:
//!   the value and the diagnostics of the pass, stored as they came.
//!
//! # The prelude, and the projects it belongs to
//!
//! Every module is compiled with the prelude of its project: the imports the module is given
//! without writing them, which lowering declares into the module's item tree
//! ([ADR-0011](../../docs/adr/0011-module-prelude.md)). A prelude belongs to a project ---
//! `std` and a project of a host each have their own --- and the driver holds the module
//! graph ([`ProjectGraph`]): which projects exist, what each of them starts from and depends
//! on, and which project a module belongs to.
//!
//! A host records them with [`Driver::set_project`], [`Driver::set_module_project`], and
//! [`Driver::remove_project`], and a module that belongs to no project --- a file a host
//! pushed on its own, which no manifest claimed --- is compiled with the prelude of the
//! language. What a project says decides what its modules' text means, so the HIR of a module
//! is dropped when the module changes project or the project's prelude changes; a parse is
//! not, because a parse does not read the prelude.
//!
//! # The green nodes of a parse
//!
//! A parse of a file is built through a table of green nodes, and the table it is built
//! through is the one its own parse before it left ([`ParseSlot`]): the tokens that did not
//! move are what the tree of a new revision shares with the tree of the old one, and a file
//! shares them however many other files were parsed in between.
//!
//! One table for the whole project would not: a table holds the nodes of the parse it was
//! last used for, so it shares a file with the file parsed just before it and with nothing
//! else, and every parse of every file takes it mutably, which is a project parsed one file
//! at a time.
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
//!
//! The driver moves between threads as well, and so do the trees it handed out:
//! everything it owns is `Send`, and a tree is `Sync` besides,
//! which is what a host that owns the driver on a thread of its own relies on.
//! The assertion at the end of this file is what keeps that true.

use std::sync::Arc;

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::{
    BodyEntityLoc, ItemLoc, ItemTree, ModuleId, Prelude, ProjectData, ProjectGraph, ProjectId,
    dump::TypePlace,
};
use mlkc_line_index::LineIndex;
use mlkc_lower::{LoweredBody, LoweringDiag, lower_body, lower_module, syntax_at};
use mlkc_parser_core::{AnyParse, diagnostic::ParseDiagnostic};
use mlkc_rowan::{AstNode, NodeCache};
use mlkc_syntax::{AnyParameter, FunDecl, ModuleRoot, SyntaxNode, TextRange};
use mlkc_vfs::{ChangedFile, FileId, FileState, FileVersion, Vfs, VfsPath};
use rustc_hash::FxHashMap;

/// The driver: the only mutable component, and the owner of the memo table.
#[derive(Default)]
pub struct Driver {
    /// The state of every file a host has pushed.
    vfs: Vfs,
    /// The projects the compiler knows, and the project each module belongs to.
    ///
    /// The graph is configuration rather than an input of a file: it is not pushed, it is set,
    /// and what a project says of its modules decides how their text is read.
    projects: ProjectGraph,
    /// The parse of every file that has been parsed, with the green nodes it was built through.
    parses: FxHashMap<FileId, ParseSlot>,
    /// The HIR of every file that has been lowered.
    lowered: FxHashMap<FileId, TextSlot<Lowered>>,
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

/// The parse of one file, and the green nodes it was built through.
///
/// The nodes are kept next to the tree rather than in one table for the whole project. A
/// table holds the nodes of the parse it was last used for, so one table for a project shares
/// a file with the file parsed just before it, and nothing with its own earlier revisions: a
/// file that is parsed, then another, then parsed again begins from nothing. One table per
/// file is what makes a revision of a file share with the revision before it however many
/// other files were parsed in between, and what lets the files of a project be parsed at once.
struct ParseSlot {
    /// The version of the contents the tree was built from.
    version: FileVersion,
    /// The tree, retained so that the driver can hand it out and compare it later.
    value: Arc<Parse>,
    /// The green nodes of the parse, which the next parse of this file shares.
    cache: NodeCache,
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
    ///
    /// The tree is built through `cache`, which is the table of the file this parse is of:
    /// what the parse before it wrote is what this one shares.
    fn of(source: &str, cache: &mut NodeCache) -> Self {
        Self {
            parse: mlkc_parser::parse_with_cache(source, cache),
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

/// The HIR of one file, and where what it holds is written.
///
/// The HIR is the surface of a module, the bodies of the entities that own one, and the names
/// of the module; what it does not hold is a place in the file, and a host that marks a buffer
/// with a range needs one. The driver holds the HIR and the syntax it was lowered from, so it
/// is the one that says where a node of the HIR is written.
pub struct Lowered {
    /// The surface of the module: its entities, their names, and their data.
    item_tree: ItemTree,
    /// Where each entity of the surface is written, in the bytes of the file.
    items: FxHashMap<ItemLoc, TextRange>,
    /// Where the types the declarations of the module write are written, by the entity that
    /// writes them.
    types: FxHashMap<ItemLoc, TypePlaces>,
    /// The bodies of the module, in the order it declares them.
    bodies: Vec<ModuleBody>,
    /// What lowering reported, as a host renders it.
    diagnostics: Arc<[Diagnostic]>,
}

impl Lowered {
    /// Runs the passes that follow the parse.
    ///
    /// The HIR of a module is lowered in two steps, and this is both of them: the surface of
    /// the module, and then each body, from the declaration it is written in.
    fn of(module: ModuleId, root: &ModuleRoot, prelude: &Prelude) -> Self {
        let lowered = lower_module(module, root, prelude);

        // An entity the module did not write --- a prelude import --- is written nowhere, and
        // a host is given no range for it.
        let items = lowered
            .item_tree
            .entities()
            .filter_map(|(loc, id)| {
                let entity = lowered.item_tree.entity(id);

                Some((loc, syntax_at(root, entity.syntax()?)?.text_trimmed_range()))
            })
            .collect();

        let types = lowered
            .item_tree
            .entities()
            .filter_map(|(loc, id)| {
                let entity = lowered.item_tree.entity(id);
                let declaration = FunDecl::cast(syntax_at(root, entity.syntax()?)?)?;

                Some((loc, TypePlaces::of(&declaration)))
            })
            .collect();

        let bodies: Vec<ModuleBody> = lowered
            .bodies
            .iter()
            .filter_map(|decl| {
                let body = lower_body(&lowered.item_tree, &decl.decl)?;

                Some(ModuleBody {
                    owner: decl.owner.clone(),
                    body,
                })
            })
            .collect();

        // What the module says of its surface is reported before what its bodies say,
        // which is the order the module is read in.
        let diagnostics = lowered
            .diagnostics
            .iter()
            .chain(bodies.iter().flat_map(|it| it.body.diagnostics.iter()))
            .map(LoweringDiag::to_diagnostic)
            .collect::<Vec<_>>();

        Self {
            item_tree: lowered.item_tree,
            items,
            types,
            bodies,
            diagnostics: Arc::from(diagnostics),
        }
    }

    /// The surface of the module: its entities, their names, and their data.
    pub fn item_tree(&self) -> &ItemTree {
        &self.item_tree
    }

    /// Where the entity this name denotes is written, if the driver found where it is.
    ///
    /// A name is what crosses a revision, and a range is where it was written in this one:
    /// the two are the driver's to join, since the HIR holds the name and the syntax holds
    /// the place.
    pub fn item_range(&self, item: &ItemLoc) -> Option<TextRange> {
        self.items.get(item).copied()
    }

    /// Where the type a declaration writes at this place is written, if it writes one there.
    ///
    /// A type is a value of the HIR rather than a node of it, and the declaration a host reads
    /// it in is what says where it is written: the type of the parameter at an index, or the
    /// type the declaration writes for its result.
    pub fn type_range(&self, item: &ItemLoc, place: TypePlace) -> Option<TextRange> {
        let places = self.types.get(item)?;

        match place {
            TypePlace::Parameter(index) => places.params.get(index).copied().flatten(),
            TypePlace::Result => places.result,
        }
    }

    /// The bodies of the module, in the order it declares them.
    pub fn bodies(&self) -> &[ModuleBody] {
        &self.bodies
    }

    /// What lowering reported, as a host renders it.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// One body of a module, and the entity that owns it.
pub struct ModuleBody {
    /// The entity that owns the body.
    owner: BodyEntityLoc,
    /// The body itself, with where its nodes are written.
    body: LoweredBody,
}

impl ModuleBody {
    /// The entity that owns the body.
    pub fn owner(&self) -> &BodyEntityLoc {
        &self.owner
    }

    /// The body: its expressions, its patterns, its paths, and where they are written.
    pub fn body(&self) -> &LoweredBody {
        &self.body
    }
}

/// Where the types of one declaration are written.
///
/// A signature is a value of the HIR, and where it was written is the declaration it was read
/// from: the type of a parameter is the annotation the parameter carries, and the result is
/// the type the declaration writes for it. A place the declaration writes no type at has none.
#[derive(Default)]
struct TypePlaces {
    /// The type of each parameter, by the index the declaration writes it at.
    params: Vec<Option<TextRange>>,
    /// The type the declaration writes for its result.
    result: Option<TextRange>,
}

impl TypePlaces {
    /// The places one declaration writes the types of its signature at.
    ///
    /// The i-th parameter of a signature is the i-th parameter of the declaration: a signature
    /// holds a parameter for every parameter the declaration wrote, the ones that broke
    /// included, which is what the lowering keeps the arity of a declaration for.
    fn of(declaration: &FunDecl) -> Self {
        let mut places = Self::default();

        if let Ok(parameters) = declaration.parameters() {
            places.params = parameters
                .items()
                .syntax()
                .children()
                .map(|node| {
                    let Some(AnyParameter::Parameter(parameter)) = AnyParameter::cast(node) else {
                        return None;
                    };

                    let ty = parameter.type_annotation()?.ty().ok()?;

                    Some(ty.syntax().text_trimmed_range())
                })
                .collect();
        }

        places.result = declaration
            .return_type_annotation()
            .and_then(|annotation| annotation.return_type().ok())
            .map(|ty| ty.syntax().text_trimmed_range());

        places
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

    /// Records a project: the module it starts from, what it depends on, and the prelude its
    /// modules are given without writing them.
    ///
    /// Returns whether the graph changed. A project says what its modules are read under, so
    /// a change to one drops the HIR of the modules that belong to it ([ADR-0008]) --- today
    /// the prelude is the part of a project that lowering reads; the parses are kept, since a
    /// parse is a function of the text and of nothing else.
    ///
    /// Recording a project records the module it starts from as a module of it; the rest of
    /// its modules are recorded with [`Driver::set_module_project`].
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    pub fn set_project(&mut self, project: ProjectId, data: ProjectData) -> bool {
        if self.projects.project(&project) == Some(&data) {
            return false;
        }

        self.projects.insert(project.clone(), data);
        self.invalidate_project(&project);

        true
    }

    /// Drops a project, returning what it was.
    ///
    /// The modules of the project belong to no project afterwards, so they are read with the
    /// prelude of the language again, and the HIR of them goes.
    pub fn remove_project(&mut self, project: &ProjectId) -> Option<ProjectData> {
        self.projects.project(project)?;
        self.invalidate_project(project);

        self.projects.remove(project)
    }

    /// Records which project a module belongs to, and returns whether the graph changed.
    ///
    /// A module is lowered with the prelude of its project, so a module that changes project
    /// drops the HIR that was read under the other one. A module may be recorded before the
    /// graph holds the project: it is read with the prelude of the language until it does.
    pub fn set_module_project(&mut self, module: ModuleId, project: ProjectId) -> bool {
        if self.projects.project_of(module) == Some(&project) {
            return false;
        }

        self.projects.set_module_project(module, project);
        self.invalidate_module(module);

        true
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

    /// The projects the compiler knows, and the project each module belongs to.
    pub fn project_graph(&self) -> &ProjectGraph {
        &self.projects
    }

    // Pulls: they may compute, and they never answer from an invalid slot.

    /// The parse of `file`, computed when the slot is missing or stale.
    ///
    /// `None` means the driver has no text for the file:
    /// it was never pushed, it is gone, or it is not text at all.
    /// [`Driver::file_state`] tells which of those it is.
    pub fn parse(&mut self, file: FileId) -> Option<Arc<Parse>> {
        let version = self.file_version(file);

        // There is nothing to back-date here: the parse is a function of the text,
        // and the tree of different text is a different tree.
        // The stages where a recomputation can end up equal to the retained value —
        // the item tree, the interface — are the ones that follow.
        if let Some(slot) = self.parses.get(&file)
            && slot.version == version
        {
            return Some(slot.value.clone());
        }

        let Some(text) = self.file_text(file) else {
            // There is no input left to describe, so the slot goes, and the green nodes of
            // this file go with it: nothing is going to be parsed the way it was.
            self.parses.remove(&file);
            return None;
        };

        // The table this parse is built through is the one the parse before it left: the
        // tokens that did not move are what the two revisions of this file share.
        let mut cache = self
            .parses
            .remove(&file)
            .map_or_else(NodeCache::default, |slot| slot.cache);
        let value = Arc::new(Parse::of(&text, &mut cache));

        self.parses.insert(file, ParseSlot {
            version,
            value: value.clone(),
            cache,
        });

        Some(value)
    }

    /// The HIR of `file`, computed when the slot is missing or stale.
    ///
    /// `None` means there is nothing to lower: the file has no text, it was never parsed,
    /// or the parse did not find a module in it.
    pub fn lower(&mut self, file: FileId) -> Option<Arc<Lowered>> {
        let Some(parse) = self.parse(file) else {
            self.lowered.remove(&file);
            return None;
        };

        let version = self.file_version(file);

        Self::text_derived(&mut self.lowered, file, version, || {
            let root = parse.module_root()?;
            let module = ModuleId(file);

            // What the module is read with is the prelude of its project, which is the
            // prelude of the language for a module no project claims.
            let prelude = self.projects.prelude_of(module);

            Some(Arc::new(Lowered::of(module, &root, prelude)))
        })
    }

    /// The diagnostics of `file`, in the shape a host renders: what the parser reported,
    /// and then what lowering reported about the HIR the parse became.
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

        let lowered = self.lower(file);
        let version = self.file_version(file);

        Self::text_derived(&mut self.diagnostics, file, version, || {
            let mut rendered = parse
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.to_diagnostic(file))
                .collect::<Vec<_>>();

            if let Some(lowered) = &lowered {
                rendered.extend(lowered.diagnostics().iter().cloned());
            }

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
    ///
    /// The parse is the one stage whose slot does not have this shape: it keeps the green
    /// nodes of the parse next to its value ([`ParseSlot`]), and that is what its own pull is
    /// written out for.
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

    /// Drops the HIR of every module that belongs to `project`.
    ///
    /// What a project says is what its modules are read under, and the prelude is the part of
    /// it that lowering reads: a module of another project, or one of no project, is not
    /// affected by a change to this one. The parses stay, since a parse reads no project.
    fn invalidate_project(&mut self, project: &ProjectId) {
        let projects = &self.projects;
        let belongs = |file: &FileId| projects.project_of(ModuleId(*file)) == Some(project);

        self.lowered.retain(|file, _| !belongs(file));
        self.diagnostics.retain(|file, _| !belongs(file));
    }

    /// Drops the HIR of one module, and the diagnostics derived from it.
    fn invalidate_module(&mut self, module: ModuleId) {
        self.lowered.remove(&module.0);
        self.diagnostics.remove(&module.0);
    }
}

/// The driver, and the trees it hands out, cross threads.
///
/// The check is here rather than in a test so that a change to what the driver owns --- a
/// value that holds a handle another thread cannot have --- is a change the build refuses.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<Driver>();
    assert_send::<Parse>();
    assert_sync::<Parse>();
};

#[cfg(test)]
mod tests {
    use mlkc_diagnostics::{Category, Level};
    use mlkc_hir_def::{EntityData, ItemLoc, ItemLocLike, Name, Namespace, PathAnchor, PlainPath};
    use mlkc_line_index::LineCol;
    use mlkc_rowan::{AstNodeList, Direction};
    use mlkc_syntax::{FUN_KW, SyntaxKind, SyntaxToken};
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

    /// The name of the entity `name` in the surface of a lowered module.
    fn item(lowered: &Lowered, name: &str) -> ItemLoc {
        lowered
            .item_tree()
            .entities()
            .map(|(loc, _)| loc)
            .find(|loc| loc.name() == Some(&Name::new(name)))
            .unwrap_or_else(|| panic!("the module to declare the name `{name}`"))
    }

    /// The text of the source a range covers, if there is a range.
    fn covered(source: &str, range: Option<TextRange>) -> Option<&str> {
        range.map(|it| &source[usize::from(it.start())..usize::from(it.end())])
    }

    #[test]
    fn the_hir_of_a_module_is_lowered_from_the_same_parse() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);

        let lowered = driver.lower(file).expect("the file to be lowered");

        // The module declares `main`, and the prelude of the language brings in two more names.
        assert_eq!(
            lowered.item_tree().scope().len(),
            3,
            "three names are declared"
        );
        assert_eq!(lowered.bodies().len(), 1, "one entity owns a body");

        // The HIR holds the name of an entity rather than a place in the file, and a host
        // that marks a buffer needs the place: the driver joins the two.
        let range = lowered
            .item_range(&item(&lowered, "main"))
            .expect("the function to be written");

        assert_eq!(
            &MODULE[usize::from(range.start())..usize::from(range.end())],
            "fun main(): Unit =\n    let x = 42 * 2 - 10 in\n    println-int(x + 20)"
        );
    }

    #[test]
    fn a_type_of_a_signature_is_written_where_the_declaration_writes_it() {
        let source = "fun f(value: Map[Int]): Int =\n    1\n";
        let (mut driver, file) = driver_with("main.mlk", source);
        let lowered = driver.lower(file).expect("the file to be lowered");
        let f = item(&lowered, "f");

        assert_eq!(
            covered(source, lowered.type_range(&f, TypePlace::Parameter(0))),
            Some("Map[Int]"),
            "a parameter is read as the type the declaration annotated it with"
        );
        assert_eq!(
            covered(source, lowered.type_range(&f, TypePlace::Result)),
            Some("Int")
        );
        assert_eq!(
            lowered.type_range(&f, TypePlace::Parameter(1)),
            None,
            "a function that writes one parameter has no type for a second"
        );
    }

    #[test]
    fn the_parameters_of_a_signature_are_counted_the_way_the_declaration_writes_them() {
        // A parameter the parser could not read is a place without a type in the signature,
        // so the parameter written after it keeps its own.
        let source = "fun f(1, value: Int): Unit = 1\n";
        let (mut driver, file) = driver_with("main.mlk", source);
        let lowered = driver.lower(file).expect("the file to be lowered");
        let f = item(&lowered, "f");

        assert_eq!(
            lowered.type_range(&f, TypePlace::Parameter(0)),
            None,
            "the parameter that broke has no type to point at"
        );
        assert_eq!(
            covered(source, lowered.type_range(&f, TypePlace::Parameter(1))),
            Some("Int")
        );
    }

    #[test]
    fn a_lowering_mistake_travels_with_the_diagnostics_of_the_file() {
        // One name declared twice is a mistake the parser has nothing to say about:
        // a module says it, and the HIR cannot hold it.
        let (mut driver, file) = driver_with(
            "main.mlk",
            "fun f(): Unit =\n    1\n\nfun f(): Unit =\n    2\n",
        );

        let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].category, Category::Lowering);
        assert_eq!(diagnostics[0].level, Level::Error);
        assert_eq!(diagnostics[0].code, "01");
    }

    #[test]
    fn a_lowered_module_is_the_value_the_slot_holds() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let first = driver.lower(file).expect("the file to be lowered");
        let second = driver.lower(file).expect("the file to be lowered");

        assert!(Arc::ptr_eq(&first, &second), "the slot was built twice");
    }

    #[test]
    fn the_hir_follows_the_text_and_the_old_one_keeps_its_own() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let before = driver.lower(file).expect("the file to be lowered");

        driver.set_file_text(path("main.mlk"), Some(BROKEN.to_string()));
        let after = driver.lower(file).expect("the file to be lowered");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not rebuilt");
        assert_eq!(before.item_tree().scope().len(), 3, "the old value stands");
        assert_eq!(
            after.item_tree().scope().len(),
            3,
            "a function without an `in` is still a function"
        );
    }

    /// The path a name of the module denotes, which is what an import brings in.
    fn import_path(lowered: &Lowered, name: &str) -> String {
        let tree = lowered.item_tree();
        let anchor = tree.scope().anchor(&Name::new(name), Namespace::Ty);
        let PathAnchor::Use(import) = anchor else {
            panic!("`{name}` to be an import: {anchor:?}");
        };

        match tree.entity_data(ItemLoc::Use(import)) {
            Some(EntityData::Use(data)) => data.path.to_string(),
            other => panic!("the import of `{name}` to be a use: {other:?}"),
        }
    }

    #[test]
    fn the_prelude_brings_the_names_of_the_language_into_a_module() {
        let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        let lowered = driver.lower(file).expect("the file to be lowered");

        assert_eq!(import_path(&lowered, "Int"), "std.prelude.Int");
        assert_eq!(import_path(&lowered, "Unit"), "std.prelude.Unit");

        // The import is written nowhere in the buffer, so a host is given no place to mark.
        let int = item(&lowered, "Int");
        assert_eq!(lowered.item_range(&int), None);
    }

    #[test]
    fn a_module_that_says_no_prelude_is_given_none() {
        let (mut driver, file) = driver_with(
            "main.mlk",
            "@no-prelude\nmodule project.main-module\n\nfun main(): Int =\n    1\n",
        );
        let lowered = driver.lower(file).expect("the file to be lowered");

        assert!(lowered.item_tree().attributes().no_prelude);
        assert_eq!(
            lowered.item_tree().scope().len(),
            1,
            "`main`, and nothing else"
        );
        assert_eq!(
            lowered
                .item_tree()
                .scope()
                .anchor(&Name::new("Int"), Namespace::Ty),
            PathAnchor::Unresolved,
        );
    }

    #[test]
    fn setting_the_prelude_of_a_project_changes_what_its_modules_are_lowered_to() {
        let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        let before = driver.lower(file).expect("the file to be lowered");
        let parse = driver.parse(file).expect("the file to be parsed");

        assert_eq!(import_path(&before, "Int"), "std.prelude.Int");

        // A prelude of the project replaces the one of the language.
        let data = ProjectData {
            prelude: prelude_of(&["project", "core", "Int"]),
            ..ProjectData::new(ModuleId(file))
        };

        assert!(driver.set_project(project(), data.clone()));
        assert!(
            !driver.set_project(project(), data),
            "the project did not change",
        );

        let after = driver.lower(file).expect("the file to be lowered");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
        assert_eq!(import_path(&after, "Int"), "project.core.Int");
        assert_eq!(
            after
                .item_tree()
                .scope()
                .anchor(&Name::new("Unit"), Namespace::Ty),
            PathAnchor::Unresolved,
            "the prelude of the project replaced the one of the language",
        );

        // The root module of a project is a module of it, and a parse is not what a project
        // changes: the one the driver already holds stands.
        assert_eq!(
            driver.project_graph().project_of(ModuleId(file)),
            Some(&project()),
        );
        let parsed_again = driver.parse(file).expect("the file to be parsed");
        assert!(Arc::ptr_eq(&parse, &parsed_again), "the parse was not kept");
    }

    #[test]
    fn a_project_changes_only_what_belongs_to_it() {
        let (mut driver, main) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        driver.set_file_text(
            path("lib.mlk"),
            Some("fun size(): Int =\n    1\n".to_string()),
        );
        let lib = driver
            .file_id(&path("lib.mlk"))
            .expect("the file to have an id");

        let first = ProjectId::new("first");
        let second = ProjectId::new("second");

        assert!(driver.set_project(first.clone(), ProjectData {
            prelude: prelude_of(&["project", "core", "Int"]),
            ..ProjectData::new(ModuleId(main))
        }));
        assert!(driver.set_project(second.clone(), ProjectData::new(ModuleId(lib))));

        let before_main = driver.lower(main).expect("the file to be lowered");
        let before_lib = driver.lower(lib).expect("the file to be lowered");

        assert_eq!(import_path(&before_main, "Int"), "project.core.Int");
        assert_eq!(import_path(&before_lib, "Int"), "std.prelude.Int");

        // The prelude of one project changes, and the module of the other is left alone.
        assert!(driver.set_project(first, ProjectData {
            prelude: prelude_of(&["project", "other", "Int"]),
            ..ProjectData::new(ModuleId(main))
        }));

        let after_main = driver.lower(main).expect("the file to be lowered");
        let after_lib = driver.lower(lib).expect("the file to be lowered");

        assert!(
            !Arc::ptr_eq(&before_main, &after_main),
            "the slot was not dropped"
        );
        assert!(Arc::ptr_eq(&before_lib, &after_lib), "the slot was dropped");
        assert_eq!(import_path(&after_main, "Int"), "project.other.Int");
    }

    #[test]
    fn a_module_that_changes_project_is_read_with_the_other_project() {
        let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        let project = ProjectId::new("the-project");

        assert!(driver.set_project(project.clone(), ProjectData {
            prelude: prelude_of(&["project", "core", "Int"]),
            ..ProjectData::new(ModuleId(file))
        }));

        // The root module of the project is recorded as one of it, so a module that changes
        // project has to be one the graph does not know yet.
        driver.set_file_text(
            path("lib.mlk"),
            Some("fun size(): Int =\n    1\n".to_string()),
        );
        let lib = driver
            .file_id(&path("lib.mlk"))
            .expect("the file to have an id");
        let before = driver.lower(lib).expect("the file to be lowered");

        assert_eq!(import_path(&before, "Int"), "std.prelude.Int");

        assert!(driver.set_module_project(ModuleId(lib), project.clone()));
        assert!(!driver.set_module_project(ModuleId(lib), project));

        let after = driver.lower(lib).expect("the file to be lowered");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
        assert_eq!(import_path(&after, "Int"), "project.core.Int");
    }

    #[test]
    fn dropping_a_project_returns_its_modules_to_the_prelude_of_the_language() {
        let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        let project = ProjectId::new("the-project");

        driver.set_project(project.clone(), ProjectData {
            prelude: Prelude::none(),
            ..ProjectData::new(ModuleId(file))
        });

        let before = driver.lower(file).expect("the file to be lowered");
        assert_eq!(
            before
                .item_tree()
                .scope()
                .anchor(&Name::new("Int"), Namespace::Ty),
            PathAnchor::Unresolved,
        );

        assert!(driver.remove_project(&project).is_some());
        assert!(driver.remove_project(&project).is_none());

        let after = driver.lower(file).expect("the file to be lowered");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
        assert_eq!(import_path(&after, "Int"), "std.prelude.Int");
        assert_eq!(driver.project_graph().project_of(ModuleId(file)), None);
    }

    /// A path in a prelude, as the paths of a project are written.
    fn prelude_of(path: &[&str]) -> Prelude {
        Prelude::from_paths([PlainPath::from_segments(
            path.iter().map(|segment| Name::new(segment)),
        )])
    }

    /// A project a test records.
    fn project() -> ProjectId {
        ProjectId::new("the-project")
    }

    /// The first token of a kind in a parse, which is what shows what the parses shared.
    fn first_token(parse: &Parse, kind: SyntaxKind) -> SyntaxToken {
        parse
            .syntax()
            .descendants_tokens(Direction::Next)
            .find(|token| token.kind() == kind)
            .expect("the tree to hold a token of that kind")
    }

    #[test]
    fn a_revision_of_a_file_shares_the_tokens_it_did_not_edit() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let before = driver.parse(file).expect("the file to be parsed");

        // A function written after the module: every token of the module is what it was.
        let text = format!("{MODULE}\nfun added(): Unit =\n    1\n");
        assert!(driver.set_file_text(path("main.mlk"), Some(text.clone())));

        let after = driver.parse(file).expect("the file to be parsed");

        assert_eq!(after.syntax().to_string(), text);
        assert!(
            first_token(&before, FUN_KW).key() == first_token(&after, FUN_KW).key(),
            "the parse was not built through the nodes the parse before it left"
        );
    }

    #[test]
    fn the_parses_of_two_files_share_no_nodes() {
        // The files are written the same way, and each is parsed through the nodes of its own
        // parses: what one file shares is with the revision before it, and not with another
        // file. That is the price of parsing the files of a project at once.
        let (mut driver, first) = driver_with("one.mlk", MODULE);
        driver.set_file_text(path("two.mlk"), Some(MODULE.to_string()));

        let second = driver
            .file_id(&path("two.mlk"))
            .expect("the file to have an id");

        let one = driver.parse(first).expect("the file to be parsed");
        let two = driver.parse(second).expect("the file to be parsed");

        assert!(first_token(&one, FUN_KW).key() != first_token(&two, FUN_KW).key());
    }

    #[test]
    fn the_driver_and_the_trees_it_hands_out_cross_threads() {
        let (mut driver, file) = driver_with("main.mlk", MODULE);
        let before = driver.parse(file).expect("the file to be parsed");

        // A tree is immutable and shared, so it is read anywhere: a host answers the requests
        // that only read from the thread that asked, and the parsing to the thread that owns
        // the driver.
        let text = std::thread::spawn({
            let before = before.clone();

            move || before.syntax().to_string()
        })
        .join()
        .expect("the thread not to panic");

        assert_eq!(text, MODULE);

        // The driver owns the green nodes of every file it parsed, and they move with it: a
        // host may hand it to another thread, which is what lets the files of a project be
        // parsed at once.
        let (mut driver, text) = std::thread::spawn(move || {
            let parse = driver.parse(file).expect("the file to be parsed");

            (driver, parse.syntax().to_string())
        })
        .join()
        .expect("the thread not to panic");

        assert_eq!(text, MODULE);

        // The driver comes back as it was: the parse of the file is the value it already was.
        let after = driver.parse(file).expect("the file to be parsed");

        assert!(Arc::ptr_eq(&before, &after), "the slot was built again");
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
