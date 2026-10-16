//! The import graph of a project: the modules its names reach, and the cycles of them.
//!
//! A module is read after the modules its names reach, so the names of a program form no cycle:
//! what a name at the end of a cycle reaches is nothing, and the cycle is a mistake of the
//! source ([ADR-0016]). The graph is what the names of every module of a project say, and the
//! cycles of it are the strongly connected components of it, each reported at a name that is in
//! one ([`mlkc_resolve`]).
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::{
    Body, BodyEntityLoc, Expr, ExprId, ItemLoc, ModuleId, PathId, PlainPathId, ProjectId,
};
use mlkc_resolve::{
    CycleDiag, ImportCycles, NamedModules, NamedPlace, ResolveDeps, closure, import_cycles,
    named_modules,
};
use mlkc_span::Span;
use mlkc_syntax::TextRange;

use super::{
    Driver, ImportCyclesSlot, Lowered, ModuleBody, NamedModulesSlot, Pass, Unit,
    entries_are_the_same,
};

impl Driver {
    /// The modules the names of `module` reach, and where each of them is named.
    ///
    /// The names are every path the module writes: its imports, the types its declarations
    /// write, and the paths of its bodies. The key of the slot is what a check reads --- the HIR,
    /// the resolution, and the wider closure of the check --- and the value is the whole of what
    /// the graph of the project is built from, so a body edit that moves no name moves no
    /// value ([ADR-0016]).
    ///
    /// `None` when there is nothing to read the names in: see [`Driver::lower`].
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub fn named_modules(&mut self, module: ModuleId) -> Option<Arc<NamedModules>> {
        let lowered = self.lower(module.0)?;
        let resolved = self.resolution(module)?;
        let graph = Arc::clone(&self.projects);
        let indexes = self.indexes_of(module);

        // The closure of the names is the closure of a check: a path of a body may name a module
        // that no signature names, and a name of a body is a name of the graph ([ADR-0017]).
        //
        // [ADR-0017]: ../../docs/adr/0017-resolved-types.md
        let closure = closure::of_check(
            module,
            lowered.item_tree(),
            lowered.bodies().iter().map(|body| &body.body().body),
            &graph,
            &indexes,
            &mut |module| self.interface(module),
        );

        let held = self.named_modules.get(&module);
        let unit = Unit::Module(module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && Arc::ptr_eq(&slot.resolution, resolved.resolution())
            && slot.closure.reads_the_same_as(&closure)
        {
            self.stats.consulted(Pass::NamedModules, &unit, true, true);

            return Some(slot.value.clone());
        }

        self.stats
            .consulted(Pass::NamedModules, &unit, held.is_some(), false);

        let started = self.ticking();
        let bodies: Vec<(BodyEntityLoc, &Body)> = lowered
            .bodies()
            .iter()
            .map(|body| (body.owner().clone(), &body.body().body))
            .collect();
        let deps = ResolveDeps {
            graph,
            closure: closure.clone(),
        };
        let value = Arc::new(named_modules(
            module,
            lowered.item_tree(),
            bodies,
            resolved.resolution(),
            &deps,
        ));

        self.stats
            .ran(Pass::NamedModules, &unit, self.clock, started);

        // A value equal to the one the driver holds is the value it holds: a body edit that
        // moves no name the module reaches does not move what the cycles are built from
        // ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match held {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::NamedModules, &unit);

                slot.value.clone()
            },
            _ => value,
        };

        self.named_modules.insert(module, NamedModulesSlot {
            lowered,
            resolution: Arc::clone(resolved.resolution()),
            closure,
            value: value.clone(),
        });

