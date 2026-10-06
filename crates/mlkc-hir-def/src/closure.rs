//! What a resolution reads of the rest of the project: the modules its walk reaches, and the
//! entries of the module indexes it reads.
//!
//! A closure is assembled by the walk of `mlkc-resolve`, which reads the paths of one module
//! once, and it is what the passes that follow are handed: the resolution walks the same paths
//! over it, and a check reads the interfaces of the modules through it ([ADR-0009]).
//!
//! [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

use std::{collections::BTreeMap, sync::Arc};

use crate::{Interface, ModuleId, ModuleIndex, PlainPathId, ProjectId};

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
    /// A closure of the values a walk gathered: the indexes it could read of, the interfaces
    /// it reached, and the entries it read.
    pub fn new(
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
}
