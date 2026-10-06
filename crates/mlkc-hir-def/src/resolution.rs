//! What the names of a module denote, once the modules they name are read.
//!
//! The HIR of a module stops at the module boundary on purpose: a path that names an entity of
//! another module is kept as the path the module wrote ([ADR-0010]), and what such a path
//! denotes is a value of its own. That value is a [`Resolution`]: the scope of the module, and
//! what each import resolved to. It is made of the module/def-map values
//! ([`ModuleScope`], [`ProjectDefMap`](crate::ProjectDefMap)), so it lives beside them, and the
//! stages that read a path of a body ([ADR-0017]) read it without depending on the pass that
//! computed it.
//!
//! What the pass found wrong is not here: an error is a value of the walk
//! ([`ResolveError`](crate::ResolveError)), and the diagnostics of the pass are the pass's own.
//!
//! [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md

use std::{collections::BTreeMap, sync::Arc};

use crate::{ModuleScope, Target, UseLoc};

/// What the names of a module denote, and what each import of it resolved to.
///
/// The imports are what a later stage that resolves a path of a body reads: a name an import
/// brought in is either an entity or a place a path goes on from ([ADR-0017]). A resolution is
/// a value of one module, kept and compared whole by the driver ([ADR-0008]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// What each name of the module denotes: a declaration of it, or what an import resolved to.
    scope: Arc<ModuleScope>,
    /// What each import of the module resolved to, by the import.
    imports: BTreeMap<UseLoc, Target>,
}

impl Resolution {
    /// A resolution of the scope a module resolved to, with what its imports resolved to.
    pub fn new(scope: Arc<ModuleScope>, imports: BTreeMap<UseLoc, Target>) -> Self {
        Self { scope, imports }
    }

    /// What each name of the module denotes.
    pub fn scope(&self) -> &Arc<ModuleScope> {
        &self.scope
    }

    /// What each import of the module resolved to, by the import.
    ///
    /// A name a body writes is anchored at the entry of the import table it came from, so a
    /// stage that resolves the paths of a body reads what the import denotes here ([ADR-0017]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn imports(&self) -> &BTreeMap<UseLoc, Target> {
        &self.imports
    }
}
