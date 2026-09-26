//! The definitions of the high-level IR.
//!
//! The HIR is the locally name-resolved tree of one module,
//! built from the module's own text alone
//! and allocated so that an edit invalidates as little as it can.
//! The model is [ADR-0010][adr-0010]; this crate is where it is implemented.
//!
//! [adr-0010]: ../../docs/adr/0010-stable-entity-identity.md
//!
//! # Two kinds of handle
//!
//! An **index** is a proof that a node exists, and the proof holds only inside the value
//! that owns the arena: `Idx<Expr>` of [`ExprId`] is meaningful inside the one
//! [`Body`] it was allocated in, and nowhere else.
//! A **name** is a value that denotes an entity of a module: [`FunctionLoc`], [`ClassLoc`],
//! and their erased sum [`ItemLoc`], all project-wide in [`EntityLoc`].
//! A name may denote nothing, so every use of one is a lookup that may fail.
//!
//! Three rules follow, and the crate is written so that they hold by construction.
//!
//! - **A positional id never leaves the value that owns the arena it points into.**
//!   Inside a body it is the natural handle; in a key, in an interface, or in a value
//!   that outlives a revision it would silently denote another node.
//! - **A name is never built by hand.**
//!   [`ItemTreeBuilder`] mints names, counts the entities that share a name,
//!   and is the only thing that can: the fields of a name are private to this crate.
//! - **A name is the only identity of a unit for anything that crosses a revision.**
//!   Memo slots, entries of a scope, and anything written to disk key on a name,
//!   never on a position.
//!
//! # The item tree is a module's surface
//!
//! [`ItemTree`] holds what the module's text alone determines:
//! its entities and their observable data, the names of the entities, where each of them
//! is in the module's syntax, and the names the module declares.
//! It holds no body and nothing that an edit inside a body can shift,
//! so a body edit leaves its value equal and no dependent of the module is invalidated.
//!
//! The data of an entity is a value of its own: it owns the paths and type references
//! it names, a local target is an [`EntityLoc`] of the same module,
//! and a target in another module is the path the module wrote, unresolved.
//! A dependent therefore records exactly the data it read, by value,
//! and keys on that instead of on the whole item tree ([ADR-0008][adr-0008]).
//!
//! [adr-0008]: ../../docs/adr/0008-compiler-driver.md
//!
//! # A body owns everything positional
//!
//! [`Body`] owns the expressions, the patterns, the paths,
//! and the entities declared inside it, in arenas of its own.
//! A body has no name: it is the body of an entity,
//! and the entity's name plus the syntax it was lowered from identify it.
//!
//! # What the model asks of a value
//!
//! Names are interned, and interning is load-bearing rather than an optimization:
//! `Eq` on two entities, two item trees, or two signatures is a handful of pointer comparisons,
//! which is what lets the driver compare the values it kept ([ADR-0008][adr-0008]).
//! For that to be exact, a value that is compared has to be retained,
//! which the driver guarantees by holding the value its key names.
//!
//! Hashing and ordering of a [`Name`] are by text, never by address,
//! so no map layout depends on where an allocation happened to land.
//! It is also why nothing large derives `Hash`:
//! a fingerprint is an explicit function next to a value, not a way to decide validity.
//!
//! # Modules
//!
//! - [`id`] — the kinds of entity, their names, and their ids.
//! - [`name`] — a [`Name`], as declarations and references spell it.
//! - [`path`] — a path as written, and what its base resolved to.
//! - [`type_ref`] — a reference to a type, and a type variable.
//! - [`item_data`] — the observable data of each kind of entity.
//! - [`item_tree`] — the surface of one module.
//! - [`body`] — the inside of one function.
//! - [`def_map`] — what names denote, and the index of the whole project.
//! - [`project_graph`] — the name of a project, and what it depends on.
//! - [`dump`] — a reading of the tree and of a body, for a person and for a diff.

mod macros;

pub mod body;
pub mod def_map;
pub mod dump;
pub mod id;
pub mod item_data;
pub mod item_tree;
pub mod name;
pub mod path;
pub mod project_graph;
pub mod type_ref;

pub use crate::{
    body::{BinaryOp, Body, BodyBuilder, Expr, ExprId, Literal, Pat, PatId, UnaryOp},
    def_map::{LocalEntry, LocalScope, LocalTarget, ModuleScope, Namespace, PerNs, ProjectDefMap},
    id::{
        BodyEntityLoc, BodyLoc, ClassLoc, ConstLoc, EntityLoc, FunctionLoc, ImplLoc, ItemKind,
        ItemLoc, ItemLocData, ItemLocLike, LocalConstId, LocalDefId, LocalFunctionId, ModuleDefId,
        ModuleDefWithBodyId, ModuleId, UseLoc, ValueLoc, WrongKind,
    },
    item_data::{
        Attributes, ClassData, ConstData, EntityData, FunctionData, ImplData, ParamData, Signature,
        UseData, ValueData, Visibility,
    },
    item_tree::{Declared, Entity, ItemSyntaxLoc, ItemTree, ItemTreeBuilder, ModuleEntity},
    name::Name,
    path::{PathAnchor, PathData, PathId, PathRoot, PlainPath, PlainPathId},
    project_graph::{ModuleLocator, ProjectData, ProjectGraph, ProjectId},
    type_ref::{TypeRef, TypeVarId},
};
