//! The driver of the compiler.
//!
//! The driver is the component that owns the inputs and the memoized passes,
//! and the only one that decides what has to be recomputed.
//! The pipeline it drives is the compiler pipeline;
//! today that pipeline is the parser, the lowering of a module into the HIR,
//! the resolution of a module against the interfaces of the modules its paths name,
//! the check of the types of the module and of each of its bodies,
//! and the lowering of every body that checks clean into MIR and into its SSA form,
//! so the table holds the values that are derived from the text of a file:
//! the parse, the HIR, the interface of the module it is,
//! the resolution of its names, the types of its entities, the check of every body,
//! the MIR of every body that checks clean, and the diagnostics of all of them,
//! and the line index positions are read with --- and, per project,
//! the module index and the def map.
//!
//! Three rules are visible in the code.
//!
//! - **Inputs are pushed, values are pulled**.
//!   The driver never opens a file, never reads a clock, never writes anywhere:
//!   a host hands it bytes, and asks it for values.
//! - **The key of a slot is the identity of what the pass read**.
//!   Everything here is a function of the text of one file --- and, for the HIR, of the prelude
//!   the file is compiled with and the projects it may name --- so the identity of that text,
//!   its [`FileVersion`], is almost the whole key, while the text itself is handed to the pass
//!   by reference.
//! - **A pass is a function of its input**.
//!   [`Parse`] is what [`mlkc_parser::parse`] returned, and nothing more:
//!   the value and the diagnostics of the pass, stored as they came.
//!
//! # The diagnostics of a file
//!
//! Every stage reports its own, and what a host reads is the [`Diagnostics`] of a file: what
//! the parser reported, what the lowering of the module reported, what the resolution of it
//! found, and what checking its types found, each of them the value the stage left. No stage's
//! diagnostics are copied into another stage's, and a mistake one stage reports is not a reason
//! to render what the stages around it reported again.
//!
//! The renderings that are the driver's are the ones of the stages that report places in the
//! HIR: a resolution reports an entity and the type among its types, a check reports an
//! expression, a pattern, or a type a declaration writes, and where a place is written is what
//! the driver holds. Such a rendering is a value of its own, keyed by what it read, and what a
//! resolution's rendering reads of another module is its HIR only for a name a walk could not
//! find ([`mlkc_resolve::hidden_name`]): a module that resolved cleanly reads no HIR beside its
//! own.
//!
//! The construction of MIR reports nothing: a body the driver hands it is one every stage
//! before it read clean, and one it cannot lower is a bug of the check ([ADR-0019]), not
//! something a host is told about.
//!
//! [ADR-0019]: ../../docs/adr/0019-mir.md
//!
//! # The types of a file
//!
//! The types of a module are resolved from the signatures it writes, and before the bodies of
//! it are checked: every top-level declaration writes its types for now ([ADR-0017]), so a
//! check never needs what another check found, and a body edit cannot change what the module
//! shows. A check of one body is a slot of its own --- the body is the unit --- and it reads the
//! surface of the module, the surfaces of the modules its paths reach, and the classes of the
//! language, and nothing of another body's check.
//!
//! The classes of the language --- `Int`, `Unit`, `String`, `Bool` --- are the ones the standard
//! library declares with `#[builtin]`: the driver reads them off `std::core` and hands them to
//! the check, and a driver whose host recorded no library checks no types.
//!
//! The diagnostics of a check are rendered here as well, the way the resolution's are: a check
//! reports places in the HIR --- an expression, a pattern, a type a declaration writes --- and
//! where a place is written is what the driver holds. The look a name a body path could not
//! find takes at the module it reached is the look the rendering of a resolution takes, and it
//! is recorded the same way.
//!
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md
//!
//! # The prelude, and the projects it belongs to
//!
//! Every module is compiled with the prelude of its project: the imports the module is given
//! without writing them, which lowering declares into the module's item tree
//! ([ADR-0011](../../docs/adr/0011-module-prelude.md)). A prelude belongs to a project ---
//! `std` and a project of a host each have their own --- and the driver holds the module
//! graph ([`ProjectGraph`]): which projects exist, what each of them depends on, and which
//! project a module belongs to.
//!
//! A host records them with [`Driver::set_project`], [`Driver::set_module_project`], and
//! [`Driver::remove_project`], and a module that belongs to no project --- a file a host
//! pushed on its own, which no manifest claimed --- is compiled with the prelude of the
//! language. What a project says decides what its modules' text means, so the HIR of a module
//! is dropped when the module changes project or the project's prelude changes; a parse is
//! not, because a parse does not read the prelude.
//!
//! A project is what a module may name, and the list of the ones it may name is what the
//! lowering of the module is handed: a project says what another project is called
//! ([`ProjectData::dependencies`]), those are the projects a path of a module of it is rooted
//! at, and a module of no project names every project of the graph, which is all there is to
//! declare a dependency on ([ADR-0016]).
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//!
//! The library the language's own prelude names is one such project, and it is the compiler's
//! rather than a host's ([`mlkc_stdlib`]): a host records it with [`Driver::use_std`], and a
//! host that shows it to a person --- the editor --- shows the files it was handed.
//!
//! A project is a set of modules rather than a tree of them: no module of a project is the one
//! it starts from, and a module is called by the place its file stands at, which the path a
//! host pushed it under says.
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

use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::{
    Body, BodyEntityLoc, ClassLoc, EntityData, EntityLoc, Interface, ItemLoc, ItemTree, ModuleId,
    ModuleIndex, Name, PathAnchor, Prelude, ProjectData, ProjectDefMap, ProjectGraph, ProjectId,
    dump::TypePlace as DeclaredType,
};
use mlkc_hir_ty::{CheckedBody, ModuleTypes};
use mlkc_line_index::LineIndex;
use mlkc_lower::{LoweredBody, LoweringDiag, lower_body, lower_module, syntax_at};
use mlkc_mir::Body as MirBody;
use mlkc_mir_build::{construct_ssa, lower_body as lower_mir};
use mlkc_parser_core::{AnyParse, diagnostic::ParseDiagnostic};
use mlkc_resolve::{
    Closure, Resolution, ResolveDeps, ResolveDiag, ResolveError, hidden_name, resolve_module,
};
use mlkc_rowan::{AstNode, NodeCache};
use mlkc_span::Span;
use mlkc_syntax::{AnyParameter, FunDecl, ModuleRoot, SyntaxNode, TextRange};
use mlkc_typeck::{
    Builtins, CheckDeps, TypeDiag, TypeError, TypePlace, check_body, resolve_module_types,
};
use mlkc_vfs::{ChangedFile, FileId, FileState, FileVersion, RelPath, RelPathBuf, Vfs, VfsPath};
use rustc_hash::FxHashMap;

/// One file of the standard library: the path a driver keeps a module under, and its source.
///
/// A host that records the library is handed these ([`Driver::use_std`]), and a host that shows
/// it to a person --- the editor --- shows the same files and writes in none of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdFile {
    /// The path the file is known by in a driver.
    pub path: VfsPath,

    /// The source of the file, as the compiler was built with it.
    pub text: &'static str,
}

/// The driver: the only mutable component, and the owner of the memo table.
#[derive(Default)]
pub struct Driver {
    /// The state of every file a host has pushed.
    vfs: Vfs,
    /// The projects the compiler knows, and the project each module belongs to.
    ///
    /// The graph is configuration rather than an input of a file: it is not pushed, it is set,
    /// and what a project says of its modules decides how their text is read. It is shared
    /// rather than owned so that a resolution can be handed the graph it was resolved against
    /// without copying it.
    projects: Arc<ProjectGraph>,
    /// The parse of every file that has been parsed, with the green nodes it was built through.
    parses: FxHashMap<FileId, ParseSlot>,
    /// The HIR of every file that has been lowered.
    lowered: FxHashMap<FileId, TextSlot<Lowered>>,
    /// The interface of every module that has been asked for one.
    interfaces: FxHashMap<ModuleId, InterfaceSlot>,
    /// The module index of every project that has been asked for one.
    module_indexes: FxHashMap<ProjectId, ModuleIndexSlot>,
    /// The resolution of every module that has been resolved.
    resolutions: FxHashMap<ModuleId, ResolutionSlot>,
    /// The types of every module whose signatures have been resolved.
    signatures: FxHashMap<ModuleId, SignaturesSlot>,
    /// The check of every body that has been checked.
    checks: FxHashMap<BodyEntityLoc, CheckSlot>,
    /// The MIR of every body that checks clean, in the CFG form.
    mirs: FxHashMap<BodyEntityLoc, MirSlot>,
    /// The SSA form of every body whose MIR has been asked for in it.
    ssas: FxHashMap<BodyEntityLoc, SsaSlot>,
    /// The def map of every project that has been asked for one.
    def_maps: FxHashMap<ProjectId, DefMapSlot>,
    /// The rendered diagnostics of the parse of every file that has been asked for them.
    parse_diagnostics: FxHashMap<FileId, TextSlot<[Diagnostic]>>,
    /// The rendered diagnostics of the resolution of every module that has been asked for them.
    resolution_diagnostics: FxHashMap<ModuleId, ResolutionDiagnosticsSlot>,
    /// The rendered diagnostics of the checks of every module that has been asked for them.
    type_diagnostics: FxHashMap<ModuleId, TypeDiagnosticsSlot>,
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

/// The interface of one module, and the HIR it was cut from ([ADR-0016]).
///
/// An interface is a function of the module's own text, which is the item tree the driver
/// holds: the slot is keyed by that value, so an edit that leaves the item tree equal --- one
/// inside a body, or one that touches a private name --- leaves the interface where it was.
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
struct InterfaceSlot {
    /// The HIR the interface was cut from.
    lowered: Arc<Lowered>,
    /// The value, retained so that a recomputation that ends up equal keeps it.
    value: Arc<Interface>,
}

/// The module index of one project, and the interfaces it was built from ([ADR-0016]).
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
struct ModuleIndexSlot {
    /// The interface of every module of the project, by module, as the index was built.
    interfaces: BTreeMap<ModuleId, Arc<Interface>>,
    /// The value, retained so that a recomputation that ends up equal keeps it.
    value: Arc<ModuleIndex>,
}

/// The resolution of one module, and what it was made from ([ADR-0016]).
///
/// The key of a resolution is what its walk read: the module's own HIR, the interfaces it
/// reached, and the entries it read of the module indexes. The two of the three that belong to
/// other units are what the [`Closure`] holds.
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
struct ResolutionSlot {
    /// The HIR the resolution was made from.
    lowered: Arc<Lowered>,
    /// The interfaces the walk reached, and the entries of the indexes it read.
    closure: Closure,
    /// The value, retained so that a recomputation that ends up equal keeps it.
    value: Arc<Resolution>,
}

/// The def map of one project, and the resolutions it was built from ([ADR-0016]).
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
struct DefMapSlot {
    /// The resolution of every module of the project, by module, as the map was built.
    resolutions: BTreeMap<ModuleId, Arc<Resolution>>,
    /// The value, retained so that a recomputation that ends up equal keeps it.
    value: Arc<ProjectDefMap>,
}

/// The rendered diagnostics of the resolution of one module, and what rendering them read
/// ([ADR-0016]).
///
/// A resolution reports places in the HIR, and where a place is written is what the driver
/// holds, so the rendering is the driver's. It is a value of its own, keyed by the resolution,
/// by the HIR it was made from, and by the HIR of the modules it looked at --- which is nothing
/// on the way a resolution usually goes, and a module per name a walk could not find
/// ([`hidden_name`]).
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
struct ResolutionDiagnosticsSlot {
    /// The resolution the diagnostics were rendered from.
    resolution: Arc<Resolution>,
    /// The HIR the places of the diagnostics are read in.
    lowered: Arc<Lowered>,
    /// The HIR of the modules a diagnostic looked at, by module.
    looked: BTreeMap<ModuleId, Arc<Lowered>>,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<[Diagnostic]>,
}

/// The types of one module's entities, and what resolving them found ([ADR-0017]).
///
/// The surface is resolved from the signatures the module writes and from nothing else, so the
/// key of the slot is the module's own item tree, its resolution, and the closure the written
/// types walk --- never a body ([ADR-0017]).
///
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
struct SignaturesSlot {
    /// The HIR the signatures are written in.
    lowered: Arc<Lowered>,
    /// What each name of the module denotes, and what each import resolved to.
    resolution: Arc<Resolution>,
    /// The modules the written types reach, and the entries of the indexes they read.
    closure: Closure,
    /// The value, retained so that the driver can hand it out and compare it later.
    signatures: Signatures,
}

/// The type surface of a module, and what resolving its written types reported.
#[derive(Debug, Clone)]
struct Signatures {
    /// The types of the module's entities, by the entity that declares them.
    value: Arc<ModuleTypes>,
    /// What resolving a written type found, in the order of the module.
    diagnostics: Arc<[TypeDiag]>,
}

impl Signatures {
    /// Whether this is the same value as `other`: the same surface, and the same report.
    fn reads_the_same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value) && Arc::ptr_eq(&self.diagnostics, &other.diagnostics)
    }
}

