//! The cycles of the import graph: the modules that name each other, round by round.
//!
//! A module is instantiated after the modules it names, so a cycle of names has no order to be
//! instantiated in: the imports of a program form no cycle, and a cycle is a mistake of the
//! source, reported at the name that closes it ([ADR-0016]).
//!
//! The graph is read as it was gathered ([`crate::named_modules`]), and what is looked for is
//! the cycles of it: the modules that reach each other, which is what a strongly connected
//! component of the graph is. Every module of a cycle is reported, so a reader of any of the
//! files of it is told what is wrong ([ADR-0004]).
//!
//! [ADR-0004]: ../../docs/adr/0004-module-system.md
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

use std::collections::{BTreeMap, BTreeSet};

use mlkc_diagnostics::{Category, DiagKind, Level};
use mlkc_hir_def::{ModuleId, PlainPathId, ResolveError};

use crate::{NamedModules, NamedPlace};

/// The cycles of the names of a project's modules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportCycles {
    /// The cycles, in the order of the modules they hold.
    cycles: Vec<ImportCycle>,
    /// What a cycle found, one report for each module that takes part in one.
    diagnostics: Vec<CycleDiag>,
}

/// One cycle of the names of a project: the modules that reach each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCycle {
    /// The modules of the cycle, in the order of their ids.
    modules: Vec<ModuleId>,
}

/// A name that takes part in a cycle, and where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleDiag {
    /// The module the name is written in.
    module: ModuleId,
    /// What was found: the module the name reaches, which is in the cycle with it.
    error: ResolveError,
    /// Where the name is written.
    place: NamedPlace,
}

impl ImportCycles {
    /// The cycles, in the order of the modules they hold.
    pub fn cycles(&self) -> &[ImportCycle] {
        &self.cycles
    }

    /// What a cycle found, one report for each module that takes part in one.
    pub fn diagnostics(&self) -> &[CycleDiag] {
        &self.diagnostics
    }
}

impl ImportCycle {
    /// The modules of the cycle, in the order of their ids.
    pub fn modules(&self) -> &[ModuleId] {
        &self.modules
    }
}

impl CycleDiag {
    /// The module the name is written in.
    pub fn module(&self) -> ModuleId {
        self.module
    }

    /// What was found: the module the name reaches.
    pub fn error(&self) -> &ResolveError {
        &self.error
    }

    /// Where the name is written.
    pub fn place(&self) -> &NamedPlace {
        &self.place
    }
}

impl DiagKind for CycleDiag {
    fn level(&self) -> Level {
        Level::Error
    }

    /// A cycle of names is found by the resolution of the modules of a project, which is the
    /// global half of name resolution ([ADR-0005]).
    ///
    /// [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md
    fn category(&self) -> Category {
        Category::Resolver
    }

    fn code(&self) -> &'static str {
        "11"
    }
}

