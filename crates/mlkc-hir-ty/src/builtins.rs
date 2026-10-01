//! The classes the language itself declares.
//!
//! `Int`, `Unit`, `String`, and `Bool` are not special syntax: the standard library declares
//! them with `#[builtin]`, and what makes them the classes of the language is that a compiler
//! hands their [`EntityLoc`]s to the stages that have to treat them specially --- the check
//! that types a literal, the lowering that chooses a MIR primitive, and the back end that
//! chooses a representation ([ADR-0017][adr-0017], [ADR-0018][adr-0018]).
//!
//! Holding them here rather than in one of those stages is what keeps the stages from each
//! looking a class up on their own: a caller reads them once, off the module that declares
//! them, and hands the same value to every stage. An entity compared by its [`EntityLoc`] is
//! compared by identity, so a stage never matches on a name ([ADR-0010][adr-0010]).
//!
//! [adr-0010]: ../../docs/adr/0010-stable-entity-identity.md
//! [adr-0017]: ../../docs/adr/0017-resolved-types.md
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md

use mlkc_hir_def::{ClassLoc, EntityLoc};

/// The classes of the language, named the way the project names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Builtins {
    int: EntityLoc<ClassLoc>,
    unit: EntityLoc<ClassLoc>,
    string: EntityLoc<ClassLoc>,
    boolean: EntityLoc<ClassLoc>,
}

impl Builtins {
    /// The classes of the language, in the order the standard library declares them: `Int`,
    /// `Unit`, `String`, and `Bool`.
    pub fn new(
        int: EntityLoc<ClassLoc>,
        unit: EntityLoc<ClassLoc>,
        string: EntityLoc<ClassLoc>,
        boolean: EntityLoc<ClassLoc>,
    ) -> Self {
        Self {
            int,
            unit,
            string,
            boolean,
        }
    }

    /// The class `Int`.
    pub fn int(&self) -> &EntityLoc<ClassLoc> {
        &self.int
    }

    /// The class `Unit`.
    pub fn unit(&self) -> &EntityLoc<ClassLoc> {
        &self.unit
    }

    /// The class `String`.
    pub fn string(&self) -> &EntityLoc<ClassLoc> {
        &self.string
    }

    /// The class `Bool`.
    pub fn boolean(&self) -> &EntityLoc<ClassLoc> {
        &self.boolean
    }
}