/// What the check of a body reads of the units around it, and what the lowering of the body
/// reads after it ([ADR-0017], [ADR-0019]).
///
/// The inputs are gathered before either slot is asked, so that a pull of the MIR of a body
/// does not walk the closure the check of it already walked.
///
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
/// [ADR-0019]: ../../docs/adr/0019-mir.md
struct CheckInputs {
    /// The HIR the body was read from.
    lowered: Arc<Lowered>,
    /// What each name of the module denotes, and what each import resolved to.
    resolution: Arc<Resolution>,
    /// The modules the check walks, and the entries of the indexes it read.
    closure: Closure,
    /// The type surface of every module the check may read, by module.
    types: BTreeMap<ModuleId, Arc<ModuleTypes>>,
    /// The classes of the language the check was given.
    builtins: Builtins,
}

/// The check of one body, and what it was made from ([ADR-0017]).
///
/// The key of a check is what the check read: the HIR of the module --- the body among it --- the
/// resolution of the module, the closure of the check, the type surfaces of the modules the
/// paths reach, and the classes of the language ([ADR-0017]).
///
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
struct CheckSlot {
    /// The HIR the body was read from.
    lowered: Arc<Lowered>,
    /// What each name of the module denotes, and what each import resolved to.
    resolution: Arc<Resolution>,
    /// The modules the check walks, and the entries of the indexes it read.
    closure: Closure,
    /// The type surface of every module the check may read, by module.
    types: BTreeMap<ModuleId, Arc<ModuleTypes>>,
    /// The classes of the language the check was given.
    builtins: Builtins,
    /// The value, retained so that the driver can hand it out and compare it later.
    checked: Checked,
}

/// The types of one checked body, and what checking it reported.
#[derive(Debug, Clone)]
struct Checked {
    /// The types of the nodes of the body, by the node inside it.
    value: Arc<CheckedBody>,
    /// What the check found, in the order of the body.
    diagnostics: Arc<[TypeDiag]>,
}

impl Checked {
    /// Whether this is the same value as `other`: the same types, and the same report.
    fn reads_the_same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value) && Arc::ptr_eq(&self.diagnostics, &other.diagnostics)
    }
}

/// The MIR of one body in the CFG form, and what it was made from ([ADR-0019]).
///
/// The key of the slot is what the lowering read: the HIR of the module --- the body among it ---
/// the resolution of the module, the closure the walk read, the classes of the language, and the
/// check of the body, whose types pick the operators and the constants. The surfaces the check
/// read are not read again here: the lowering reads the types of the check's value and no surface
/// of another module.
///
/// [ADR-0019]: ../../docs/adr/0019-mir.md
struct MirSlot {
    /// The HIR the body was read from.
    lowered: Arc<Lowered>,
    /// What each name of the module denotes, and what each import resolved to.
    resolution: Arc<Resolution>,
    /// The modules the walk read, and the entries of the indexes it read.
    closure: Closure,
    /// The classes of the language the lowering was given.
    builtins: Builtins,
    /// The check the lowering read.
    checked: Checked,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<MirBody>,
}

/// The SSA form of one body, and the CFG form it was computed from ([ADR-0019]).
///
/// [ADR-0019]: ../../docs/adr/0019-mir.md
struct SsaSlot {
    /// The CFG form the pass read.
    cfg: Arc<MirBody>,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<MirBody>,
}

/// The rendered diagnostics of the checks of one module, and what rendering them read
/// ([ADR-0017]).
///
/// A check reports places in the HIR --- an expression, a pattern, a type a declaration writes ---
/// and where a place is written is what the driver holds, so the rendering is the driver's. The
/// look a body path takes at a module a walk could not find ([`hidden_name`]) is the one the
/// rendering of a resolution takes, and it is recorded the same way.
///
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
struct TypeDiagnosticsSlot {
    /// The HIR the places of the diagnostics are read in.
    lowered: Arc<Lowered>,
    /// The signatures of the module, as they were rendered.
    signatures: Signatures,
    /// The check of every body of the module, in the order the module declares them.
    checks: Vec<Checked>,
    /// The HIR of the modules a diagnostic looked at, by module.
    looked: BTreeMap<ModuleId, Arc<Lowered>>,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<[Diagnostic]>,
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
    ///
    /// `relative` is where the file of the module stands, which is what the module is called
    /// when it declares no path of its own: see [`Driver::module_path`].
    fn of(
        module: ModuleId,
        root: &ModuleRoot,
        prelude: &Prelude,
        projects: &[ProjectId],
        relative: &RelPath,
    ) -> Self {
        let lowered = lower_module(module, root, prelude, projects, relative);

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
    pub fn type_range(&self, item: &ItemLoc, place: DeclaredType) -> Option<TextRange> {
        let places = self.types.get(item)?;

        match place {
            DeclaredType::Parameter(index) => places.params.get(index).copied().flatten(),
            DeclaredType::Result => places.result,
        }
    }

    /// The bodies of the module, in the order it declares them.
    pub fn bodies(&self) -> &[ModuleBody] {
        &self.bodies
    }

    /// What lowering reported, as a host renders it.
    pub fn diagnostics(&self) -> &Arc<[Diagnostic]> {
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
    ///
    /// The standard library of the language is the one thing a host does not have to know:
    /// a host asks for it ([`Driver::use_std`]) and is handed the files of the library the
    /// compiler was built with.
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

    /// Records a project: what it depends on, and the prelude its modules are given without
    /// writing them.
    ///
    /// Returns whether the graph changed. A project says what its modules are read under, so
    /// a change to one drops the HIR of the modules that belong to it ([ADR-0008]) --- today
    /// the prelude is the part of a project that lowering reads; the parses are kept, since a
    /// parse is a function of the text and of nothing else.
    ///
    /// A project holds no module it starts from: which modules are its own is recorded one by
    /// one with [`Driver::set_module_project`].
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    pub fn set_project(&mut self, project: ProjectId, data: ProjectData) -> bool {
        if self.projects.project(&project) == Some(&data) {
            return false;
        }

        Arc::make_mut(&mut self.projects).insert(project.clone(), data);
        self.invalidate_project(&project);

        true
    }

    /// Records the standard library of the language: pushes its sources, and records the
    /// project they are.
    ///
    /// The library is part of the compiler ([`mlkc_stdlib`]) and not of a host, so nothing here
    /// is a host's to say: where the modules land, which project they are, and the imports the
    /// library gives its own modules are all decided by the compiler.
    ///
    /// A host says *when* the library goes in, and a driver whose host never calls this holds no
    /// library: the tests of this crate want that, and so does a host that compiles against
    /// another library, which records its own ([`Driver::set_project`]).
    ///
    /// Returns the files the library is made of, in the order of [`mlkc_stdlib::modules`]:
    /// the path each was recorded at, and its source, which is what a host shows. The library is
    /// the compiler's rather than a person's, and nothing in it is a person's to write.
    pub fn use_std(&mut self) -> Vec<StdFile> {
        let files: Vec<StdFile> = mlkc_stdlib::modules()
            .iter()
            .map(|module| {
                StdFile {
                    path: mlkc_stdlib::path(module),
                    text: module.source,
                }
            })
            .collect();

        for file in &files {
            self.set_file_text(file.path.clone(), Some(file.text.to_owned()));
        }

        let id = ProjectId::new(mlkc_stdlib::PROJECT);

        self.set_project(id.clone(), mlkc_stdlib::project());

        // Every module of the library is one of the project: a project holds no module it
        // starts from, so each of them is recorded.
        for file in &files {
            let module = ModuleId(
                self.file_id(&file.path)
                    .expect("a file of the library to have an id once it is pushed"),
            );

            self.set_module_project(module, id.clone());
        }

        files
    }

    /// Drops a project, returning what it was.
    ///
    /// The modules of the project belong to no project afterwards, so they are read with the
    /// prelude of the language again, and the HIR of them goes.
    pub fn remove_project(&mut self, project: &ProjectId) -> Option<ProjectData> {
        self.projects.project(project)?;
        self.invalidate_project(project);

        Arc::make_mut(&mut self.projects).remove(project)
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

        Arc::make_mut(&mut self.projects).set_module_project(module, project);
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
    /// the parse did not find a module in it, or the place of the file names no module.
    pub fn lower(&mut self, file: FileId) -> Option<Arc<Lowered>> {
        let Some(parse) = self.parse(file) else {
            self.lowered.remove(&file);
            return None;
        };

        // Where the module stands, which is what it is called when it declares no path of its
        // own. It is read before the slot is taken: the lowering holds the slot of the HIR
        // while it runs.
        let Some(relative) = self.module_path(ModuleId(file)) else {
            // The place names no file, and a place that names no file names no module: there
            // is no path to call one by, and nothing to lower it to.
            self.lowered.remove(&file);
            return None;
        };

        let version = self.file_version(file);
        let module = ModuleId(file);

        // The projects the module may name, which the lowering reads the name a type is rooted
        // at against ([ADR-0016]).
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        let projects: Vec<ProjectId> = self.named_projects(module);

        Self::text_derived(&mut self.lowered, file, version, || {
            let root = parse.module_root()?;

            // What the module is read with is the prelude of its project, which is the
            // prelude of the language for a module no project claims.
            let prelude = self.projects.prelude_of(module);

            Some(Arc::new(Lowered::of(
                module,
                &root,
                prelude,
                &projects,
                relative.as_path(),
            )))
        })
    }

    /// The interface of `module`: what it shows to the modules that name it.
    ///
    /// The interface is a function of the module's own text, which is the HIR the driver
    /// holds ([ADR-0016]): a module that changed its surface is described again, and one whose
    /// item tree stayed the same --- a body edited, a private name changed --- is the value the
    /// driver already holds.
    ///
    /// `None` when there is nothing to cut an interface from: see [`Driver::lower`].
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn interface(&mut self, module: ModuleId) -> Option<Arc<Interface>> {
        let lowered = self.lower(module.0)?;
        let held = self.interfaces.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
        {
            return Some(slot.value.clone());
        }

        let value = Arc::new(Interface::of(lowered.item_tree()));

        // An interface equal to the one the driver holds is the value it holds: everything
        // keyed by the interface stays where it was ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match held {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.interfaces.insert(module, InterfaceSlot {
            lowered,
            value: value.clone(),
        });

        Some(value)
    }

    /// The module index of `project`: which path of it names which module.
    ///
    /// The index is built from the interfaces of the modules the graph assigns to the project,
    /// entry by entry ([ADR-0016]): a module that changed its surface replaces one entry, and
    /// an index whose entries are the ones it was built from is the value the driver holds.
    ///
    /// `None` when the graph holds no such project.
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn module_index(&mut self, project: &ProjectId) -> Option<Arc<ModuleIndex>> {
        self.projects.project(project)?;

        let modules: Vec<ModuleId> = self.projects.modules_of(project).collect();
        let mut interfaces: BTreeMap<ModuleId, Arc<Interface>> = BTreeMap::new();

        for module in modules {
            if let Some(interface) = self.interface(module) {
                interfaces.insert(module, interface);
            }
        }

        let held = self.module_indexes.get(project);

        if let Some(slot) = held
            && entries_are_the_same(&slot.interfaces, &interfaces)
        {
            return Some(slot.value.clone());
        }

        let mut index = ModuleIndex::new(project.clone());

        for (module, interface) in &interfaces {
            index.insert(*module, interface.path());
        }

        let value = Arc::new(index);
        let value = match held {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.module_indexes
            .insert(project.clone(), ModuleIndexSlot {
                interfaces,
                value: value.clone(),
            });

        Some(value)
    }

    /// The resolution of `module`: what its names denote, and what its walk found wrong.
    ///
    /// The input of the resolution is the closure of the modules its paths name ([ADR-0009]),
    /// gathered here by making the walk once: what the walk reaches --- the interfaces, and the
    /// entries it read of the module indexes --- is what the pass that follows is handed, and
    /// what the slot is keyed by ([ADR-0016]).
    ///
    /// `None` when there is nothing to resolve: see [`Driver::lower`].
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn resolution(&mut self, module: ModuleId) -> Option<Arc<Resolution>> {
        let lowered = self.lower(module.0)?;
        let graph = Arc::clone(&self.projects);
        let indexes = self.indexes_of(module);

        let closure = Closure::of(
            module,
            lowered.item_tree(),
            &graph,
            &indexes,
            &mut |module| self.interface(module),
        );

        let held = self.resolutions.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && slot.closure.reads_the_same_as(&closure)
        {
            return Some(slot.value.clone());
        }

        let value = Arc::new(resolve_module(module, lowered.item_tree(), &ResolveDeps {
            graph,
            closure: closure.clone(),
        }));

        // A resolution equal to the one the driver holds is the value it holds: a reader that
        // came to the same entities came to nothing new ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match held {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.resolutions.insert(module, ResolutionSlot {
            lowered,
            closure,
            value: value.clone(),
        });

        Some(value)
    }