        Some(value)
    }

    /// The cycles of the names of the modules of `project`.
    ///
    /// The graph is read from the named modules of every module of the project, which are values
    /// of their own: a body edit that moves no name is read again by the graph of one module and
    /// no further, and the cycles of the project are the value the driver holds until a name of
    /// it moves --- or until a module of it is called by another path, which is what a report
    /// names it by ([ADR-0008]).
    ///
    /// `None` when the graph holds no such project.
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    pub fn import_cycles(&mut self, project: &ProjectId) -> Option<Arc<ImportCycles>> {
        let index = self.module_index(project)?;
        let mut named: BTreeMap<ModuleId, Arc<NamedModules>> = BTreeMap::new();
        let mut paths: BTreeMap<ModuleId, PlainPathId> = BTreeMap::new();

        for (_, module) in index.iter() {
            // The path a module is called by is the one its interface records, and it is what a
            // message names the module with: a name a cycle is reported at reaches a module, and
            // a reader is told which one.
            let path = self.interface(module)?.path().clone();

            named.insert(module, self.named_modules(module)?);
            paths.insert(module, path);
        }

        let held = self.import_cycles.get(project);
        let unit = Unit::Project(project.clone());

        if let Some(slot) = held
            && entries_are_the_same(&slot.named, &named)
            && slot.paths == paths
        {
            self.stats.consulted(Pass::ImportCycles, &unit, true, true);

            return Some(slot.value.clone());
        }

        self.stats
            .consulted(Pass::ImportCycles, &unit, held.is_some(), false);

        let started = self.ticking();
        let value =
            Arc::new(import_cycles(paths.iter().map(|(module, path)| {
                (*module, path.clone(), named[module].as_ref())
            })));

        self.stats
            .ran(Pass::ImportCycles, &unit, self.clock, started);

        let value = match held {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::ImportCycles, &unit);

                slot.value.clone()
            },
            _ => value,
        };

        self.import_cycles
            .insert(project.clone(), ImportCyclesSlot {
                named,
                paths,
                value: value.clone(),
            });

        Some(value)
    }

    /// The diagnostic a host renders of a name that takes part in a cycle.
    ///
    /// The place is the name the module wrote --- an import, a type, or a path of a body --- and
    /// where it is written is what the driver holds.
    pub(super) fn rendered_cycle(
        &self,
        diagnostic: &CycleDiag,
        file: ModuleId,
        lowered: &Lowered,
    ) -> Diagnostic {
        let error = diagnostic.error();

        // A place the driver wrote no range for has nowhere to point at: what the cycle is is
        // still what a host is told about, and it is told without a place.
        let Some(range) = named_place_range(lowered, diagnostic.place()) else {
            return Diagnostic::from_kind(diagnostic, error.message());
        };

        Diagnostic::from_kind(diagnostic, error.message())
            .with_primary(Span::new(file.0, range), "")
    }
}

/// Where the place a name that takes part in a cycle is written, if the driver found it.
///
/// An import, a type, and a path of a body are written in three different places, and the HIR of
/// the module holds where each of them is: an import and a declaration by their position in the
/// module, a type by the place the declaration writes it at, and a path of a body by the
/// expression that writes it.
fn named_place_range(lowered: &Lowered, place: &NamedPlace) -> Option<TextRange> {
    match place {
        NamedPlace::Import(import) => lowered.item_range(&ItemLoc::Use(import.clone())),
        NamedPlace::Declared { item, place } => {
            match place {
                Some(place) => {
                    lowered
                        .type_range(item, *place)
                        .or_else(|| lowered.item_range(item))
                },
                None => lowered.item_range(item),
            }
        },
        NamedPlace::Body { owner, path } => {
            lowered
                .bodies()
                .iter()
                .find(|body| body.owner() == owner)
                .and_then(|body| written_path(body, *path))
                .or_else(|| lowered.item_range(&ItemLoc::from(owner.item.clone())))
        },
    }
}

/// Where a path of a body is written: the expression that writes it.
///
/// A path a body writes is the expression of a `PathExpr`, and the source map of the body is
/// what says where that expression stands.
fn written_path(body: &ModuleBody, path: PathId) -> Option<TextRange> {
    let lowered = body.body();
    let (expr, _): (ExprId, &Expr) = lowered
        .body
        .exprs()
        .iter()
        .find(|(_, expr)| matches!(expr, Expr::Path(written) if *written == path))?;

    lowered.source_map.expr(expr)
}
