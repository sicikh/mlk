//! What a resolution reads of the rest of the project: the modules its walk reaches, and the
//! entries of the module indexes it reads.

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{
    Body, Interface, ItemTree, ModuleId, ModuleIndex, PlainPathId, ProjectGraph, ProjectId,
};

use crate::walk::Walk;

/// One entry a walk read of the module index of a project, and what it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Read {
    /// The module a path of a project names, if the project holds one.
    Module {
        /// The project whose index was read.
        project: ProjectId,
        /// The path that was read.
        path: PlainPathId,
        /// The module the path names, or `None` when no module of the project has that path.
        found: Option<ModuleId>,
    },
    /// Whether any module of a project stands under a path: the prefix a name denotes.
    Prefix {
        /// The project whose index was read.
        project: ProjectId,
        /// The path that was read.
        path: PlainPathId,
        /// Whether the project holds a module under it.
        found: bool,
    },
}

/// The part of a resolution's input that belongs to other units ([ADR-0009]).
///
/// It holds the module indexes of the projects a path of the module may name, the interfaces
/// of the modules the walk reached, and the entries it read of those indexes. The interfaces
/// and the entries are what the closure is compared by: they are what a resolution read, and
/// a resolution that read the same values resolves the same way ([ADR-0008]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
/// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
#[derive(Debug, Clone, Default)]
pub struct Closure {
    /// The module indexes of the projects a path of the module may name.
    indexes: BTreeMap<ProjectId, Arc<ModuleIndex>>,
    /// The interfaces of the modules the walk reached.
    interfaces: BTreeMap<ModuleId, Arc<Interface>>,
    /// The entries of the indexes the walk read, in the order it read them.
    reads: Vec<Read>,
}

impl Closure {
    /// The closure of a module: the modules its paths reach, and the entries they read.
    ///
    /// The walk is the one a resolution makes, run once to gather what it reads: the driver
    /// assembles the closure, and the pass that follows walks the same paths over the value it
    /// is handed ([ADR-0009]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    pub fn of(
        module: ModuleId,
        tree: &ItemTree,
        graph: &ProjectGraph,
        indexes: &BTreeMap<ProjectId, Arc<ModuleIndex>>,
        fetch: &mut dyn FnMut(ModuleId) -> Option<Arc<Interface>>,
    ) -> Self {
        let mut walk = Walk::gathering(graph, indexes, fetch);
        crate::resolve::walk_all(module, tree, &mut walk);

        walk.into_closure()
    }

    /// The closure of a module whose bodies a check reads: the modules the check of a body
    /// reaches.
    ///
    /// It is wider than the closure of a resolution: a path of a body may name a module that no
    /// signature of the module names, so the walk of the surface is followed by the walk of every
    /// body ([ADR-0017]). The driver gathers this closure for a check, and the check walks the
    /// same paths over the value it is handed ([ADR-0009]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn of_check<'a>(
        module: ModuleId,
        tree: &'a ItemTree,
        bodies: impl IntoIterator<Item = &'a Body>,
        graph: &'a ProjectGraph,
        indexes: &'a BTreeMap<ProjectId, Arc<ModuleIndex>>,
        fetch: &'a mut dyn FnMut(ModuleId) -> Option<Arc<Interface>>,
    ) -> Self {
        let mut walk = Walk::gathering(graph, indexes, fetch);
        crate::resolve::walk_check(module, tree, bodies, &mut walk);

        walk.into_closure()
    }

    /// Whether this closure reads what `other` reads.
    ///
    /// A consumer of a module index is keyed by the entries it read ([ADR-0008]), so the two
    /// things a closure is compared by are the interfaces it reached and the entries it read,
    /// and not the indexes it searched: two closures that searched different indexes but found
    /// the same entries resolve a module the same way.
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    pub fn reads_the_same_as(&self, other: &Self) -> bool {
        self.reads == other.reads
            && self.interfaces.len() == other.interfaces.len()
            && self.interfaces.iter().zip(&other.interfaces).all(
                |((module, interface), (other_module, other_interface))| {
                    module == other_module && Arc::ptr_eq(interface, other_interface)
                },
            )
    }

    /// The module indexes the walk may read of.
    pub fn indexes(&self) -> &BTreeMap<ProjectId, Arc<ModuleIndex>> {
        &self.indexes
    }

    /// The interfaces the walk reached.
    pub fn interfaces(&self) -> &BTreeMap<ModuleId, Arc<Interface>> {
        &self.interfaces
    }

    /// The entries the walk read of the module indexes, in the order it read them.
    pub fn reads(&self) -> &[Read] {
        &self.reads
    }

    /// A closure of the values a walk gathered.
    pub(crate) fn new(
        indexes: BTreeMap<ProjectId, Arc<ModuleIndex>>,
        interfaces: BTreeMap<ModuleId, Arc<Interface>>,
        reads: Vec<Read>,
    ) -> Self {
        Self {
            indexes,
            interfaces,
            reads,
        }
    }
}