    /// The classes of the language, as the standard library declares them.
    ///
    /// `Int`, `Unit`, `String`, and `Bool` are ordinary classes ([ADR-0017]): the library
    /// declares them with `#[builtin]`, and the check reads them by name rather than looking a
    /// primitive type up. A driver whose host recorded no library holds no classes to give, and
    /// checks no types: there is nothing for a literal or an operator to be.
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn builtins(&mut self) -> Option<Builtins> {
        let project = ProjectId::new(mlkc_stdlib::PROJECT);
        self.projects.project(&project)?;

        let index = self.module_index(&project)?;
        let core = index.get(&[Name::new(mlkc_stdlib::CORE)])?;
        let lowered = self.lower(core.0)?;
        let tree = lowered.item_tree();

        let class = |name: &str| -> Option<EntityLoc<ClassLoc>> {
            let target = tree.scope().get(&Name::new(name))?.ty.as_ref()?;
            let PathAnchor::Item(entity) = target.anchor() else {
                return None;
            };
            let EntityData::Class(data) = tree.entity_data(entity.item.clone())? else {
                return None;
            };

            // The language's class is the one the library declares as a builtin; a class that
            // happens to carry the name of one is a class like any other.
            if !data.attributes.builtin {
                return None;
            }

            Some(EntityLoc {
                module: entity.module,
                item: ClassLoc::try_from(entity.item).ok()?,
            })
        };

        Some(Builtins::new(
            class("Int")?,
            class("Unit")?,
            class("String")?,
            class("Bool")?,
        ))
    }

