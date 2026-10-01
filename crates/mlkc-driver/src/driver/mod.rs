//! The driver's state: the table of slots, the counters of what the passes did, the clock the
//! host gave it, the guards around a pass, and the report a bug leaves.

mod check;
mod diagnostics;
mod host;
mod link;
mod lower;
mod mir;
mod mir_module;
mod parse;
mod resolve;
mod stats;

#[cfg(test)]
mod tests;

use std::{
    collections::BTreeMap,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, OnceLock},
};

use mlkc_codegen_wasm::ModuleMir;
use mlkc_diagnostics::{Diagnostic, Ice};
use mlkc_hir_def::{
    BodyEntityLoc, Interface, ItemLocLike, ModuleId, ModuleIndex, ProjectDefMap, ProjectGraph,
    ProjectId,
};
use mlkc_hir_ty::{CheckedBody, ModuleTypes};
use mlkc_line_index::LineIndex;
use mlkc_mir::Body as MirBody;
use mlkc_resolve::{Closure, Resolution};
use mlkc_rowan::NodeCache;
use mlkc_typeck::{Builtins, TypeDiag};
use mlkc_vfs::{FileId, FileVersion, Vfs};
use rustc_hash::FxHashMap;

pub use self::{
    diagnostics::Diagnostics,
    host::StdFile,
    link::{LinkError, LinkPlan},
    lower::{Lowered, ModuleBody},
    parse::Parse,
    stats::{Clock, Pass, Stats, Tally, Unit, system_clock},
};

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
    /// The module of every module that has been asked for one.
    mir_modules: FxHashMap<ModuleId, ModuleMirSlot>,
    /// The plan of every project that has been asked to link.
    links: FxHashMap<ProjectId, LinkSlot>,
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
    /// What the passes did since a host last took the counters.
    stats: Stats,
    /// The clock the host gave the driver, if it gave it one.
    ///
    /// The driver reads no clock of its own ([`Clock`]), so a host that wants passes timed
    /// hands it one; a host that does not leaves it out, and the work is counted without a time.
    clock: Option<Clock>,
    /// The first internal compiler exception a pass raised, and what the driver was computing.
    ice: Option<Arc<IceReport>>,
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

/// The module of one module, and what building it read ([ADR-0021]).
///
/// The key of the slot is what the stage read: the HIR the functions and their names are read
/// from, the types of the module's entities, the classes of the language, the SSA form of every
/// body, and the type surface of every module a callee belongs to --- an import carries the
/// signature of the function it names ([ADR-0021]).
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
struct ModuleMirSlot {
    /// The HIR the functions and their names were read from.
    lowered: Arc<Lowered>,
    /// The types of the module's entities, which say what each function takes and gives back.
    types: Arc<ModuleTypes>,
    /// The classes of the language the module was read with.
    builtins: Builtins,
    /// The SSA form of every body of the module, in the order the module declares them.
    bodies: Vec<Arc<MirBody>>,
    /// The type surface of every module a callee belongs to, by module.
    callees: BTreeMap<ModuleId, Arc<ModuleTypes>>,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<ModuleMir>,
}

/// The plan of one project, and the modules it was built from ([ADR-0021]).
///
/// A plan is a function of the modules of the program and of nothing else: every import a
/// compiled module carries is a name the module of the plan already has.
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
struct LinkSlot {
    /// The module of every module of the program, by module, as the plan was built.
    modules: BTreeMap<ModuleId, Arc<ModuleMir>>,
    /// The value, retained so that the driver can hand it out and compare it later.
    value: Arc<LinkPlan>,
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

/// The report of an internal compiler exception: the bug, and what the driver was computing.
///
/// A pass that raises one was handed input the earlier stages should have kept out
/// ([`mlkc_diagnostics::Ice`]), and the driver is what knows which module and which body it was
/// asked about: the report is the exception with that context. It is held rather than raised ---
/// [`Driver::ice`] is how a host tells a person to file it --- and the pull that met the bug
/// answered as if the value were not there, so the rest of the compiler keeps working.
#[derive(Debug)]
pub struct IceReport {
    /// The exception itself: what the pass assumed, where it was raised, and its backtrace.
    ice: Ice,
    /// What the driver was computing when the pass raised it.
    context: String,
}

impl IceReport {
    /// A report of `ice`, raised while the driver was doing `context`.
    fn new(ice: Ice, context: String) -> Self {
        Self { ice, context }
    }

