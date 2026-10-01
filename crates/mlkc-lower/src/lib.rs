//! The lowering of MLK into the HIR.
//!
//! Lowering turns the syntax tree of one module into the HIR of that module, which is two
//! values: the [`ItemTree`] of the module, its surface, and the [`Body`] of each entity
//! that owns one ([ADR-0010]). The two are lowered apart, from the same syntax and at
//! different times: an edit inside a body leaves the item tree equal, and an edit to the
//! surface of the module leaves the bodies that did not read it equal.
//!
//! [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
//!
//! # What lowering does
//!
//! - it reads the preamble of a module: the path the module declares itself as, and what the
//!   module says about itself as a whole. A module that declares no path of its own is called
//!   by where its file stands, which is what the caller hands it;
//! - it reads the items of a module and declares each of them with the data a dependent may
//!   read: the attributes a declaration carries, its visibility, the signature of a function,
//!   the path an import names;
//! - it declares the imports of the prelude the module is given ([ADR-0011]), after the items
//!   of the module, so that everything the module writes shadows them;
//! - it reads the body of a function into expressions, patterns, paths, and the names the
//!   body binds, and anchors a path to the binding it names before anything else can know
//!   the binding: only the body knows its own names;
//! - it says where every node of a body is written ([`BodySourceMap`]), since it is the only
//!   stage that knows which syntax a node was read from;
//! - it reports a name the module is written with --- in its surface, or in a body, where
//!   a binding of the body is a name as well --- that is neither a name of the module --- an
//!   entity of it, or an import of it --- nor the name of a project the module may name: the
//!   module alone decides it, and the module is where a reader is told about it
//!   ([ADR-0004], [ADR-0016]);
//! - it reports what the HIR cannot hold, and nothing else: an attribute that the language
//!   has no meaning for is the one thing a module can say that has nowhere to go.
//!
//! [ADR-0011]: ../../docs/adr/0011-module-prelude.md
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//!
//! # What lowering does not do
//!
//! - it does not resolve a name beyond the module it is in. A path that names a module of
//!   another project, or a name that is not there, stays
//!   [`PathAnchor::Unresolved`](mlkc_hir_def::PathAnchor::Unresolved), and the stage that
//!   holds the scopes of the project resolves it ([ADR-0004]);
//! - it does not check a type, a value, or a name. The HIR is what a module says, not what
//!   it means, and a check is a pass of its own over what lowering produced;
//! - it does not report what the parser already reported. A declaration, a parameter, or a
//!   name the parser could not read becomes a `Missing` node, or a name that is
//!   [`Name::missing`](mlkc_hir_def::Name::missing), and the parse is what a reader reads
//!   to see the mistake.
//!
//! [ADR-0004]: ../../docs/adr/0004-module-system.md
//!
//! # Lowering one module
//!
//! ```
//! use mlkc_hir_def::{Name, Namespace, PathAnchor, Prelude, ProjectId};
//! use mlkc_lower::{ModuleId, lower_body, lower_module};
//! use mlkc_vfs::{FileId, RelPathBuf};
//!
//! let source = "fun main(): Int = 1\n";
//! let parse = mlkc_parser::parse(source);
//! let root = parse.tree::<mlkc_syntax::ModuleRoot>();
//! let relative = RelPathBuf::try_from("main.mlk").expect("a relative path");
//!
//! let lowered = lower_module(
//!     ModuleId(FileId::from_raw(0)),
//!     &root,
//!     Prelude::standard(),
//!     &[ProjectId::new("std")],
//!     relative.as_path(),
//! );
//! assert!(lowered.diagnostics.is_empty());
//!
//! // The surface of the module: its entities, their names, and their data.
//! let tree = &lowered.item_tree;
//! assert_eq!(
//!     tree.scope().len(),
//!     7,
//!     "`main`, and the six names of the prelude"
//! );
//!
//! // The names of the prelude are names of the module like any other: `Int` is what the
//! // module's own text does not declare and the prelude brings in.
//! let int = tree
//!     .scope()
//!     .anchor(&mlkc_hir_def::Name::new("Int"), Namespace::Ty);
//! assert!(matches!(int, PathAnchor::Use(_)));
//!
//! // The bodies, lowered one at a time, from the declarations they are written in.
//! for decl in &lowered.bodies {
//!     // The work list holds the declarations that declare a body, so every one of them has
//!     // one; a declaration that declares none has no body to lower.
//!     if let Some(lowered_body) = lower_body(tree, &decl.decl) {
//!         assert!(lowered_body.diagnostics.is_empty());
//!     }
//! }
//! ```
//!
//! A body is lowered from the declaration it is written in, which is what
//! [`LoweredModule::bodies`] hands to [`lower_body`]. The syntax of an entity is found
//! again from the position the item tree recorded for it, with [`syntax_at`].
//!
//! # Modules
//!
//! - [`diagnostic`] — what lowering found, as typed errors and the spans they are at.

