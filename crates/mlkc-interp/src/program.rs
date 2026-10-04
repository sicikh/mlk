//! The functions a program is made of, and the ones it only declares.
//!
//! A program is the bodies of every module of a project, keyed by the function each body is
//! ([ADR-0010][adr-0010]): the body of an entity, a function declared in a `local`, and a lambda
//! are all functions of their module ([ADR-0019][adr-0019]), so all of them are keyed the same
//! way. The declarations of the functions implemented outside the program are keyed by the
//! entity a call names, which is what makes a call between two modules an ordinary call: the
//! caller holds a function of its module, and the program has the body or the extern under it.
//!
//! [adr-0010]: ../../docs/adr/0010-stable-entity-identity.md
//! [adr-0019]: ../../docs/adr/0019-mir.md

use std::{collections::BTreeMap, fmt, sync::Arc};

use mlkc_hir_def::{EntityLoc, FunctionLoc};
use mlkc_mir::{Body, FunctionLoc as MirFunctionLoc};

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
/// A body is keyed by the function it is, and an extern declaration by the entity it declares:
/// a call resolves against both, and a function is one of them ([`Program::body`],
/// [`Program::external`]).
#[derive(Debug, Clone, Default)]
pub struct Program {
    bodies: BTreeMap<MirFunctionLoc, Arc<Body>>,
    externals: BTreeMap<EntityLoc<FunctionLoc>, Extern>,
}

impl Program {
    /// A program of no functions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the body of a function.
    ///
    /// The body of a constant is a body, but no call reaches it: it is recorded like any other,
    /// because a lambda written in one is a function a call may reach.
    pub fn insert(&mut self, body: Arc<Body>) {
        self.bodies.insert(body.function.clone(), body);
    }

    /// Records that a function is implemented outside the program, under `external`.
    pub fn declare_extern(&mut self, function: EntityLoc<FunctionLoc>, external: Extern) {
        self.externals.insert(function, external);
    }

    /// The body of a function, if the program has one.
    pub fn body(&self, function: &MirFunctionLoc) -> Option<&Arc<Body>> {
        self.bodies.get(function)
    }

    /// The declaration of an entity outside the program, if there is one.
    pub fn external(&self, function: &EntityLoc<FunctionLoc>) -> Option<&Extern> {
        self.externals.get(function)
    }
}
