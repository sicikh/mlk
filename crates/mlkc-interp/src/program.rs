//! The functions a program is made of, and the ones it only declares.
//!
//! A program is the bodies of every module of a project, keyed by the entity that owns each
//! ([ADR-0010][adr-0010]), and the declarations of the functions implemented outside it. The
//! interpreter reads a body by the entity a call names, which is what makes a call between two
//! modules an ordinary call: the caller holds an [`EntityLoc`], and the program has the body or
//! the extern under it.
//!
//! [adr-0010]: ../../docs/adr/0010-stable-entity-identity.md

use std::{collections::BTreeMap, fmt, sync::Arc};

use mlkc_hir_def::{BodyLoc, EntityLoc, FunctionLoc};
use mlkc_mir::Body;

/// A function implemented outside the program: what a host is asked for by name.
///
/// The name is the canonical one of [ADR-0021][adr-0021]: the path of the module that declares
/// the function, and the name of the entity under it. An extern function of `std::runtime` is
/// `print-int` of `std::runtime`, whether it is called from the module that declares it or from
/// a module that imports it.
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Extern {
    /// The canonical path of the module that declares the function.
    pub module: String,
    /// The name of the function inside the module.
    pub name: String,
}

impl fmt::Display for Extern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.module, self.name)
    }
}

/// The functions a program is made of, across its modules.
///
/// A body is keyed by the function that owns it, and an extern declaration by the entity it
/// declares: a call resolves against both, and a function is one of them ([`Program::body`],
/// [`Program::external`]).
#[derive(Debug, Clone, Default)]
pub struct Program {
    bodies: BTreeMap<EntityLoc<FunctionLoc>, Arc<Body>>,
    externals: BTreeMap<EntityLoc<FunctionLoc>, Extern>,
}

impl Program {
    /// A program of no functions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the body of a function.
    ///
    /// A body of a constant is a body, but no call reaches it: it is not recorded.
    pub fn insert(&mut self, body: Arc<Body>) {
        let BodyLoc::Function(loc) = &body.owner.item else {
            return;
        };

        let function = EntityLoc {
            module: body.owner.module,
            item: loc.clone(),
        };

        self.bodies.insert(function, body);
    }

    /// Records that a function is implemented outside the program, under `external`.
    pub fn declare_extern(&mut self, function: EntityLoc<FunctionLoc>, external: Extern) {
        self.externals.insert(function, external);
    }

    /// The body of `function`, if the program has one.
    pub fn body(&self, function: &EntityLoc<FunctionLoc>) -> Option<&Arc<Body>> {
        self.bodies.get(function)
    }

    /// The declaration of `function` outside the program, if there is one.
    pub fn external(&self, function: &EntityLoc<FunctionLoc>) -> Option<&Extern> {
        self.externals.get(function)
    }
}