mod body;
mod decl;
pub mod diagnostic;
mod item;
mod pat;
mod path;
mod source_map;
mod syntax;
mod ty;

use std::fmt;

pub use mlkc_hir_def::{
    Body, BodyEntityLoc, ItemSyntaxLoc, ItemTree, ModuleId, Name, Prelude, ProjectId,
};
use mlkc_rowan::AstNode;
use mlkc_syntax::{FunDecl, ModuleRoot};
use mlkc_vfs::RelPath;

pub use crate::{
    diagnostic::{LoweringDiag, LoweringError},
    source_map::BodySourceMap,
    syntax::syntax_at,
};

/// The HIR of one module, and what lowering it reported.
#[derive(Debug)]
pub struct LoweredModule {
    /// The surface of the module: its entities, their names, and their data.
    pub item_tree: ItemTree,
    /// What lowering found, in the order of the source.
    pub diagnostics: Vec<LoweringDiag>,
    /// The entities that own a body, in the order the module declares them.
    pub bodies: Vec<BodyDecl>,
}

/// One entity that owns a body, and the declaration the body is written in.
///
/// The name is what a body is remembered by: a body has no name of its own, and the entity
/// that owns it plus the syntax it was lowered from identify it. What the declaration
/// declares is a body: a declaration that declares none is not in the list.
#[derive(Clone)]
pub struct BodyDecl {
    /// The name of the entity that owns the body.
    pub owner: BodyEntityLoc,
    /// The declaration the body is written in.
    pub decl: FunDecl,
}

impl fmt::Debug for BodyDecl {
    /// The name of the entity and the text of the declaration, since a syntax node
    /// is not a value that prints itself.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BodyDecl")
            .field("owner", &self.owner)
            .field("decl", &self.decl.syntax().text_trimmed())
            .finish()
    }
}

/// The HIR of one body, and what lowering it reported.
#[derive(Debug)]
pub struct LoweredBody {
    /// The body: its expressions, its patterns, its paths, and its parameters.
    pub body: Body,
    /// What lowering found, in the order of the source.
    pub diagnostics: Vec<LoweringDiag>,
    /// Where the nodes of the body are written, which is what a host marks a buffer by.
    pub source_map: BodySourceMap,
}

/// Lowers the items of a module into the surface of that module.
///
/// `root` is the syntax of the module, and `module` is the file it was read from: the names
/// the item tree mints are names inside that module, and a path that names an entity of it
/// resolves to that entity.
///
/// `prelude` is what the module is given without writing it: unless the module says
/// `#[no-prelude]`, the imports of the prelude are declared after the items of the module, so
/// that what the module writes is what its names denote ([ADR-0011]).
///
/// `relative` is where the file of the module stands: the path of the file under the root of
/// the project it is in --- `lib/arith.mlk` --- or the name of the file itself when it stands
/// in none. It is what the module is called when it declares no path of its own: a file of a
/// project is a module of it, and the place the file stands at is the canonical path of the
/// module (`project::lib::arith`).
///
/// `projects` is the projects the module may name: the projects the project it belongs to depends
/// on, or every project of the world for a module that belongs to no project ([ADR-0016]). What a
/// module calls its own project by is the keyword `project`, and not a name. A name written where
/// a type belongs that is neither a name of the module nor one of those projects is what the
/// lowering reports.
///
/// # Panics
///
/// Panics if the place names no file --- the root of a file system, pushed as if it were a
/// file --- since a module is a file, and a place that names no file is not where one stands.
/// The driver keeps such files out of the lowering, so a caller that hands one over is a bug.
///
/// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
pub fn lower_module(
    module: ModuleId,
    root: &ModuleRoot,
    prelude: &Prelude,
    projects: &[ProjectId],
    relative: &RelPath,
) -> LoweredModule {
    item::lower(module, root, prelude, projects, relative)
}

/// Lowers the body of a function, or nothing if the declaration declares no body.
///
/// `tree` is the item tree of the module the function is declared in: a path of the body
/// that names nothing inside the body is anchored against the names that module declares and
/// the projects it may name, where a value belongs.
///
/// A declaration that declares no body --- a function of a prelude, or an external one --- has
/// none here: there is no body to hold, and [`LoweredModule::bodies`] is the list of the
/// declarations that declare one.
pub fn lower_body(tree: &ItemTree, decl: &FunDecl) -> Option<LoweredBody> {
    body::lower(tree, decl)
}