    /// The exception itself.
    pub fn ice(&self) -> &Ice {
        &self.ice
    }

    /// What the driver was computing: the pass, the body, and where it is written.
    pub fn context(&self) -> &str {
        &self.context
    }
}

impl fmt::Display for IceReport {
    /// The report as a person reads it: what the driver was doing, and the exception itself.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "internal compiler error: while {}, the compiler bugged:",
            self.context,
        )?;
        write!(f, "{}", self.ice)
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

impl Driver {
    /// The first internal compiler exception a pass raised, if one has.
    ///
    /// `None` is a healthy compiler. `Some` means a pull answered with no value where a pass
    /// bugged instead of computing one: the report is what a host shows, and a host that would
    /// rather stop can stop on it. The driver keeps the first and never overwrites it --- what
    /// follows a bug is its wake.
    pub fn ice(&self) -> Option<Arc<IceReport>> {
        self.ice.clone()
    }

    /// What the passes did since a host last took this, and a clean slate again.
    ///
    /// The counters are the driver's account of its own incrementality: what it handed over
    /// from a slot, what it had to read again, what it kept although it read again, what went,
    /// and --- for a host that gave the driver a clock --- how long the passes it ran took. What
    /// a host reads them by is the pass and the unit ([`Stats`]).
    pub fn take_stats(&mut self) -> Stats {
        self.stats.take()
    }

    /// Gives the driver a clock to measure the passes it runs by ([`Clock`]).
    ///
    /// The clock is the host's, and it stays with the driver: taking the counters takes what
    /// the passes did, not the clock they were read against.
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = Some(clock);
    }

    /// The name a host knows a unit by: where a file or a module is written, what a project is
    /// called, and where the body of an entity is.
    pub fn unit_name(&self, unit: &Unit) -> String {
        match unit {
            Unit::File(file) => self.file_path(*file).to_string(),
            Unit::Module(module) => self.file_path(module.0).to_string(),
            Unit::Project(project) => project.as_str().to_owned(),
            Unit::Body(owner) => {
                format!("{}: {}", self.file_path(owner.module().0), body_name(owner),)
            },
        }
    }

    /// Runs a pass that may bug, holding the report and answering `None` instead of unwinding.
    ///
    /// A slot is written only when its value is complete, so a pull a panic travels out of leaves
    /// the table as valid as it found it ([ADR-0008]), and its caller reads the answer as "no
    /// value". `context` is what the driver was computing, read only when a bug happens.
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    fn guarded<T>(
        &mut self,
        context: impl FnOnce(&Self) -> String,
        run: impl FnOnce() -> T,
    ) -> Option<T> {
        match catch_unwind(AssertUnwindSafe(run)) {
            Ok(value) => Some(value),
            Err(payload) => {
                let report = Arc::new(IceReport::new(Ice::of(payload), context(self)));

                if self.ice.is_none() {
                    self.ice = Some(report);
                }

                None
            },
        }
    }

    /// The body a report is about, as a person reads it: the name of the entity that owns it and
    /// the path of the file it is written in.
    fn body_context(&self, owner: &BodyEntityLoc) -> String {
        format!(
            "the body of `{}` in `{}`",
            body_name(owner),
            self.file_path(owner.module().0),
        )
    }

    /// The reading of the host's clock, for a pass that is about to run.
    ///
    /// `None` when the host gave the driver no clock, which is what a host that reads the work
    /// of the passes rather than the time they took leaves behind.
    fn ticking(&self) -> Option<f64> {
        self.clock.map(|clock| clock())
    }