    /// The types of `module`'s entities, resolved from the signatures it writes ([ADR-0017]).
    ///
    /// The surface is a function of the module's own text and of the names it read, and never of
    /// a body: an edit inside a body leaves it where it was, and only the check of that body is
    /// read again.
    ///
    /// `None` when there is nothing to resolve the types against: the module has no HIR, or the
    /// driver holds no standard library, whose `#[builtin]` classes the check is given
    /// ([`Driver::use_std`]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn module_types(&mut self, module: ModuleId) -> Option<Arc<ModuleTypes>> {
        self.signatures(module).map(|signatures| signatures.value)
    }

    /// The type surface of `module`, and what resolving its written types reported.
    ///
    /// The key of the slot is what the pass read: the module's item tree, its resolution, and
    /// the closure its written types walk. A recomputation that ends up equal to what the driver
    /// holds is the value it holds, so a reader that came to the same surface came to nothing new
    /// ([ADR-0008]).
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn signatures(&mut self, module: ModuleId) -> Option<Signatures> {
        let lowered = self.lower(module.0)?;
        let resolution = self.resolution(module)?;
        let builtins = self.builtins()?;
        let graph = Arc::clone(&self.projects);
        let indexes = self.indexes_of(module);

        let closure = Closure::of(
            module,
            lowered.item_tree(),
            &graph,
            &indexes,
            &mut |module| self.interface(module),
        );

        let held = self.signatures.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && Arc::ptr_eq(&slot.resolution, &resolution)
            && slot.closure.reads_the_same_as(&closure)
        {
            return Some(slot.signatures.clone());
        }

        let deps = CheckDeps::new(builtins)
            .with_graph(graph)
            .with_closure(closure.clone());
        let (value, diagnostics) = resolve_module_types(lowered.item_tree(), &resolution, &deps);
        let signatures = Signatures {
            value: Arc::new(value),
            diagnostics: Arc::from(diagnostics),
        };

        // A surface equal to the one the driver holds is the value it holds: a reader that came
        // to the same types came to nothing new ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let signatures = match held {
            Some(slot)
                if *slot.signatures.value == *signatures.value
                    && *slot.signatures.diagnostics == *signatures.diagnostics =>
            {
                slot.signatures.clone()
            },
            _ => signatures,
        };

        self.signatures.insert(module, SignaturesSlot {
            lowered,
            resolution,
            closure,
            signatures: signatures.clone(),
        });

        Some(signatures)
    }

    /// The check of one body: the types of its nodes, checked against the signatures its module
    /// wrote ([ADR-0017]).
    ///
    /// `None` when the body is not a body of a module the driver holds, or when there is nothing
    /// to check against: see [`Driver::module_types`].
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn check(&mut self, owner: &BodyEntityLoc) -> Option<Arc<CheckedBody>> {
        self.checked(owner).map(|checked| checked.value)
    }

    /// The check of one body, and what checking it reported.
    ///
    /// The key of the slot is what the check read: the HIR of the module --- the body among it
    /// --- the resolution, the wider closure of the check (the paths of every body, not only of
    /// the surface), the type surfaces of the modules the paths reach, and the classes of the
    /// language. A body edit changes the HIR, and the recomputation of a body that did not change
    /// ends up equal to what the driver holds, so the value a reader sees stays where it was.
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn checked(&mut self, owner: &BodyEntityLoc) -> Option<Checked> {
        let inputs = self.check_inputs(owner.module())?;

        self.checked_with(owner, &inputs)
    }

    /// What the check of a body reads of the units around it ([`CheckInputs`]).
    ///
    /// `None` when there is nothing to check against: the module has no HIR, it has no resolution,
    /// or the driver holds no standard library, whose `#[builtin]` classes the check is given
    /// ([`Driver::use_std`]).
    fn check_inputs(&mut self, module: ModuleId) -> Option<CheckInputs> {
        let lowered = self.lower(module.0)?;
        let resolution = self.resolution(module)?;
        let builtins = self.builtins()?;
        let graph = Arc::clone(&self.projects);
        let indexes = self.indexes_of(module);

        // The closure of the check is wider than the closure of a resolution: a path of a body
        // may name a module that no signature names ([ADR-0017]).
        let bodies: Vec<&Body> = lowered
            .bodies()
            .iter()
            .map(|body| &body.body().body)
            .collect();
        let closure = Closure::of_check(
            module,
            lowered.item_tree(),
            bodies,
            &graph,
            &indexes,
            &mut |module| self.interface(module),
        );

        // Every module the check may read a type from: its own, and the ones its paths reach.
        let mut modules: Vec<ModuleId> = closure.interfaces().keys().copied().collect();
        modules.push(module);
        modules.sort_unstable();
        modules.dedup();

        let mut types: BTreeMap<ModuleId, Arc<ModuleTypes>> = BTreeMap::new();

        for named in modules {
            if let Some(signatures) = self.signatures(named) {
                types.insert(named, signatures.value);
            }
        }

        Some(CheckInputs {
            lowered,
            resolution,
            closure,
            types,
            builtins,
        })
    }

    /// The check of a body against inputs already gathered ([`Driver::check_inputs`]).
    fn checked_with(&mut self, owner: &BodyEntityLoc, inputs: &CheckInputs) -> Option<Checked> {
        let CheckInputs {
            lowered,
            resolution,
            closure,
            types,
            builtins,
        } = inputs;
        let held = self.checks.get(owner);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, lowered)
            && Arc::ptr_eq(&slot.resolution, resolution)
            && slot.closure.reads_the_same_as(closure)
            && entries_are_the_same(&slot.types, types)
            && slot.builtins == *builtins
        {
            return Some(slot.checked.clone());
        }

        let body = lowered.bodies().iter().find(|body| body.owner() == owner)?;
        let mut deps = CheckDeps::new(builtins.clone())
            .with_graph(Arc::clone(&self.projects))
            .with_closure(closure.clone());

        for (named, surface) in types {
            deps = deps.with_types(*named, Arc::clone(surface));
        }

        let (value, diagnostics) = check_body(
            owner.clone(),
            lowered.item_tree(),
            &body.body().body,
            resolution,
            &deps,
        );
        let checked = Checked {
            value: Arc::new(value),
            diagnostics: Arc::from(diagnostics),
        };

        // A check that ends up equal to the one the driver holds is the one it holds: a body edit
        // neither moves the types of the bodies that did not change nor the values that read
        // them ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let checked = match held {
            Some(slot)
                if *slot.checked.value == *checked.value
                    && *slot.checked.diagnostics == *checked.diagnostics =>
            {
                slot.checked.clone()
            },
            _ => checked,
        };

        self.checks.insert(owner.clone(), CheckSlot {
            lowered: Arc::clone(lowered),
            resolution: Arc::clone(resolution),
            closure: closure.clone(),
            types: types.clone(),
            builtins: builtins.clone(),
            checked: checked.clone(),
        });

        Some(checked)
    }

    /// The MIR of `owner` in the CFG form: the checked body lowered, before SSA ([ADR-0019]).
    ///
    /// `None` when the body is not one the front end read clean: the file has no parse, the
    /// parser reported a mistake about it, the lowering of the HIR reported a mistake about the
    /// body, or the check of it did. A body whose meaning is a mistake has no MIR, so no back end
    /// meets an expression whose meaning is one.
    ///
    /// [ADR-0019]: ../../docs/adr/0019-mir.md
    pub fn mir(&mut self, owner: &BodyEntityLoc) -> Option<Arc<MirBody>> {
        self.checked_mir(owner)
    }

    /// The MIR of `owner` in the SSA form: the CFG form with block parameters ([ADR-0019]).
    ///
    /// The pass reads the CFG form and nothing else, so a second pull is the value the first one
    /// returned.
    ///
    /// [ADR-0019]: ../../docs/adr/0019-mir.md
    pub fn mir_ssa(&mut self, owner: &BodyEntityLoc) -> Option<Arc<MirBody>> {
        let cfg = self.mir(owner)?;

        if let Some(slot) = self.ssas.get(owner)
            && Arc::ptr_eq(&slot.cfg, &cfg)
        {
            return Some(slot.value.clone());
        }

        let value = Arc::new(construct_ssa(&cfg));

        // An SSA form equal to the one the driver holds is the value it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.ssas.get(owner) {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.ssas.insert(owner.clone(), SsaSlot {
            cfg,
            value: value.clone(),
        });

        Some(value)
    }

    /// The MIR of one body in the CFG form, computed against the inputs of its check.
    fn checked_mir(&mut self, owner: &BodyEntityLoc) -> Option<Arc<MirBody>> {
        let module = owner.module();

        // A file the parser reported a mistake about is not compiled: a body of it may hold an
        // expression that is not there, and MIR has no meaning for one.
        if self.parse(module.0)?.has_errors() {
            return None;
        }

        let inputs = self.check_inputs(module)?;
        let checked = self.checked_with(owner, &inputs)?;

        // A body whose check reported a mistake is not lowered ([ADR-0019]): there is no meaning
        // to lower, and codegen never meets an expression whose meaning is a mistake.
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        if !checked.diagnostics.is_empty() {
            return None;
        }

        let held = self.mirs.get(owner);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &inputs.lowered)
            && Arc::ptr_eq(&slot.resolution, &inputs.resolution)
            && slot.closure.reads_the_same_as(&inputs.closure)
            && slot.builtins == inputs.builtins
            && slot.checked.reads_the_same_as(&checked)
        {
            return Some(slot.value.clone());
        }

        let body = inputs
            .lowered
            .bodies()
            .iter()
            .find(|body| body.owner() == owner)?;

        // A body the HIR lowering reported a mistake about is not compiled: what it holds is not
        // a body the language means.
        if !body.body().diagnostics.is_empty() {
            return None;
        }

        // The surfaces the check read are not handed over: the lowering reads the types of the
        // check's value, and no surface of another module.
        let deps = CheckDeps::new(inputs.builtins.clone())
            .with_graph(Arc::clone(&self.projects))
            .with_closure(inputs.closure.clone());

        let value = Arc::new(lower_mir(
            owner.clone(),
            inputs.lowered.item_tree(),
            &body.body().body,
            &body.body().source_map,
            &checked.value,
            &inputs.resolution,
            &deps,
        ));

        // MIR that ends up equal to the one the driver holds is the one it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match held {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.mirs.insert(owner.clone(), MirSlot {
            lowered: Arc::clone(&inputs.lowered),
            resolution: Arc::clone(&inputs.resolution),
            closure: inputs.closure.clone(),
            builtins: inputs.builtins.clone(),
            checked,
            value: value.clone(),
        });

        Some(value)
    }

    /// The def map of `project`: the scopes of its modules.
    ///
    /// The map is the index of the resolutions of the project's modules ([ADR-0016]): it holds
    /// the scope of a module, and it is keyed by the resolutions it was built from, entry by
    /// entry. A project the graph holds no modules of has an empty map.
    ///
    /// `None` when the graph holds no such project.
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn def_map(&mut self, project: &ProjectId) -> Option<Arc<ProjectDefMap>> {
        self.projects.project(project)?;

        let modules: Vec<ModuleId> = self.projects.modules_of(project).collect();
        let mut resolutions: BTreeMap<ModuleId, Arc<Resolution>> = BTreeMap::new();

        for module in modules {
            if let Some(resolution) = self.resolution(module) {
                resolutions.insert(module, resolution);
            }
        }

        let held = self.def_maps.get(project);

        if let Some(slot) = held
            && entries_are_the_same(&slot.resolutions, &resolutions)
        {
            return Some(slot.value.clone());
        }

        let mut map = ProjectDefMap::default();

        for (module, resolution) in &resolutions {
            map.set(*module, Arc::clone(resolution.scope()));
        }

        let value = Arc::new(map);
        let value = match held {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.def_maps.insert(project.clone(), DefMapSlot {
            resolutions,
            value: value.clone(),
        });

        Some(value)
    }

    /// The module indexes a path of `module` may be read in.
    ///
    /// A path is read inside a project: the names of the project a module belongs to are read in
    /// the index of that project --- the module calls it by the keyword --- and the names of the
    /// projects it depends on are read in theirs ([`Driver::read_projects`]).
    fn indexes_of(&mut self, module: ModuleId) -> BTreeMap<ProjectId, Arc<ModuleIndex>> {
        let mut indexes = BTreeMap::new();

        for project in self.read_projects(module) {
            if let Some(index) = self.module_index(&project) {
                indexes.insert(project, index);
            }
        }

        indexes
    }

    /// The projects a path of a module is read in: the project the module belongs to, and the
    /// projects that project depends on.
    ///
    /// A module names the project it belongs to by the keyword `project`, so what the names
    /// after the keyword are read against is the index of that project, and the names of the
    /// projects it depends on are read against their indexes. A module that belongs to no
    /// project is read in every project of the graph, which is all there is to declare a
    /// dependency on ([ADR-0016]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    fn read_projects(&self, module: ModuleId) -> Vec<ProjectId> {
        match self.projects.project_of(module) {
            Some(project) => {
                match self.projects.project(project) {
                    Some(data) => {
                        std::iter::once(project.clone())
                            .chain(data.dependencies.values().cloned())
                            .collect()
                    },
                    None => Vec::new(),
                }
            },
            None => {
                self.projects
                    .projects()
                    .map(|(project, _)| project.clone())
                    .collect()
            },
        }
    }

    /// The projects a module may name.
    ///
    /// A module names the projects that the project it belongs to depends on; what it calls its
    /// own project by is the keyword `project`, and not a name ([ADR-0016]). A module that
    /// belongs to no project names the projects of the graph, each of them by its own name,
    /// which is all there is to declare a dependency on.
    ///
    /// The lowering is keyed by the list, so it is sorted: the same projects are the same input
    /// however a host declared them.
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    fn named_projects(&self, module: ModuleId) -> Vec<ProjectId> {
        let mut projects: Vec<ProjectId> = match self.projects.project_of(module) {
            Some(project) => {
                match self.projects.project(project) {
                    Some(data) => data.dependencies.values().cloned().collect(),
                    None => Vec::new(),
                }
            },
            None => {
                self.projects
                    .projects()
                    .map(|(project, _)| project.clone())
                    .collect()
            },
        };

        projects.sort();
        projects
    }

    /// Where a module stands: the place of its file, which is what the module is called when it
    /// declares no path of its own.
    ///
    /// The place of a file is the path of it under the root of the file system it was pushed
    /// into: the module `lib/arith.mlk` is the module `project::lib::arith`, and `main.mlk`
    /// itself is `project::main`. A host lays a project out in that file system, so the paths
    /// it pushes are what says where the modules of the project stand.
    ///
    /// A file of the host's own file system is a path of the machine rather than a place in the
    /// tree a host laid out, and a module of either is called by the name of its file, the name
    /// being all there is to say which module it is.
    ///
    /// `None` for a file whose place names no file --- the root of the file system, pushed as
    /// if it were a file --- since a module is a file, and a place that names no file names no
    /// module: there is nothing to lower such a file to.
    fn module_path(&self, module: ModuleId) -> Option<RelPathBuf> {
        let file = self.vfs.file_path(module.0);

        match file.strip_prefix(&root()) {
            // What stands under the root is the place of the module; the root itself is a place
            // that names no file.
            Some(relative) if relative.as_utf8_path().file_name().is_some() => {
                Some(relative.to_path_buf())
            },
            Some(_) => None,
            None => file_name(file),
        }
    }

    /// The diagnostics of `file`: what the parser reported, what the lowering of the module
    /// reported, what the resolution of it found, and what checking its types found
    /// ([ADR-0016], [ADR-0017]).
    ///
    /// Each stage's diagnostics are a value of their own, computed once per what their stage
    /// read and shared as an `Arc`: a mistake the parser reported is not a reason to render
    /// what the lowering and the resolution reported again.
    ///
    /// `None` when the file has no parse: see [`Driver::parse`].
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn diagnostics(&mut self, file: FileId) -> Option<Diagnostics> {
        let parse = self.parse_diagnostics(file)?;
        let lowered = self.lower(file);

        let lowering = lowered
            .as_ref()
            .map_or_else(none, |lowered| lowered.diagnostics().clone());
        let resolution = lowered
            .as_ref()
            .and_then(|_| self.resolution_diagnostics(ModuleId(file)))
            .unwrap_or_else(none);
        let types = lowered
            .as_ref()
            .and_then(|_| self.type_diagnostics(ModuleId(file)))
            .unwrap_or_else(none);

        Some(Diagnostics {
            parse,
            lowering,
            resolution,
            types,
        })
    }

    /// The diagnostics of the parse of `file`, in the shape a host renders.
    ///
    /// The rendering is a function of the parse, so it is keyed by the text of the file like
    /// the parse itself: what a parser reported is rendered once per text it read.
    fn parse_diagnostics(&mut self, file: FileId) -> Option<Arc<[Diagnostic]>> {
        let parse = self.parse(file)?;
        let version = self.file_version(file);

        Self::text_derived(&mut self.parse_diagnostics, file, version, || {
            let rendered = parse
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.to_diagnostic(file))
                .collect::<Vec<_>>();

            Some(Arc::from(rendered))
        })
    }

    /// The diagnostics of the resolution of `module`, in the shape a host renders.
    ///
    /// A resolution reports places in the HIR, and where a place is written is what the driver
    /// holds, so the rendering is the driver's. It is a value of its own, and a resolution or
    /// an HIR that did not change is a value the driver already holds.
    fn resolution_diagnostics(&mut self, module: ModuleId) -> Option<Arc<[Diagnostic]>> {
        let lowered = self.lower(module.0)?;
        let resolution = self.resolution(module)?;

        let held = self.resolution_diagnostics.get(&module).filter(|slot| {
            Arc::ptr_eq(&slot.resolution, &resolution) && Arc::ptr_eq(&slot.lowered, &lowered)
        });

        if let Some(slot) = held {
            // What the rendering looked at is the HIR of the modules a name the walk could not
            // find may belong to: a look that names the same HIR is a look that read the same
            // thing, and the value that was rendered from it is the value the driver holds.
            let looked: Vec<(ModuleId, Arc<Lowered>)> = slot
                .looked
                .iter()
                .map(|(module, lowered)| (*module, lowered.clone()))
                .collect();

            if looked.iter().all(|(module, held)| {
                self.lower(module.0)
                    .is_some_and(|current| Arc::ptr_eq(held, &current))
            }) {
                return Some(self.resolution_diagnostics[&module].value.clone());
            }
        }

        let mut looked = BTreeMap::new();
        let mut rendered = Vec::with_capacity(resolution.diagnostics().len());

        for diagnostic in resolution.diagnostics().iter() {
            rendered.push(self.rendered(diagnostic, module, &lowered, &mut looked));
        }

        let value: Arc<[Diagnostic]> = Arc::from(rendered);

        self.resolution_diagnostics
            .insert(module, ResolutionDiagnosticsSlot {
                resolution,
                lowered,
                looked,
                value: value.clone(),
            });

        Some(value)
    }

    /// The diagnostic a host renders of what a resolution found.
    ///
    /// A name the walk could not find may be a name the module at the end of the path holds and
    /// does not show, and telling that is a look at the module itself: what the look read is
    /// recorded, since it is what the rendering is keyed by ([`hidden_name`]).
    fn rendered(
        &mut self,
        diagnostic: &ResolveDiag,
        file: ModuleId,
        lowered: &Lowered,
        looked: &mut BTreeMap<ModuleId, Arc<Lowered>>,
    ) -> Diagnostic {
        let place = diagnostic.place();
        let range = match place.type_place() {
            Some(type_place) => lowered.type_range(place.item(), type_place),
            None => lowered.item_range(place.item()),
        };

        // A place the driver wrote no range for has nowhere to point at: what the resolution
        // found is still what a host is told about, and it is told without a place.
        let Some(range) = range else {
            return Diagnostic::from_kind(diagnostic.error(), diagnostic.error().message());
        };

        let span = Span::new(file.0, range);
        let error = self.hidden(diagnostic.error(), looked);

        match error {
            Some(hidden) => Diagnostic::from_kind(&hidden, hidden.message()).with_primary(span, ""),
            None => diagnostic.to_diagnostic(span),
        }
    }

    /// The diagnostics of the checks of `module`'s bodies, in the shape a host renders.
    ///
    /// A check reports places in the HIR --- an expression, a pattern, a type a declaration
    /// writes --- and where a place is written is what the driver holds, so the rendering is the
    /// driver's. What a path of a body could not find is looked up in the module the path
    /// reached, exactly as the rendering of a resolution does ([`hidden_name`]), and what the
    /// look read is what the slot is keyed by.
    ///
    /// The reports of the module come first --- what resolving its signatures found, in the
    /// order it declares them --- and then the reports of its bodies, in the order it declares
    /// them.
    fn type_diagnostics(&mut self, module: ModuleId) -> Option<Arc<[Diagnostic]>> {
        let lowered = self.lower(module.0)?;
        let signatures = self.signatures(module)?;

        // Every body of the module is checked, in the order the module declares them.
        let checks: Vec<Checked> = lowered
            .bodies()
            .iter()
            .map(|body| self.checked(body.owner()))
            .collect::<Option<Vec<_>>>()?;

        let held = self.type_diagnostics.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && slot.signatures.reads_the_same_as(&signatures)
            && slot.checks.len() == checks.len()
            && slot
                .checks
                .iter()
                .zip(&checks)
                .all(|(held, current)| held.reads_the_same_as(current))
        {
            // What the rendering looked at is the HIR of the modules a name the walk could not
            // find may belong to: a look that names the same HIR is a look that read the same
            // thing, and the value that was rendered from it is the value the driver holds.
            let looked: Vec<(ModuleId, Arc<Lowered>)> = slot
                .looked
                .iter()
                .map(|(module, lowered)| (*module, lowered.clone()))
                .collect();

            if looked.iter().all(|(module, held)| {
                self.lower(module.0)
                    .is_some_and(|current| Arc::ptr_eq(held, &current))
            }) {
                return Some(self.type_diagnostics[&module].value.clone());
            }
        }

        let mut looked = BTreeMap::new();
        let mut rendered = Vec::new();

        // What resolving the signatures reported belongs to the module as a whole, and it comes
        // before what its bodies reported, which is the order the module is read in.
        for diagnostic in signatures.diagnostics.iter() {
            rendered.push(self.rendered_type(diagnostic, module, &lowered, None, &mut looked));
        }

        for (body, checked) in lowered.bodies().iter().zip(&checks) {
            for diagnostic in checked.diagnostics.iter() {
                rendered.push(self.rendered_type(
                    diagnostic,
                    module,
                    &lowered,
                    Some(body),
                    &mut looked,
                ));
            }
        }

        let value: Arc<[Diagnostic]> = Arc::from(rendered);

        // A rendering equal to the one the driver holds is the value it holds: an edit that moved
        // a body but not what a host reads does not move the value the buffer is marked by
        // ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.type_diagnostics.get(&module) {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.type_diagnostics.insert(module, TypeDiagnosticsSlot {
            lowered,
            signatures,
            checks,
            looked,
            value: value.clone(),
        });

        Some(value)
    }

    /// The diagnostic a host renders of what a check found.
    ///
    /// A name a path of a body could not find may be a name the module at the end of the path
    /// holds and does not show, and telling that is a look at the module itself: what the look
    /// read is recorded, since it is what the rendering is keyed by ([`hidden_name`]).
    fn rendered_type(
        &mut self,
        diagnostic: &TypeDiag,
        file: ModuleId,
        lowered: &Lowered,
        body: Option<&ModuleBody>,
        looked: &mut BTreeMap<ModuleId, Arc<Lowered>>,
    ) -> Diagnostic {
        // A place the driver wrote no range for has nowhere to point at: what the check found is
        // still what a host is told about, and it is told without a place.
        let Some(range) = type_place_range(lowered, body, diagnostic.place()) else {
            return Diagnostic::from_kind(diagnostic.error(), diagnostic.error().message());
        };

        let span = Span::new(file.0, range);

        // The paths of a body are walked by the check and never by the resolution ([ADR-0016]),
        // so a name a body walk could not find is the check's to report --- and the look that
        // tells a name a module keeps to itself is the look the rendering of a resolution
        // takes.
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        let error = match diagnostic.error() {
            TypeError::Unresolved { error } => self.hidden(error, looked),
            _ => None,
        };

        match error {
            Some(hidden) => diagnostic.to_diagnostic_with(span, hidden.message()),
            None => diagnostic.to_diagnostic(span),
        }
    }

    /// The error a walk should have reported instead, when the name it could not find is a name
    /// a module keeps to itself ([`hidden_name`]).
    ///
    /// The look is taken only for a name a walk could not find, and what it read is what
    /// a caller keys the rendered diagnostics by: a module that resolved cleanly reads no item
    /// tree of another module, and the key of its diagnostics is not widened by this.
    fn hidden(
        &mut self,
        error: &ResolveError,
        looked: &mut BTreeMap<ModuleId, Arc<Lowered>>,
    ) -> Option<ResolveError> {
        let ResolveError::UnknownName { path, module, name } = error else {
            return None;
        };

        let lowered = self.lower(module.0)?;
        let hidden = hidden_name(path, *module, name, lowered.item_tree());

        looked.insert(*module, lowered);

        hidden
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

    /// Drops what belongs to `project`: the HIR of its modules, their interfaces and
    /// resolutions, its module index and its def map.
    ///
    /// What a project says is what its modules are read under, and the prelude is the part of
    /// it that lowering reads: a module of another project, or one of no project, is not
    /// affected by a change to this one --- and neither are the modules that read the interfaces
    /// of this one's modules, which see the new values when their own resolutions are read
    /// again. The parses stay, since a parse reads no project.
    fn invalidate_project(&mut self, project: &ProjectId) {
        let modules: Vec<ModuleId> = self.projects.modules_of(project).collect();

        for module in modules {
            self.invalidate_module(module);
        }

        self.module_indexes.remove(project);
        self.def_maps.remove(project);
    }

    /// Drops the values derived from one module: its HIR, its interface, its resolution, its
    /// type surface, the checks and the MIR of its bodies, and the diagnostics that were rendered
    /// from them.
    ///
    /// A module that changed project or text is read again; what reads this module is not
    /// dropped with it, and sees the new value when it is read again.
    fn invalidate_module(&mut self, module: ModuleId) {
        self.lowered.remove(&module.0);
        self.interfaces.remove(&module);
        self.resolutions.remove(&module);
        self.resolution_diagnostics.remove(&module);
        self.signatures.remove(&module);
        self.type_diagnostics.remove(&module);
        self.checks.retain(|owner, _| owner.module() != module);
        self.mirs.retain(|owner, _| owner.module() != module);
        self.ssas.retain(|owner, _| owner.module() != module);
    }
}

