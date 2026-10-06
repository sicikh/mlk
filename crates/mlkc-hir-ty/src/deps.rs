//! What a check reads of the rest of the project.
//!
//! The value is assembled by the driver and handed to the check, which reads nothing else
//! ([ADR-0009]).
//!
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{Closure, ModuleId, ProjectGraph};

use crate::{Builtins, ModuleTypes};

/// What checking a body reads of the rest of the project ([ADR-0009]).
///
/// The driver assembles the value --- the graph, the closure the walk gathered, and the
/// surfaces of the modules the body names --- and the check reads nothing else ([ADR-0008]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
/// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
#[derive(Debug, Clone)]
pub struct CheckDeps {
    /// The projects, what each of them depends on, and the project of every module.
    graph: Arc<ProjectGraph>,
    /// The modules the check walks, and the interfaces of them.
    closure: Closure,
    /// The type surface of the module the body belongs to, and of the ones it names.
    types: BTreeMap<ModuleId, Arc<ModuleTypes>>,
    /// The classes of the language.
    builtins: Builtins,
}

impl CheckDeps {
    /// Deps of a module that names nothing: no closure, and no surface of another module.
    pub fn new(builtins: Builtins) -> Self {
        Self {
            graph: Arc::new(ProjectGraph::default()),
            closure: Closure::default(),
            types: BTreeMap::new(),
            builtins,
        }
    }

    /// With the projects of the check.
    pub fn with_graph(mut self, graph: Arc<ProjectGraph>) -> Self {
        self.graph = graph;
        self
    }

    /// With the closure of the check: the modules its paths reach.
    pub fn with_closure(mut self, closure: Closure) -> Self {
        self.closure = closure;
        self
    }

    /// With the type surface of one module.
    pub fn with_types(mut self, module: ModuleId, types: Arc<ModuleTypes>) -> Self {
        self.types.insert(module, types);
        self
    }

    /// The projects of the check.
    pub fn graph(&self) -> &ProjectGraph {
        &self.graph
    }

    /// The closure of the check.
    pub fn closure(&self) -> &Closure {
        &self.closure
    }

    /// The type surface of a module, if the check was given one.
    pub fn types(&self, module: ModuleId) -> Option<&Arc<ModuleTypes>> {
        self.types.get(&module)
    }

    /// The classes of the language.
    pub fn builtins(&self) -> &Builtins {
        &self.builtins
    }
}
