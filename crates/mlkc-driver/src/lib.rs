//! The driver of the compiler.
//!
//! The driver is the component that owns the inputs and the memoized passes,
//! and the only one that decides what has to be recomputed.
//!
//! Three rules are visible in the code.
//!
//! - **Inputs are pushed, values are pulled**.
//!   The driver never opens a file, never reads a clock, never writes anywhere:
//!   a host hands it bytes, and asks it for values.
//! - **The key of a slot is the identity of what the pass read**.
//!   Everything here is a function of the text of one file --- and, for the HIR, of the prelude
//!   the file is compiled with and the projects it may name --- so the identity of that text,
//!   its [`mlkc_vfs::FileVersion`], is almost the whole key, while the text itself is handed to
//!   the pass by reference.
//! - **A pass is a function of its input**.
//!   [`Parse`] is what [`mlkc_parser::parse`] returned, and nothing more:
//!   the value and the diagnostics of the pass, stored as they came.
//!
//! # The diagnostics of a file
//!
//! Every stage reports its own, and what a host reads is the [`Diagnostics`] of a file: what
//! the parser reported, what the lowering of the module reported, what the resolution of it
//! found, and what checking its types found, each of them the value the stage left. No stage's
//! diagnostics are copied into another stage's, and a mistake one stage reports is not a reason
//! to render what the stages around it reported again.
//!
//! The renderings that are the driver's are the ones of the stages that report places in the
//! HIR: a resolution reports an entity and the type among its types, a check reports an
//! expression, a pattern, or a type a declaration writes, and where a place is written is what
//! the driver holds. Such a rendering is a value of its own, keyed by what it read, and what a
//! resolution's rendering reads of another module is its HIR only for a name a walk could not
//! find ([`mlkc_resolve::hidden_name`]): a module that resolved cleanly reads no HIR beside its
//! own.
//!
//! The construction of MIR reports nothing: a body the driver hands it is one every stage
//! before it read clean, and one it cannot lower is a bug of the check ([ADR-0019]), not
//! something a host is told about.
//!
//! [ADR-0019]: ../../docs/adr/0019-mir.md
//!
//! # Internal compiler exceptions
//!
//! A pass the driver calls is one the earlier stages made total input for, and a panic in one is
//! a bug of the compiler and not of the program ([`mlkc_diagnostics::Ice`]). The driver calls a
//! pass in a guarded way: the pull answers as if the value were not there, the first exception is
//! held with what the driver was computing while it happened ([`IceReport`]), and [`Driver::ice`]
//! is what a host reads to tell a person to file it. A host that would rather die on the spot
//! gets the same report from the panic it catches itself.
//!
//! # The types of a file
//!
//! The types of a module are resolved from the signatures it writes, and before the bodies of
//! it are checked: every top-level declaration writes its types for now ([ADR-0017]), so a
//! check never needs what another check found, and a body edit cannot change what the module
//! shows. A check of one body is a slot of its own --- the body is the unit --- and it reads the
//! surface of the module, the surfaces of the modules its paths reach, and the classes of the
//! language, and nothing of another body's check.
//!
//! The classes of the language --- `Int`, `Unit`, `String`, `Bool` --- are the ones the standard
//! library declares with `#[builtin]`: the driver reads them off `std::core` and hands them to
//! the check, and a driver whose host recorded no library checks no types.
//!
//! The diagnostics of a check are rendered here as well, the way the resolution's are: a check
//! reports places in the HIR --- an expression, a pattern, a type a declaration writes --- and
//! where a place is written is what the driver holds. The look a name a body path could not
//! find takes at the module it reached is the look the rendering of a resolution takes, and it
//! is recorded the same way.
//!
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md
//!
//! # The prelude, and the projects it belongs to
//!
//! Every module is compiled with the prelude of its project: the imports the module is given
//! without writing them, which lowering declares into the module's item tree
//! ([ADR-0011](../../docs/adr/0011-module-prelude.md)). A prelude belongs to a project ---
//! `std` and a project of a host each have their own --- and the driver holds the module
//! graph ([`mlkc_hir_def::ProjectGraph`]): which projects exist, what each of them depends on,
//! and which project a module belongs to.
//!
//! A host records them with [`Driver::set_project`], [`Driver::set_module_project`], and
//! [`Driver::remove_project`], and a module that belongs to no project --- a file a host
//! pushed on its own, which no manifest claimed --- is compiled with the prelude of the
//! language. What a project says decides what its modules' text means, so the HIR of a module
//! is dropped when the module changes project or the project's prelude changes; a parse is
//! not, because a parse does not read the prelude.
//!
//! A project is what a module may name, and the list of the ones it may name is what the
//! lowering of the module is handed: a project says what another project is called
//! ([`mlkc_hir_def::ProjectData::dependencies`]), those are the projects a path of a module of
//! it is rooted at, and a module of no project names every project of the graph, which is all
//! there is to declare a dependency on ([ADR-0016]).
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//!
//! The library the language's own prelude names is one such project, and it is the compiler's
//! rather than a host's ([`mlkc_stdlib`]): a host records it with [`Driver::use_std`], and a
//! host that shows it to a person --- the editor --- shows the files it was handed.
//!
//! A project is a set of modules rather than a tree of them: no module of a project is the one
//! it starts from, and a module is called by the place its file stands at, which the path a
//! host pushed it under says.
//!
//! # The green nodes of a parse
//!
//! A parse of a file is built through a table of green nodes, and the table it is built
//! through is the one its own parse before it left (`ParseSlot`): the tokens that did not
//! move are what the tree of a new revision shares with the tree of the old one, and a file
//! shares them however many other files were parsed in between.
//!
//! One table for the whole project would not: a table holds the nodes of the parse it was
//! last used for, so it shares a file with the file parsed just before it and with nothing
//! else, and every parse of every file takes it mutably, which is a project parsed one file
//! at a time.
//!
//! # Concurrency
//!
//! A pull takes `&mut self`, because it may compute, so the driver is not shared:
//! one thread owns it and asks it for values.
//! What that thread hands out are `Arc`s, which are immutable and safe to read anywhere,
//! so a host answers the requests that only read from the values it already pulled,
//! and sends the ones that need computing to the thread that owns the driver.
//!
//! Nothing read takes a lock on the driver, so a long pull cannot block a reader;
//! and a pull a host abandons leaves the table as valid as it found it,
//! because a slot is written only when its value is complete.
//!
//! The driver moves between threads as well, and so do the trees it handed out:
//! everything it owns is `Send`, and a tree is `Sync` besides,
//! which is what a host that owns the driver on a thread of its own relies on.
//! The assertion at the end of the driver module is what keeps that true.

mod driver;

pub use crate::driver::{Diagnostics, Driver, IceReport, Lowered, ModuleBody, Parse, StdFile};