/// Finds the cycles of the names of the modules of a project.
///
/// `named` is every module of the project with the modules its names reach — a name that reaches
/// a module of another project is not a cycle, since no name of that project can come back: the
/// projects a project names are the ones it depends on, and the graph of them is acyclic. `paths`
/// is the path each module is called by, which is what a message names it with.
///
/// # Panics
///
/// Panics when a module of a cycle reaches no module of the cycle. Every module of a cycle
/// reaches itself through it, which is what the cycle was found by, so the panic is a bug of the
/// compiler and not a case a program can raise.
pub fn import_cycles<'a>(
    named: impl IntoIterator<Item = (ModuleId, PlainPathId, &'a NamedModules)>,
) -> ImportCycles {
    let mut names: BTreeMap<ModuleId, PlainPathId> = BTreeMap::new();
    let mut edges: BTreeMap<ModuleId, Vec<(ModuleId, NamedPlace)>> = BTreeMap::new();

    for (module, path, reached) in named {
        names.insert(module, path);

        // A module that names itself is not a cycle: what a module declares is read by its own
        // bodies, and nothing has to stand before it.
        edges.insert(
            module,
            reached
                .iter()
                .filter(|(reached, _)| *reached != module)
                .map(|(reached, place)| (reached, place.clone()))
                .collect(),
        );
    }

    // The names of a project reach a module of it, or none: a name of another project cannot come
    // back --- the projects a project names are the ones it depends on, and the graph of them is
    // acyclic --- so the names that leave the project are dropped before the cycles are looked
    // for.
    for edges in edges.values_mut() {
        edges.retain(|(reached, _)| names.contains_key(reached));
    }

    let reversed = reversed(&edges);
    let mut cycles = Vec::new();
    let mut diagnostics = Vec::new();
    let mut assigned: BTreeSet<ModuleId> = BTreeSet::new();

    for module in names.keys() {
        if assigned.contains(module) {
            continue;
        }

        let mut cycle = cycle_of(*module, &edges, &reversed);
        cycle.sort_unstable();

        if cycle.len() < 2 {
            assigned.insert(*module);
            continue;
        }

        assigned.extend(cycle.iter().copied());

        for member in &cycle {
            let (reached, place) = edges[member]
                .iter()
                .find(|(reached, _)| cycle.contains(reached))
                .expect("a module of a cycle to name a module of it");

            diagnostics.push(CycleDiag {
                module: *member,
                error: ResolveError::ImportCycle {
                    module: names[reached].clone(),
                },
                place: place.clone(),
            });
        }

        cycles.push(ImportCycle { modules: cycle });
    }

    ImportCycles {
        cycles,
        diagnostics,
    }
}

/// The modules that name each other with `module`, if any of them do.
///
/// Two modules are one cycle when each of them reaches the other: one is a module of the cycle
/// when it is reached from `module` and reaches it in turn.
fn cycle_of(
    module: ModuleId,
    edges: &BTreeMap<ModuleId, Vec<(ModuleId, NamedPlace)>>,
    reversed: &BTreeMap<ModuleId, BTreeSet<ModuleId>>,
) -> Vec<ModuleId> {
    let forward = reachable(module, edges);
    let backward = reachable_reversed(module, reversed);

    forward
        .into_iter()
        .filter(|reached| backward.contains(reached))
        .collect()
}

/// The modules `module` reaches, itself included.
fn reachable(
    module: ModuleId,
    edges: &BTreeMap<ModuleId, Vec<(ModuleId, NamedPlace)>>,
) -> BTreeSet<ModuleId> {
    let mut reached = BTreeSet::new();
    let mut pending = vec![module];

    while let Some(module) = pending.pop() {
        if !reached.insert(module) {
            continue;
        }

        if let Some(edges) = edges.get(&module) {
            pending.extend(edges.iter().map(|(reached, _)| *reached));
        }
    }

    reached
}

/// The modules that reach `module`, itself included.
fn reachable_reversed(
    module: ModuleId,
    reversed: &BTreeMap<ModuleId, BTreeSet<ModuleId>>,
) -> BTreeSet<ModuleId> {
    let mut reached = BTreeSet::new();
    let mut pending = vec![module];

    while let Some(module) = pending.pop() {
        if !reached.insert(module) {
            continue;
        }

        if let Some(names) = reversed.get(&module) {
            pending.extend(names.iter().copied());
        }
    }

    reached
}

/// The names, read the other way round: the modules that name each one.
fn reversed(
    edges: &BTreeMap<ModuleId, Vec<(ModuleId, NamedPlace)>>,
) -> BTreeMap<ModuleId, BTreeSet<ModuleId>> {
    let mut reversed: BTreeMap<ModuleId, BTreeSet<ModuleId>> = BTreeMap::new();

    for (module, edges) in edges {
        for (reached, _) in edges {
            reversed.entry(*reached).or_default().insert(*module);
        }
    }

    reversed
}
