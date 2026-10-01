//! What the names of a module denote: the interfaces, the indexes, the resolutions, and the map.

use std::{collections::BTreeMap, sync::Arc};

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::{Interface, ModuleId, ModuleIndex, ProjectDefMap, ProjectId};
use mlkc_resolve::{
    Closure, Resolution, ResolveDeps, ResolveDiag, ResolveError, hidden_name, resolve_module,
};
use mlkc_span::Span;

use super::{
    DefMapSlot, Driver, InterfaceSlot, Lowered, ModuleIndexSlot, Pass, ResolutionDiagnosticsSlot,
    ResolutionSlot, Unit, entries_are_the_same,
};

impl Driver {
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
        let unit = Unit::Module(module);
        let held = self.interfaces.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
        {
            self.stats.consulted(Pass::Interface, &unit, true, true);

            return Some(slot.value.clone());
        }

        self.stats
            .consulted(Pass::Interface, &unit, held.is_some(), false);

        let started = self.ticking();
        let value = Arc::new(Interface::of(lowered.item_tree()));

        self.stats.ran(Pass::Interface, &unit, self.clock, started);

        // An interface equal to the one the driver holds is the value it holds: everything
        // keyed by the interface stays where it was ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match held {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::Interface, &unit);

                slot.value.clone()
            },
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
        let unit = Unit::Project(project.clone());

        if let Some(slot) = held
            && entries_are_the_same(&slot.interfaces, &interfaces)
        {
            self.stats.consulted(Pass::ModuleIndex, &unit, true, true);

            return Some(slot.value.clone());
        }

        self.stats
            .consulted(Pass::ModuleIndex, &unit, held.is_some(), false);

        let started = self.ticking();
        let mut index = ModuleIndex::new(project.clone());

        for (module, interface) in &interfaces {
            index.insert(*module, interface.path());
        }

        self.stats
            .ran(Pass::ModuleIndex, &unit, self.clock, started);

        let value = Arc::new(index);
        let value = match held {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::ModuleIndex, &unit);

                slot.value.clone()
            },
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
        let unit = Unit::Module(module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && slot.closure.reads_the_same_as(&closure)
        {
            self.stats.consulted(Pass::Resolution, &unit, true, true);

            return Some(slot.value.clone());
        }

        self.stats
            .consulted(Pass::Resolution, &unit, held.is_some(), false);

        let started = self.ticking();
        let value = Arc::new(resolve_module(module, lowered.item_tree(), &ResolveDeps {
            graph,
            closure: closure.clone(),
        }));

        self.stats.ran(Pass::Resolution, &unit, self.clock, started);

        // A resolution equal to the one the driver holds is the value it holds: a reader that
        // came to the same entities came to nothing new ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match held {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::Resolution, &unit);

                slot.value.clone()
            },
            _ => value,
        };

        self.resolutions.insert(module, ResolutionSlot {
            lowered,
            closure,
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
        let unit = Unit::Project(project.clone());

        if let Some(slot) = held
            && entries_are_the_same(&slot.resolutions, &resolutions)
        {
            self.stats.consulted(Pass::DefMap, &unit, true, true);

            return Some(slot.value.clone());
        }

        self.stats
            .consulted(Pass::DefMap, &unit, held.is_some(), false);

        let started = self.ticking();
        let mut map = ProjectDefMap::default();

        for (module, resolution) in &resolutions {
            map.set(*module, Arc::clone(resolution.scope()));
        }

        self.stats.ran(Pass::DefMap, &unit, self.clock, started);

        let value = Arc::new(map);
        let value = match held {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::DefMap, &unit);

                slot.value.clone()
            },
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
    pub(super) fn indexes_of(&mut self, module: ModuleId) -> BTreeMap<ProjectId, Arc<ModuleIndex>> {
        let mut indexes = BTreeMap::new();

        for project in self.read_projects(module) {
            if let Some(index) = self.module_index(&project) {
                indexes.insert(project, index);
            }
        }

        indexes
    }

    /// The diagnostics of the resolution of `module`, in the shape a host renders.
    ///
    /// A resolution reports places in the HIR, and where a place is written is what the driver
    /// holds, so the rendering is the driver's. It is a value of its own, and a resolution or
    /// an HIR that did not change is a value the driver already holds.
    pub(super) fn resolution_diagnostics(&mut self, module: ModuleId) -> Option<Arc<[Diagnostic]>> {
        let lowered = self.lower(module.0)?;
        let resolution = self.resolution(module)?;

        let held = self.resolution_diagnostics.get(&module);
        let existed = held.is_some();
        let held = held.filter(|slot| {
            Arc::ptr_eq(&slot.resolution, &resolution) && Arc::ptr_eq(&slot.lowered, &lowered)
        });
        let unit = Unit::Module(module);

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
                self.stats
                    .consulted(Pass::ResolutionDiagnostics, &unit, true, true);

                return Some(self.resolution_diagnostics[&module].value.clone());
            }
        }

        self.stats
            .consulted(Pass::ResolutionDiagnostics, &unit, existed, false);

        let started = self.ticking();
        let mut looked = BTreeMap::new();
        let mut rendered = Vec::with_capacity(resolution.diagnostics().len());

        for diagnostic in resolution.diagnostics().iter() {
            rendered.push(self.rendered(diagnostic, module, &lowered, &mut looked));
        }

        self.stats
            .ran(Pass::ResolutionDiagnostics, &unit, self.clock, started);

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

    /// The error a walk should have reported instead, when the name it could not find is a name
    /// a module keeps to itself ([`hidden_name`]).
    ///
    /// The look is taken only for a name a walk could not find, and what it read is what
    /// a caller keys the rendered diagnostics by: a module that resolved cleanly reads no item
    /// tree of another module, and the key of its diagnostics is not widened by this.
    pub(super) fn hidden(
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
}