    /// The value of a slot keyed by the version of a file,
    /// computed when the slot is missing or stale and remembered otherwise.
    ///
    /// The parse is the one stage whose slot does not have this shape: it keeps the green
    /// nodes of the parse next to its value ([`ParseSlot`]), and that is what its own pull is
    /// written out for.
    fn text_derived<T: ?Sized>(
        stats: &mut Stats,
        clock: Option<Clock>,
        pass: Pass,
        slots: &mut FxHashMap<FileId, TextSlot<T>>,
        file: FileId,
        version: FileVersion,
        build: impl FnOnce() -> Option<Arc<T>>,
    ) -> Option<Arc<T>> {
        let unit = Unit::File(file);
        let held = slots.get(&file);

        if let Some(slot) = held
            && slot.version == version
        {
            stats.consulted(pass, &unit, true, true);

            return Some(slot.value.clone());
        }

        stats.consulted(pass, &unit, held.is_some(), false);

        let started = clock.map(|clock| clock());
        let value = build();

        stats.ran(pass, &unit, clock, started);

        let Some(value) = value else {
            // There is no input left to describe, so the slot goes.
            // The next push of this file builds it again from nothing.
            stats.dropped(pass, &unit, slots.remove(&file).is_some() as usize);

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

        let unit = Unit::Project(project.clone());

        self.stats.dropped(
            Pass::ModuleIndex,
            &unit,
            self.module_indexes.remove(project).is_some() as usize,
        );
        self.stats.dropped(
            Pass::DefMap,
            &unit,
            self.def_maps.remove(project).is_some() as usize,
        );
        self.stats.dropped(
            Pass::Link,
            &unit,
            self.links.remove(project).is_some() as usize,
        );
    }

    /// Drops the values derived from one module: its HIR, its interface, its resolution, its
    /// type surface, the checks and the MIR of its bodies, and the diagnostics that were rendered
    /// from them.
    ///
    /// A module that changed project or text is read again; what reads this module is not
    /// dropped with it, and sees the new value when it is read again. A value of a body goes by
    /// its own name: what a host reads of a drop is which body it was, and a count of them is
    /// not.
    fn invalidate_module(&mut self, module: ModuleId) {
        let file = Unit::File(module.0);
        let named = Unit::Module(module);

        self.stats.dropped(
            Pass::Lower,
            &file,
            self.lowered.remove(&module.0).is_some() as usize,
        );
        self.stats.dropped(
            Pass::Interface,
            &named,
            self.interfaces.remove(&module).is_some() as usize,
        );
        self.stats.dropped(
            Pass::Resolution,
            &named,
            self.resolutions.remove(&module).is_some() as usize,
        );
        self.stats.dropped(
            Pass::ResolutionDiagnostics,
            &named,
            self.resolution_diagnostics.remove(&module).is_some() as usize,
        );
        self.stats.dropped(
            Pass::Signatures,
            &named,
            self.signatures.remove(&module).is_some() as usize,
        );
        self.stats.dropped(
            Pass::TypeDiagnostics,
            &named,
            self.type_diagnostics.remove(&module).is_some() as usize,
        );

        let bodies: Vec<BodyEntityLoc> = self
            .checks
            .keys()
            .filter(|owner| owner.module() == module)
            .cloned()
            .collect();
        for owner in bodies {
            self.checks.remove(&owner);
            self.stats.dropped(Pass::Check, &Unit::Body(owner), 1);
        }

        let bodies: Vec<BodyEntityLoc> = self
            .mirs
            .keys()
            .filter(|owner| owner.module() == module)
            .cloned()
            .collect();
        for owner in bodies {
            self.mirs.remove(&owner);
            self.stats.dropped(Pass::Mir, &Unit::Body(owner), 1);
        }

        let bodies: Vec<BodyEntityLoc> = self
            .ssas
            .keys()
            .filter(|owner| owner.module() == module)
            .cloned()
            .collect();
        for owner in bodies {
            self.ssas.remove(&owner);
            self.stats.dropped(Pass::Ssa, &Unit::Body(owner), 1);
        }

        self.stats.dropped(
            Pass::MirModule,
            &named,
            self.mir_modules.remove(&module).is_some() as usize,
        );

        // A plan is compiled from the modules of its project, so it goes with the first of them
        // that changes; what a host reads of the drop is the project it was of.
        if let Some(project) = self.projects.project_of(module).cloned() {
            self.stats.dropped(
                Pass::Link,
                &Unit::Project(project.clone()),
                self.links.remove(&project).is_some() as usize,
            );
        }
    }
}

/// What a body is called by: the name of the entity that owns it, or the entity itself.
fn body_name(owner: &BodyEntityLoc) -> String {
    owner
        .item
        .name()
        .map_or_else(|| format!("{:?}", owner.item), ToString::to_string)
}