/// Whether two maps hold the same value for every module, value by value.
///
/// What a consumer of a project is keyed by is the contributions of its modules, entry by
/// entry ([ADR-0008]): a value the driver already holds --- the same `Arc` --- is the same
/// contribution, and a value that is new is one the consumer has not read.
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
fn entries_are_the_same<T>(
    held: &BTreeMap<ModuleId, Arc<T>>,
    current: &BTreeMap<ModuleId, Arc<T>>,
) -> bool {
    held.len() == current.len()
        && held.iter().all(|(module, value)| {
            current
                .get(module)
                .is_some_and(|current| Arc::ptr_eq(value, current))
        })
}

/// Where the place a check reported is written, if the driver found it.
///
/// An expression and a pattern are nodes of one body, and where they are written is the source
/// map of that body; an entity and a type a declaration writes are part of the surface of the
/// module, and where they are written is what the HIR of the module holds. A node the lowering
/// made up has no range of its own, and the declaration it stands in is what is marked for it
/// instead.
fn type_place_range(
    lowered: &Lowered,
    body: Option<&ModuleBody>,
    place: &TypePlace,
) -> Option<TextRange> {
    let declaration = || {
        let item = body.map(|body| ItemLoc::from(body.owner().item.clone()))?;
        lowered.item_range(&item)
    };

    match place {
        TypePlace::Expr(expr) => {
            body.and_then(|body| body.body().source_map.expr(*expr))
                .or_else(declaration)
        },
        TypePlace::Pat(pat) => {
            body.and_then(|body| body.body().source_map.pat(*pat))
                .or_else(declaration)
        },
        TypePlace::Entity(item) => lowered.item_range(item),
        TypePlace::Declared { item, place } => {
            lowered
                .type_range(item, *place)
                .or_else(|| lowered.item_range(item))
        },
    }
}

/// The diagnostics of one file, in the order the stages of the pipeline reported them.
///
/// The parts are the values the stages left: what the parser reported, what the lowering of the
/// module reported, what the resolution of it found, and what checking its types found. A part
/// that did not change is the value the driver already held, and no part is copied into another:
/// what a host reads is the parts in order ([`Diagnostics::iter`]).
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// What the parser reported.
    parse: Arc<[Diagnostic]>,
    /// What the lowering of the module reported.
    lowering: Arc<[Diagnostic]>,
    /// What the resolution of the module reported.
    resolution: Arc<[Diagnostic]>,
    /// What checking the types of the module's signatures and bodies reported.
    types: Arc<[Diagnostic]>,
}

impl Diagnostics {
    /// What the parser reported, as the stage left it.
    pub fn parse(&self) -> &Arc<[Diagnostic]> {
        &self.parse
    }

    /// What the lowering of the module reported, as the stage left it.
    pub fn lowering(&self) -> &Arc<[Diagnostic]> {
        &self.lowering
    }

    /// What the resolution of the module reported, as the stage left it.
    pub fn resolution(&self) -> &Arc<[Diagnostic]> {
        &self.resolution
    }

    /// What checking the types of the module reported, as the stage left it.
    pub fn types(&self) -> &Arc<[Diagnostic]> {
        &self.types
    }

    /// The diagnostics of the file, in the order the stages reported them.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.parse
            .iter()
            .chain(self.lowering.iter())
            .chain(self.resolution.iter())
            .chain(self.types.iter())
    }

    /// How many diagnostics the file has.
    pub fn len(&self) -> usize {
        self.parse.len() + self.lowering.len() + self.resolution.len() + self.types.len()
    }

    /// Whether the file has no diagnostics at all.
    pub fn is_empty(&self) -> bool {
        self.parse.is_empty()
            && self.lowering.is_empty()
            && self.resolution.is_empty()
            && self.types.is_empty()
    }
}

/// What a stage that reported nothing is handed: one empty list for every stage and file.
///
/// An empty list is one value whatever it is a list of, and a caller that compares what it was
/// handed --- which is what a driver is for --- compares the value that is there rather than a
/// new one that is equal to it.
fn none() -> Arc<[Diagnostic]> {
    static NONE: OnceLock<Arc<[Diagnostic]>> = OnceLock::new();

    NONE.get_or_init(|| Arc::from(Vec::new())).clone()
}

/// The root of the file system: what the place of a file is read against
/// ([`Driver::module_path`]).
fn root() -> VfsPath {
    VfsPath::new_virtual_path("/".to_owned())
}

