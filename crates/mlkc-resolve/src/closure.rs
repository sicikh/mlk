//! How the closure of a resolution is gathered.
//!
//! The value itself is [`mlkc_hir_def::Closure`]: what a walk read of the module indexes and
//! the interfaces, and everything the passes that follow read of the rest of the project.
//! A walk runs over the paths of a module once here, and the resolution that follows walks the
//! same paths over what it gathered ([ADR-0009]).
//!
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{
    Body, Closure, Interface, ItemTree, ModuleId, ModuleIndex, ProjectGraph, ProjectId,
};

use crate::walk::Walk;

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
) -> Closure {
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
) -> Closure {
    let mut walk = Walk::gathering(graph, indexes, fetch);
    crate::resolve::walk_check(module, tree, bodies, &mut walk);

    walk.into_closure()
}