/// The path of a file under the directory it is written in: the name of the file itself.
///
/// This is what a module is called by when the place of its file says nothing of it: a file of
/// the host's own file system, and a file written at the root of the file system. The extension
/// is kept --- it is what says that the file is a file of the language, and it is lowering that
/// leaves it out of the name of the module --- and a path that names no file names no module.
fn file_name(path: &VfsPath) -> Option<RelPathBuf> {
    let name = match path.name_and_extension() {
        Some((stem, Some(extension))) => format!("{stem}.{extension}"),
        Some((stem, None)) => stem.to_owned(),
        None => return None,
    };

    Some(RelPathBuf::try_from(name.as_str()).expect("the name of a file to be a relative path"))
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
    const MODULE: &str = "\
#[extern]
fun println-int(value: Int): Unit

fun main(): Unit =
    let x = 42 * 2 - 10 in
    println-int(x + 20)
";

    /// The same module with the `in` of the `let` missing.
    const BROKEN: &str = "fun main(): Unit =\n    let x = 1\n";

    /// A module of a project: it shows a class, and has a body no other module reads.
    const DATA: &str = "\
pub type Point

pub fun origin(): Point = origin()

fun helper() = 1
";

    /// The same module with the body of the private function edited.
    const DATA_EDITED: &str = "\
pub type Point

pub fun origin(): Point = origin()

fun helper() = 2
";

    /// The same module with another name shown.
    const DATA_EXTENDED: &str = "\
pub type Point

pub type Extra

pub fun origin(): Point = origin()
";

    /// A module of the same project that reads what `DATA` shows.
    const READER: &str = "\
use project::data::Point

fun get(): Point = get()
";

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

    /// A driver that holds one file of a project that depends on the standard library.
    ///
    /// The names of the language are names the library declares ([ADR-0011]): a module that
    /// writes `Unit` is a module whose project is one that depends on `std`, and whose prelude
    /// reaches the name through the re-export the library writes.
    ///
    /// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    fn driver_with_std(name: &str, text: &str) -> (Driver, FileId) {
        let mut driver = Driver::new();
        driver.use_std();

        let mut data = ProjectData::default();
        data.dependencies.insert(
            Name::new(mlkc_stdlib::PROJECT),
            ProjectId::new(mlkc_stdlib::PROJECT),
        );
        driver.set_project(project(), data);

        let path = path(name);
        driver.set_file_text(path.clone(), Some(text.to_string()));

        let file = driver.file_id(&path).expect("the file to have an id");
        driver.set_module_project(ModuleId(file), project());

        (driver, file)
    }

    /// A driver that holds the project a fixture writes, one module per file.
    ///
    /// A fixture holds a project in one value ([`mlkc_fixture`]): every module of it is headed
    /// by the place it stands at, and what follows is its source.
    ///
    /// The project depends on nothing and gives its modules no prelude: what a test writes is
    /// what its modules resolve against.
    fn project_of(fixture: &str) -> Driver {
        let mut driver = Driver::new();

        let data = ProjectData {
            prelude: Prelude::none(),
            ..ProjectData::default()
        };
        driver.set_project(project(), data);

        for module in mlkc_fixture::modules(fixture) {
            let path = VfsPath::new_virtual_path(module.place.clone());
            driver.set_file_text(path.clone(), Some(module.source.clone()));

            let file = driver.file_id(&path).expect("the file to have an id");
            driver.set_module_project(ModuleId(file), project());
        }

        driver
    }

    /// The id of the file a fixture wrote at `place`.
    fn file(driver: &Driver, place: &str) -> FileId {
        driver
            .file_id(&path(place))
            .unwrap_or_else(|| panic!("a fixture to write a module at `{place}`"))
    }

    /// A driver that holds the project a fixture writes, every module of it depending on the
    /// standard library.
    ///
    /// The names of the language are names the library declares ([ADR-0011]): a module of the
    /// fixture writes `Int` because the prelude of the project, the prelude of the language,
    /// reaches the name through the re-export the library writes. A check reads the classes of
    /// the language from the library, so the fixture is the one of tests that check types.
    ///
    /// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    fn std_project_of(fixture: &str) -> Driver {
        let mut driver = Driver::new();
        driver.use_std();

        let mut data = ProjectData::default();
        data.dependencies.insert(
            Name::new(mlkc_stdlib::PROJECT),
            ProjectId::new(mlkc_stdlib::PROJECT),
        );
        driver.set_project(project(), data);

        for module in mlkc_fixture::modules(fixture) {
            let path = VfsPath::new_virtual_path(module.place.clone());
            driver.set_file_text(path.clone(), Some(module.source.clone()));

            let file = driver.file_id(&path).expect("the file to have an id");
            driver.set_module_project(ModuleId(file), project());
        }

        driver
    }

    /// The owner of the body a module declares under `name`, in its current revision.
    fn body_of(driver: &mut Driver, file: FileId, name: &str) -> BodyEntityLoc {
        let lowered = driver.lower(file).expect("the file to be lowered");

        lowered
            .bodies()
            .iter()
            .find(|body| body.owner().item.name() == Some(&Name::new(name)))
            .map_or_else(
                || panic!("the module to declare a body for `{name}`"),
                |body| body.owner().clone(),
            )
    }

    /// A driver that holds a project of two modules: the one of `data`, and the one that reads
    /// it. Both stand at the root of the file system, which is what names them.
    fn reader_and_data(data: &str, reader: &str) -> (Driver, FileId, FileId) {
        let fixture = format!("//- /data.mlk\n{data}\n//- /main.mlk\n{reader}");
        let driver = project_of(&fixture);

        let data = file(&driver, "data.mlk");
        let main = file(&driver, "main.mlk");

        (driver, main, data)
    }

    /// The module of the standard library the driver holds, by the name the library gives it.
    fn std_module(driver: &Driver, name: &str) -> ModuleId {
        let module = mlkc_stdlib::modules()
            .iter()
            .find(|module| module.name == name)
            .expect("a module the library has");

        ModuleId(
            driver
                .file_id(&mlkc_stdlib::path(module))
                .expect("a module of the library to be pushed"),
        )
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

        assert_eq!(root.items().len(), 2);
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

        // The module declares `println-int` and `main`, and the prelude of the language brings
        // in four more names.
        assert_eq!(
            lowered.item_tree().scope().len(),
            6,
            "six names are declared"
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
            covered(source, lowered.type_range(&f, DeclaredType::Parameter(0))),
            Some("Map[Int]"),
            "a parameter is read as the type the declaration annotated it with"
        );
        assert_eq!(
            covered(source, lowered.type_range(&f, DeclaredType::Result)),
            Some("Int")
        );
        assert_eq!(
            lowered.type_range(&f, DeclaredType::Parameter(1)),
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
            lowered.type_range(&f, DeclaredType::Parameter(0)),
            None,
            "the parameter that broke has no type to point at"
        );
        assert_eq!(
            covered(source, lowered.type_range(&f, DeclaredType::Parameter(1))),
            Some("Int")
        );
    }

    #[test]
    fn a_lowering_mistake_travels_with_the_diagnostics_of_the_file() {
        // One name declared twice is a mistake the parser has nothing to say about:
        // a module says it, and the HIR cannot hold it.
        let (mut driver, file) = driver_with_std(
            "main.mlk",
            "fun f(): Unit =\n    1\n\nfun f(): Unit =\n    2\n",
        );

        let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

        assert_eq!(diagnostics.lowering().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.lowering()[0];

        assert_eq!(diagnostic.category, Category::Lowering);
        assert_eq!(diagnostic.level, Level::Error);
        assert_eq!(diagnostic.code, "01");
    }

    #[test]
    fn a_mistake_of_a_body_travels_with_the_diagnostics_of_the_file() {
        // A body is lowered from the item tree of the module, and a name it is written with is
        // lowered with it: what a body says wrong is a mistake of the module, and a host reads
        // it for the file like any other.
        let (mut driver, file) = driver_with_std("main.mlk", "fun main(): Unit =\n    nope(1)\n");

        let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

        assert_eq!(diagnostics.lowering().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.lowering()[0];

        assert_eq!(diagnostic.category, Category::Lowering);
        assert_eq!(diagnostic.level, Level::Error);
        assert_eq!(diagnostic.code, "15");
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
        assert_eq!(before.item_tree().scope().len(), 6, "the old value stands");
        assert_eq!(
            after.item_tree().scope().len(),
            5,
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

        assert_eq!(import_path(&lowered, "Int"), "std::prelude::Int");
        assert_eq!(import_path(&lowered, "Unit"), "std::prelude::Unit");

        // The import is written nowhere in the buffer, so a host is given no place to mark.
        let int = item(&lowered, "Int");
        assert_eq!(lowered.item_range(&int), None);
    }

    #[test]
    fn a_module_that_says_no_prelude_is_given_none() {
        let (mut driver, file) = driver_with(
            "main.mlk",
            "#[no-prelude]\nmodule project::main-module\n\nfun main(): Int =\n    1\n",
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

        assert_eq!(import_path(&before, "Int"), "std::prelude::Int");

        // A prelude of the project replaces the one of the language.
        let data = ProjectData {
            prelude: prelude_of(&["project", "core", "Int"]),
            ..ProjectData::default()
        };

        assert!(driver.set_project(project(), data.clone()));
        assert!(
            !driver.set_project(project(), data),
            "the project did not change",
        );
        assert!(driver.set_module_project(ModuleId(file), project()));

        let after = driver.lower(file).expect("the file to be lowered");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
        assert_eq!(import_path(&after, "Int"), "project::core::Int");
        assert_eq!(
            after
                .item_tree()
                .scope()
                .anchor(&Name::new("Unit"), Namespace::Ty),
            PathAnchor::Unresolved,
            "the prelude of the project replaced the one of the language",
        );

        // A parse is not what a project changes: the one the driver already holds stands.
        assert_eq!(
            driver.project_graph().project_of(ModuleId(file)),
            Some(&project()),
        );
        let parsed_again = driver.parse(file).expect("the file to be parsed");
        assert!(Arc::ptr_eq(&parse, &parsed_again), "the parse was not kept");
    }

    #[test]
    fn a_module_is_called_by_where_its_file_stands() {
        let (mut driver, main) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        let lib = path("lib/arith.mlk");

        driver.set_file_text(lib.clone(), Some("fun size(): Int =\n    1\n".to_string()));
        let lib = driver.file_id(&lib).expect("the file to have an id");

        // A module is called by the place of its file, and a project is a set of modules:
        // recording a module as one of the project says nothing about where it stands.
        assert!(driver.set_project(project(), ProjectData::default()));
        assert!(driver.set_module_project(ModuleId(main), project()));
        assert!(driver.set_module_project(ModuleId(lib), project()));

        let root = driver.lower(main).expect("the file to be lowered");
        let module = driver.lower(lib).expect("the file to be lowered");

        assert_eq!(called_by(&root), "project::main");
        assert_eq!(called_by(&module), "project::lib::arith");

        // A file written at the root of the file system has no place under it, and is called
        // by the name of its file: a module a host pushes on its own is the module the file it
        // is written in is called.
        let (mut driver, lone) = driver_with("lone.mlk", "fun main(): Int =\n    1\n");
        let lowered = driver.lower(lone).expect("the file to be lowered");

        assert_eq!(called_by(&lowered), "project::lone");

        // A module of a project is called by its own place: the project holds the module, and
        // says nothing about where it stands.
        let (mut driver, first) = driver_with("lib/main.mlk", "fun main(): Int =\n    1\n");
        let elsewhere = path("main.mlk");

        driver.set_file_text(
            elsewhere.clone(),
            Some("fun size(): Int =\n    1\n".to_string()),
        );
        let elsewhere = driver.file_id(&elsewhere).expect("the file to have an id");

        assert!(driver.set_project(project(), ProjectData::default()));
        assert!(driver.set_module_project(ModuleId(first), project()));
        assert!(driver.set_module_project(ModuleId(elsewhere), project()));

        let lowered = driver.lower(elsewhere).expect("the file to be lowered");

        assert_eq!(called_by(&lowered), "project::main");

        // A file pushed at the root of the file system stands at a place that names no file,
        // and a place that names no file names no module: there is nothing to lower.
        let (mut driver, root) = driver_with("", "fun main(): Int =\n    1\n");

        assert!(driver.lower(root).is_none());
    }

    /// The path a lowered module is called by.
    fn called_by(lowered: &Lowered) -> String {
        lowered.item_tree().path().to_string()
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
            ..ProjectData::default()
        }));
        assert!(driver.set_project(second.clone(), ProjectData::default()));
        assert!(driver.set_module_project(ModuleId(main), first.clone()));
        assert!(driver.set_module_project(ModuleId(lib), second));

        let before_main = driver.lower(main).expect("the file to be lowered");
        let before_lib = driver.lower(lib).expect("the file to be lowered");

        assert_eq!(import_path(&before_main, "Int"), "project::core::Int");
        assert_eq!(import_path(&before_lib, "Int"), "std::prelude::Int");

        // The prelude of one project changes, and the module of the other is left alone.
        assert!(driver.set_project(first, ProjectData {
            prelude: prelude_of(&["project", "other", "Int"]),
            ..ProjectData::default()
        }));

        let after_main = driver.lower(main).expect("the file to be lowered");
        let after_lib = driver.lower(lib).expect("the file to be lowered");

        assert!(
            !Arc::ptr_eq(&before_main, &after_main),
            "the slot was not dropped"
        );
        assert!(Arc::ptr_eq(&before_lib, &after_lib), "the slot was dropped");
        assert_eq!(import_path(&after_main, "Int"), "project::other::Int");
    }

    #[test]
    fn a_module_that_changes_project_is_read_with_the_other_project() {
        let mut driver = Driver::new();
        let project = ProjectId::new("the-project");

        assert!(driver.set_project(project.clone(), ProjectData {
            prelude: prelude_of(&["project", "core", "Int"]),
            ..ProjectData::default()
        }));

        // A module the graph does not know is read with the prelude of the language, and one
        // recorded as a module of a project is read with the prelude of it.
        driver.set_file_text(
            path("lib.mlk"),
            Some("fun size(): Int =\n    1\n".to_string()),
        );
        let lib = driver
            .file_id(&path("lib.mlk"))
            .expect("the file to have an id");
        let before = driver.lower(lib).expect("the file to be lowered");

        assert_eq!(import_path(&before, "Int"), "std::prelude::Int");

        assert!(driver.set_module_project(ModuleId(lib), project.clone()));
        assert!(!driver.set_module_project(ModuleId(lib), project));

        let after = driver.lower(lib).expect("the file to be lowered");

        assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
        assert_eq!(import_path(&after, "Int"), "project::core::Int");
    }

    #[test]
    fn dropping_a_project_returns_its_modules_to_the_prelude_of_the_language() {
        let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
        let project = ProjectId::new("the-project");

        driver.set_project(project.clone(), ProjectData {
            prelude: Prelude::none(),
            ..ProjectData::default()
        });
        assert!(driver.set_module_project(ModuleId(file), project.clone()));

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
        assert_eq!(import_path(&after, "Int"), "std::prelude::Int");
        assert_eq!(driver.project_graph().project_of(ModuleId(file)), None);
    }

    /// The standard library of the language, as a host records it, and the files it was handed.
    fn with_std() -> (Driver, Vec<StdFile>) {
        let mut driver = Driver::new();
        let files = driver.use_std();

        (driver, files)
    }

    /// The id of a module of the library in a driver that holds it.
    fn std_file(driver: &Driver, files: &[StdFile], name: &str) -> FileId {
        let at = mlkc_stdlib::modules()
            .iter()
            .position(|module| module.name == name)
            .expect("a module the library has");

        driver
            .file_id(&files[at].path)
            .expect("a module of the library to be pushed")
    }

    #[test]
    fn the_standard_library_is_the_project_the_compiler_names() {
        let (driver, files) = with_std();
        let id = ProjectId::new(mlkc_stdlib::PROJECT);
        let project = driver
            .project_graph()
            .project(&id)
            .expect("the library to be recorded");

        for module in mlkc_stdlib::modules() {
            let module = ModuleId(std_file(&driver, &files, module.name));

            assert_eq!(driver.project_graph().project_of(module), Some(&id));
        }

        // A project is what its modules are read with, and the library gives its own modules
        // the names of the library.
        assert_eq!(project.prelude, mlkc_stdlib::prelude());
    }

    #[test]
    fn the_standard_library_compiles_without_diagnostics() {
        let (mut driver, files) = with_std();

        for module in mlkc_stdlib::modules() {
            let file = std_file(&driver, &files, module.name);
            let diagnostics = driver
                .diagnostics(file)
                .expect("a module of the library to be parsed");

            assert!(diagnostics.is_empty(), "{}: {diagnostics:?}", module.name);
        }
    }

    #[test]
    fn recording_the_standard_library_twice_changes_nothing() {
        let (mut driver, files) = with_std();
        let core = std_file(&driver, &files, mlkc_stdlib::CORE);
        let before = driver
            .lower(core)
            .expect("the module of the library to be lowered");

        let again = driver.use_std();

        assert_eq!(again, files, "the library lands where it landed");
        let after = driver
            .lower(core)
            .expect("the module of the library to be lowered");

        assert!(
            Arc::ptr_eq(&before, &after),
            "a library that did not change is not read again",
        );
    }

    /// The prelude of the language is a list of names the library exports: the language names
    /// them (`std::prelude::Int`), the library declares and re-exports them, and a test is what
    /// keeps the two sides of that fact from drifting apart ([ADR-0011]).
    ///
    /// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    #[test]
    fn the_prelude_of_the_language_names_what_the_library_exports() {
        let (mut driver, files) = with_std();
        let prelude = driver
            .lower(std_file(&driver, &files, "prelude"))
            .expect("the module that re-exports the names to be lowered");
        let scope = prelude.item_tree().scope();

        for import in Prelude::standard().imports() {
            let anchor = scope.anchor(import.name(), Namespace::Ty);

            assert!(
                matches!(anchor, PathAnchor::Use(_)),
                "the library to export {}: {anchor:?}",
                import.name(),
            );
        }
    }

    #[test]
    fn a_name_of_another_module_resolves_in_the_middle_of_a_project() {
        let (mut driver, main, data) = reader_and_data(DATA, READER);

        let resolution = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");

        assert!(
            resolution.diagnostics().is_empty(),
            "{:?}",
            resolution.diagnostics(),
        );

        let point = resolution
            .scope()
            .get(&Name::new("Point"))
            .expect("`Point` to be a name of the reader");
        let (entity, _) = point.ty.clone().expect("`Point` to be a type");

        assert_eq!(entity.module, ModuleId(data));
    }

    #[test]
    fn a_body_edit_leaves_the_interface_of_a_module_where_it_was() {
        let (mut driver, _, data) = reader_and_data(DATA, READER);

        let module = ModuleId(data);
        let before = driver
            .interface(module)
            .expect("the module to have an interface");

        driver.set_file_text(path("data.mlk"), Some(DATA_EDITED.to_owned()));
        let after = driver
            .interface(module)
            .expect("the module to have an interface");

        // What a module shows is a function of its own text, and a body is not what it shows:
        // the interface a person reads is the value the driver already held.
        assert!(
            Arc::ptr_eq(&before, &after),
            "a body edit changed the interface"
        );
    }

    #[test]
    fn a_body_edit_of_a_module_leaves_the_modules_that_read_it_where_they_were() {
        let (mut driver, main, _) = reader_and_data(DATA, READER);

        let before = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");

        driver.set_file_text(path("data.mlk"), Some(DATA_EDITED.to_owned()));

        let after = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");

        // The interface of `data` is the value the driver holds, so the walk of `main` reaches
        // it again: what `main` resolved to is what it had, and nothing of it was paid for.
        assert!(
            Arc::ptr_eq(&before, &after),
            "a body edit re-resolved a reader"
        );
    }

    #[test]
    fn a_name_added_to_a_module_does_not_move_what_its_readers_resolved_to() {
        let (mut driver, main, data) = reader_and_data(DATA, READER);

        let module = ModuleId(data);
        let before = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");
        let interface = driver
            .interface(module)
            .expect("the module to have an interface");

        driver.set_file_text(path("data.mlk"), Some(DATA_EXTENDED.to_owned()));

        let after = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");
        let current = driver
            .interface(module)
            .expect("the module to have an interface");

        // A name added is a different interface, so the readers of `data` walk again --- and
        // the walk ends at the same entity, so the resolution it came to is the one it had.
        assert!(
            !Arc::ptr_eq(&interface, &current),
            "the interface to change"
        );
        assert!(
            Arc::ptr_eq(&before, &after),
            "a name added re-resolved a reader to something else",
        );
    }

    #[test]
    fn the_index_of_a_project_holds_the_modules_its_files_are() {
        let (mut driver, main, data) = reader_and_data(DATA, READER);

        let index = driver
            .module_index(&project())
            .expect("the project to have an index");

        assert_eq!(index.get(&[Name::new("data")]), Some(ModuleId(data)));
        assert_eq!(index.get(&[Name::new("main")]), Some(ModuleId(main)));
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn the_def_map_of_a_project_holds_the_scopes_of_its_modules() {
        let (mut driver, main, _) = reader_and_data(DATA, READER);

        let map = driver
            .def_map(&project())
            .expect("the project to have a def map");
        let resolution = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");

        assert_eq!(map.len(), 2);
        assert!(
            Arc::ptr_eq(
                map.get(ModuleId(main))
                    .expect("the reader to be in the map"),
                resolution.scope(),
            ),
            "the map to hold the scope of the resolution",
        );
    }

    #[test]
    fn a_name_a_module_keeps_to_itself_is_told_about_as_kept() {
        let mut driver = project_of(
            "\
//- /data.mlk
type Hidden

//- /main.mlk
use project::data::Hidden

fun get() = 1
",
        );
        let main = file(&driver, "main.mlk");

        let diagnostics = driver.diagnostics(main).expect("the file to be diagnosed");

        // The walk of the reader finds no name where it ends, and telling a reader why is a
        // look at the module the name belongs to --- which is read only for such a name.
        assert_eq!(diagnostics.resolution().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.resolution()[0];

        assert_eq!(diagnostic.category, Category::Resolver);
        assert_eq!(diagnostic.code, "08");
        assert_eq!(
            diagnostic.message,
            "the module `project::data` holds the name `Hidden` and does not show it",
        );
    }

    #[test]
    fn what_a_resolution_found_travels_with_the_diagnostics_of_the_file() {
        const SOURCE: &str = "use project::data::Nope\n\nfun get() = 1\n";

        let (mut driver, main, _) = reader_and_data(DATA, SOURCE);

        let diagnostics = driver.diagnostics(main).expect("the file to be diagnosed");

        assert_eq!(diagnostics.resolution().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.resolution()[0];
        let label = diagnostic.labels.first().expect("a label");

        // A resolution reports a place in the HIR, and the driver turns it into the span of the
        // text the place is written at.
        assert_eq!(diagnostic.category, Category::Resolver);
        assert_eq!(diagnostic.code, "04");
        assert_eq!(
            diagnostic.message,
            "the module `project::data` exports no name `Nope`"
        );
        assert_eq!(
            covered(SOURCE, Some(label.span.range)),
            Some("use project::data::Nope")
        );
    }

    #[test]
    fn a_module_added_under_a_prefix_makes_the_module_that_named_it_resolve_again() {
        let mut driver = project_of(
            "\
//- /main.mlk
use project::data

use project::data::utils::Point

fun get(): Point = get()
",
        );
        let main = file(&driver, "main.mlk");

        let before = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");

        // No module of `data` is there yet, so the paths that read it name nothing.
        assert!(!before.diagnostics().is_empty());
        assert!(before.scope().get(&Name::new("Point")).is_none());

        let path = path("data/utils.mlk");
        driver.set_file_text(path.clone(), Some("pub type Point\n".to_owned()));

        let utils = driver.file_id(&path).expect("the file to have an id");
        driver.set_module_project(ModuleId(utils), project());

        let after = driver
            .resolution(ModuleId(main))
            .expect("the module to resolve");

        // A module that appeared under the prefix is a module the walk reads: the resolution of
        // a module that named the prefix is read again ([ADR-0016]).
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        assert!(after.diagnostics().is_empty(), "{:?}", after.diagnostics());
        assert_eq!(
            after
                .scope()
                .get(&Name::new("Point"))
                .and_then(|per_ns| per_ns.ty.clone())
                .map(|(entity, _)| entity.module),
            Some(ModuleId(utils)),
        );
    }

    #[test]
    fn the_names_of_the_language_resolve_through_the_library() {
        let (mut driver, file) = driver_with_std("main.mlk", MODULE);

        let resolution = driver
            .resolution(ModuleId(file))
            .expect("the module to resolve");

        assert!(
            resolution.diagnostics().is_empty(),
            "{:?}",
            resolution.diagnostics(),
        );

        let unit = resolution
            .scope()
            .get(&Name::new("Unit"))
            .expect("`Unit` to be a name of the module");
        let (entity, _) = unit.ty.clone().expect("`Unit` to be a type");

        // The name is the one the library declares: the prelude of the language names a path of
        // `std::prelude`, and the walk follows the re-export it wrote ([ADR-0011]).
        //
        // [ADR-0011]: ../../docs/adr/0011-module-prelude.md
        assert_eq!(entity.module, std_module(&driver, mlkc_stdlib::CORE));
        assert!(
            driver
                .diagnostics(file)
                .expect("the file to be diagnosed")
                .is_empty()
        );
    }

    /// A path in a prelude, as the paths of a project are written.
    fn prelude_of(path: &[&str]) -> Prelude {
        Prelude::from_paths([PlainPath::from_segments(
            path.iter().map(|segment| Name::new(segment)),
        )])
    }

    #[test]
    fn the_types_of_a_module_are_resolved_from_the_signatures_it_writes() {
        let (mut driver, file) =
            driver_with_std("main.mlk", "fun double(value: Int): Int = value\n");
        let module = ModuleId(file);
        let types = driver
            .module_types(module)
            .expect("the signatures of the module to resolve");
        let lowered = driver.lower(file).expect("the file to be lowered");
        let double = lowered
            .item_tree()
            .entities()
            .find(|(item, _)| item.name() == Some(&Name::new("double")))
            .map(|(item, _)| EntityLoc { module, item })
            .expect("the module to declare `double`");

        // The signature is a value: `Int` is the class the library declares, and the type of the
        // function reads as the types it writes ([ADR-0017]).
        //
        // [ADR-0017]: ../../docs/adr/0017-resolved-types.md
        assert_eq!(
            types.get(&double).map(ToString::to_string),
            Some("(Int) -> Int".to_owned()),
        );
    }

    #[test]
    fn a_body_is_checked_against_the_signatures_of_its_module() {
        let (mut driver, file) =
            driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
        let owner = body_of(&mut driver, file, "double");
        let lowered = driver.lower(file).expect("the file to be lowered");
        let body = lowered
            .bodies()
            .iter()
            .find(|body| body.owner() == &owner)
            .expect("the body of `double`");
        let checked = driver.check(&owner).expect("the body to check");

        assert_eq!(
            checked
                .expr_type(body.body().body.root())
                .map(ToString::to_string),
            Some("Int".to_owned()),
        );
    }

    #[test]
    fn a_type_mistake_of_a_body_travels_with_the_diagnostics_of_the_file() {
        const SOURCE: &str = "fun main(): Unit =\n    \"text\"\n";

        let (mut driver, file) = driver_with_std("main.mlk", SOURCE);
        let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

        assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.types()[0];
        let label = diagnostic.labels.first().expect("a label");

        // A check reports a place in the HIR, and the driver turns it into the span of the text
        // the place is written at, with the words that belong under it ([ADR-0009]).
        //
        // [ADR-0009]: ../../docs/adr/0009-pass-contract.md
        assert_eq!(diagnostic.category, Category::TypeChecker);
        assert_eq!(diagnostic.code, "09");
        assert_eq!(
            diagnostic.message,
            "a value of type `String` is where a value of type `Unit` belongs",
        );
        assert_eq!(label.message, "expected `Unit`, found `String`");
        assert!(label.primary);
        assert_eq!(covered(SOURCE, Some(label.span.range)), Some("\"text\""));
    }

    #[test]
    fn a_signature_that_is_not_written_is_reported_at_its_declaration() {
        const SOURCE: &str = "fun helper() = 1\n";

        let (mut driver, file) = driver_with_std("main.mlk", SOURCE);
        let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

        assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.types()[0];
        let label = diagnostic.labels.first().expect("a label");

        // The declaration writes no result type, so there is no type to point at and the
        // declaration itself is what is marked.
        assert_eq!(diagnostic.category, Category::TypeChecker);
        assert_eq!(diagnostic.code, "01");
        assert_eq!(
            diagnostic.message,
            "`fun helper` does not declare the type of its result",
        );
        assert_eq!(label.message, "the type of this result is not written");
        assert_eq!(
            covered(SOURCE, Some(label.span.range)),
            Some("fun helper() = 1")
        );
        assert!(!diagnostic.notes.is_empty(), "the deferral is explained");
    }

    #[test]
    fn a_name_a_module_keeps_to_itself_is_told_about_as_kept_from_a_body() {
        const MAIN: &str = "\
//- /data.mlk
fun hidden(): Int = 1

//- /main.mlk
fun get(): Int = project::data::hidden()
";

        let mut driver = std_project_of(MAIN);
        let main = file(&driver, "main.mlk");
        let diagnostics = driver.diagnostics(main).expect("the file to be diagnosed");

        // The paths of a body are walked by the check and never by the resolution ([ADR-0016]),
        // so the walk that ends at a name a module keeps to itself is the check's --- and the
        // look that tells why is the one the rendering of a resolution takes.
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        assert_eq!(diagnostics.resolution().len(), 0, "{diagnostics:?}");
        assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.types()[0];

        assert_eq!(diagnostic.category, Category::TypeChecker);
        assert_eq!(diagnostic.code, "11");
        assert_eq!(
            diagnostic.message,
            "the module `project::data` holds the name `hidden` and does not show it",
        );
    }

    #[test]
    fn a_body_edit_leaves_the_signatures_and_the_other_bodies_where_they_were() {
        const SOURCE: &str = "\
//- /main.mlk
fun first(): Int = 1

fun second(): Int = 2
";

        let mut driver = std_project_of(SOURCE);
        let main = file(&driver, "main.mlk");
        let module = ModuleId(main);

        let before_types = driver
            .module_types(module)
            .expect("the signatures of the module to resolve");
        let owner = body_of(&mut driver, main, "second");
        let before = driver.check(&owner).expect("the body to check");

        driver.set_file_text(
            path("main.mlk"),
            Some("fun first(): Int = 42\n\nfun second(): Int = 2\n".to_owned()),
        );

        let after_types = driver
            .module_types(module)
            .expect("the signatures of the module to resolve");
        let owner = body_of(&mut driver, main, "second");
        let after = driver.check(&owner).expect("the body to check");

        // A body edit cannot change what a module shows, and it does not change the check of a
        // body it did not edit: a surface is resolved from signatures alone, and a check that
        // ends up equal to the one the driver held is the one it held ([ADR-0017]).
        //
        // [ADR-0017]: ../../docs/adr/0017-resolved-types.md
        assert!(
            Arc::ptr_eq(&before_types, &after_types),
            "a body edit moved the types of the module",
        );
        assert!(
            Arc::ptr_eq(&before, &after),
            "a body edit re-checked a body that did not change",
        );
    }

    #[test]
    fn the_mir_of_a_body_is_the_cfg_form_of_it() {
        let (mut driver, file) =
            driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
        let owner = body_of(&mut driver, file, "double");
        let mir = driver.mir(&owner).expect("the body to have MIR");

        // The construction is one walk of the checked body: the body is the one the entity
        // owns, a parameter is a value the body is entered with, and the CFG form holds the
        // slots the SSA construction will give definitions of their own ([ADR-0019]).
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        assert_eq!(mir.owner, owner);
        assert_eq!(mir.params.len(), 1);
        assert_eq!(mir.validate_cfg(), Ok(()));
    }

    #[test]
    fn a_second_pull_of_mir_is_the_value_the_first_one_returned() {
        let (mut driver, file) =
            driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
        let owner = body_of(&mut driver, file, "double");

        let first = driver.mir(&owner).expect("the body to have MIR");
        let second = driver.mir(&owner).expect("the body to have MIR");

        assert!(Arc::ptr_eq(&first, &second), "the slot was built twice");
    }

    #[test]
    fn a_body_that_does_not_check_clean_is_not_lowered() {
        let (mut driver, file) = driver_with_std("main.mlk", "fun main(): Unit = 1\n");
        let owner = body_of(&mut driver, file, "main");

        // A body whose check reported a mistake has no MIR: there is no meaning to lower, and
        // codegen never meets an expression whose meaning is a mistake ([ADR-0019]).
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        assert!(driver.check(&owner).is_some(), "the body to be checked");
        assert!(
            driver.mir(&owner).is_none(),
            "a body with a mistake to be lowered"
        );
        assert!(driver.mir_ssa(&owner).is_none(), "an SSA form of a mistake");
    }

    #[test]
    fn a_literal_out_of_the_range_of_int_is_the_checks_mistake() {
        const SOURCE: &str = "fun big(): Int = 1099511627776\n";

        let (mut driver, file) = driver_with_std("main.mlk", SOURCE);
        let owner = body_of(&mut driver, file, "big");
        let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

        // The range of a literal is the meaning of the type it is written with, so the check is
        // what reports it, and the construction of MIR reads it as an invariant ([ADR-0018]).
        //
        // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
        assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

        let diagnostic = &diagnostics.types()[0];
        let label = diagnostic.labels.first().expect("a label");

        assert_eq!(diagnostic.category, Category::TypeChecker);
        assert_eq!(diagnostic.code, "13");
        assert_eq!(
            diagnostic.message,
            "the integer literal `1099511627776` is outside the 31-bit range of `Int`",
        );
        assert!(label.primary);
        assert_eq!(
            covered(SOURCE, Some(label.span.range)),
            Some("1099511627776")
        );

        // A body with a mistake has no MIR at all ([ADR-0019]).
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        assert!(
            driver.mir(&owner).is_none(),
            "a mistaken body to be lowered"
        );
    }

    #[test]
    fn a_file_that_did_not_parse_has_no_mir() {
        // The body of a module that broke the parser may hold an expression that is not there,
        // and MIR has no meaning for one: the driver lowers nothing for such a file.
        let (mut driver, file) = driver_with_std("main.mlk", "fun main(): Unit =\n");
        let owner = body_of(&mut driver, file, "main");

        assert!(driver.parse(file).is_some_and(|parse| parse.has_errors()));
        assert!(driver.mir(&owner).is_none(), "MIR of a file that broke");
        assert!(driver.mir_ssa(&owner).is_none(), "SSA of a file that broke");
    }

    #[test]
    fn the_ssa_form_of_a_body_is_reached_by_a_pass() {
        let (mut driver, file) =
            driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
        let owner = body_of(&mut driver, file, "double");

        let cfg = driver.mir(&owner).expect("the body to have MIR");
        let ssa = driver
            .mir_ssa(&owner)
            .expect("the body to have an SSA form");
        let again = driver
            .mir_ssa(&owner)
            .expect("the body to have an SSA form");

        assert!(Arc::ptr_eq(&ssa, &again), "the slot was built twice");
        assert!(
            !Arc::ptr_eq(&cfg, &ssa),
            "the SSA form is a pass over the CFG form",
        );
        assert_eq!(cfg.validate_cfg(), Ok(()));
        assert_eq!(ssa.validate_ssa(), Ok(()));
    }

    #[test]
    fn a_body_edit_leaves_the_mir_of_the_bodies_that_did_not_change_where_they_were() {
        const SOURCE: &str = "\
//- /main.mlk
fun first(): Int = 1

fun second(): Int = 2
";

        let mut driver = std_project_of(SOURCE);
        let main = file(&driver, "main.mlk");

        let owner = body_of(&mut driver, main, "second");
        let before = driver.mir(&owner).expect("the body to have MIR");
        let before_ssa = driver
            .mir_ssa(&owner)
            .expect("the body to have an SSA form");

        driver.set_file_text(
            path("main.mlk"),
            Some("fun first(): Int = 3\n\nfun second(): Int = 2\n".to_owned()),
        );

        let owner = body_of(&mut driver, main, "second");
        let after = driver.mir(&owner).expect("the body to have MIR");
        let after_ssa = driver
            .mir_ssa(&owner)
            .expect("the body to have an SSA form");

        // A body edit that does not move a body cannot change its MIR: the lowering of a body
        // that ends up equal to the one the driver held is the one it held, spans included
        // ([ADR-0019]).
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        assert!(
            Arc::ptr_eq(&before, &after),
            "a body edit re-lowered a body that did not move",
        );
        assert!(
            Arc::ptr_eq(&before_ssa, &after_ssa),
            "a body edit rebuilt the SSA form of a body that did not move",
        );
    }

    #[test]
    fn a_driver_without_the_library_checks_no_types() {
        // The classes of the language are the library's declaration, and a driver whose host
        // recorded no library has none: what a literal is is nothing the checker can say.
        let (mut driver, file) = driver_with("main.mlk", "fun main(): Int = 1\n");

        assert!(driver.module_types(ModuleId(file)).is_none());
        assert!(
            driver
                .diagnostics(file)
                .expect("the file to be diagnosed")
                .types()
                .is_empty()
        );
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
        let diagnostic = diagnostics
            .iter()
            .next()
            .expect("the parse to report something");

        assert_eq!(diagnostic.level, Level::Error);
        assert_eq!(diagnostic.category, Category::Parser);
        assert_eq!(diagnostic.labels.first().expect("a label").span.file, file);
    }

    #[test]
    fn the_diagnostics_are_converted_once() {
        let (mut driver, file) = driver_with("main.mlk", BROKEN);

        let first = driver.diagnostics(file).expect("the file to be parsed");
        let second = driver.diagnostics(file).expect("the file to be parsed");

        // Nothing is rendered again: a part of the diagnostics is the value the stage left.
        assert!(
            Arc::ptr_eq(first.parse(), second.parse())
                && Arc::ptr_eq(first.lowering(), second.lowering())
                && Arc::ptr_eq(first.resolution(), second.resolution()),
            "the conversion ran twice",
        );
    }

    #[test]
    fn the_diagnostics_follow_the_text_and_the_old_ones_stay_as_they_were() {
        let (mut driver, file) = driver_with_std("main.mlk", BROKEN);
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
            7,
            "six lines of source and the line the trailing break makes"
        );
        assert_eq!(
            &MODULE[index.line_range(5).expect("a sixth line")],
            "    println-int(x + 20)"
        );
        assert_eq!(index.line_col(MODULE.text_len()), LineCol {
            line: 6,
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
