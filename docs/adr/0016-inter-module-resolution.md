# Resolve names across modules against interfaces

- Status: accepted
- Date: 2026-09-30

## Context and Problem Statement

The pipeline stops at the module boundary on purpose.
The HIR of a module holds what the module's own text determines:
its entities, their data, and a scope in which every name the module writes
denotes an entity of that module or an entry of its import table.
A path that names an entity of another module is kept as the path the module wrote,
and nothing in the HIR points outside the module ([ADR-0004][0004-module-system.md], [ADR-0010][0010-stable-entity-identity.md]).

So every stage after lowering — checking, and every question a host asks —
needs the value the HIR deliberately does not hold:
what a name that crosses a module boundary denotes.
No stage produces it today.
`ModuleScope` and `ProjectDefMap` exist in [mlkc-hir-def] as the shape of it,
and nothing fills them: the import table of a module resolves to nothing,
a path that names another module stays `PathAnchor::Unresolved`,
and no diagnostic says so.

The names of the language are the first case of it, and the hardest one.
`Int` and `Unit` are declared by the `core` module of the project `std`,
re-exported by its `prelude`,
and given to every module by the prelude of the language ([ADR-0011][0011-module-prelude.md], [ADR-0015][0015-standard-library.md]).
Resolving `Int` in a file no project claims is therefore a walk
through the module index of `std` and the interfaces of two of its modules.

The question:
what is one unit of cross-module resolution, what does such a unit read,
and what has to hold of it so that a keystroke in one module
does not re-resolve the whole project ([ADR-0004][0004-module-system.md], [ADR-0008][0008-compiler-driver.md])?

The naming this record decides is the naming the language has today:
a name denotes what the text of the modules that write it says, and nothing else.
A type system with type classes adds names that are not a function of text alone —
which `impl` provides the name a path reaches through a class —
and that naming is left to a record of its own.
What is text-determined does not change when the rest arrives:
the resolution this record fixes stays as it is,
and the names a type decides are added on top of it.

## Decision Drivers

- An edit must cost O(size of the edit), not O(project) ([ADR-0004][0004-module-system.md], [ADR-0008][0008-compiler-driver.md]):
  a resolution has to be invalidated by an edit to its own module,
  by an edit to a module whose _surface_ it read, and by nothing else.
- The pull graph stays stratified ([ADR-0008][0008-compiler-driver.md]):
  no fixpoint, no cycle handling at the driver,
  and no unit that reads another unit of its own level.
- The interface is the cross-module key ([ADR-0008][0008-compiler-driver.md]):
  a private edit of another module must not invalidate a resolution that never saw it.
- A name is the only identity that crosses a revision ([ADR-0010][0010-stable-entity-identity.md]),
  so a resolved entry can be stored, compared, and cited in a diagnostic.
- A pass is a pure function of an input the driver assembles ([ADR-0009][0009-pass-contract.md]),
  the input is a value, and the driver is the only thing that decides what it holds.
- Each error has one owner and one place ([ADR-0012][0012-split-the-dot-operator.md]):
  a name the module may not write at all is the lowering's, since the module alone decides it,
  and a path the rest of the project does not answer is the resolution's,
  reported where the module wrote the path.
- A stage's diagnostics are a value of that stage, and are not joined to another stage's:
  what a stage did not change, no later stage renders again.
- The language rules a resolution stands on:
  named imports and no globs; `pub` and private names; paths of `::` segments
  rooted at a project or at a name; a prelude that is a list of imports ([ADR-0004][0004-module-system.md], [ADR-0011][0011-module-prelude.md], [ADR-0012][0012-split-the-dot-operator.md]).
- Determinism: a resolution reads the same way in every process ([ADR-0009][0009-pass-contract.md]).

## Considered Options

- **One table of the whole project** —
  assemble the item trees of every module into one table,
  resolve each module in the context of that table,
  and update the entry of a module when the module changes.
- **The interfaces a module names** —
  a module is resolved against the _interfaces_ of the modules its own paths name,
  the walk through re-exports happens inside the resolution,
  and the def map is an index of per-module resolutions.
- **Exports resolved to a fixpoint** —
  the exported name of every module is first resolved to an entity,
  the modules are revisited until nothing more resolves,
  and a consumer reads a table of resolved exports.
- **Resolution on demand** —
  no resolution value per module: the driver answers "what does this name denote"
  per question, and memoizes the answer together with the entries it read.

## Decision Outcome

Chosen option: "The interfaces a module names",
because it is the only option that keeps every property the drivers above ask for.
Its unit is local (a module and the interfaces its own paths reach),
its value is keyed by exactly what it read,
and no unit of the pull graph reads another unit of its own level.

The option is a refinement of the first one rather than a rejection of it.
The table still exists — it is the def map — and it is still updated entry by entry.
What changes is what its entries are made of (a resolution, not an item tree)
and what a resolution is allowed to read while it is made.

### The stage, and the values it leaves behind

**Global name resolution** ([ADR-0005][0005-compiler-pipeline.md]) runs per module, in parallel,
after lowering and before checking. A module resolves its import table,
and the paths its own surface writes (the types of signatures and of `impl`s).
It produces three values.

- One **resolution** per module: the `ModuleScope` — what each name of the module denotes,
  an entity of it or an entity of another module it reaches — and the diagnostics of the walk.
- One **module index** per project: which path of the project names which module.
- One **def map** per project: the index of the resolutions of its modules,
  the `ProjectDefMap` that [ADR-0004][0004-module-system.md] asked for.

What a stage found wrong is among the values it leaves behind, and each stage's diagnostics
are its own: what the parse reported, what the lowering reported, and what a resolution found
are read apart, and no stage's mistakes are collected into a list of another stage's.
A parse error therefore renders once, and not again when a stage after it is re-run.

[ADR-0004][0004-module-system.md] called the join of the modules a sequential pass,
and [ADR-0005][0005-compiler-pipeline.md] annotates the stage as sequential;
[ADR-0008][0008-compiler-driver.md] turned it into an index of per-module contributions.
This record is the shape of one such contribution.
What resolves is a per-module value, computed in parallel with every other module;
what is sequential is the update of the indexes, one entry per module
("an update applies the diff of the changed scope rather than rebuilding the map").
What [ADR-0004][0004-module-system.md] decided about that pass —
that it sees surfaces and never bodies — stands.

The paths inside a _body_ are not resolved here.
A body is a unit of its own, and the stage that reads it — the check —
resolves its paths against the same values with the same function.
The split is the one the language already makes:
a module's meaning is its surface, and a body of it cannot change what another module reads ([ADR-0004][0004-module-system.md]).

The pass is a free function of its input ([ADR-0009][0009-pass-contract.md]):

```rust
pub fn resolve_module(
    module: ModuleId,
    tree: &ItemTree,
    deps: &ResolveDeps,
) -> (ModuleScope, Vec<ResolveDiag>);

/// The part of a resolution's input that belongs to other units.
pub struct ResolveDeps {
    /// The projects, what each of them depends on, and the project of every module.
    pub graph: Arc<ProjectGraph>,
    /// The modules the walk reached, and every entry of a module index it read.
    pub closure: Closure,
}
```

### The interface of a module is a function of its own text

An **interface** is what a module shows to the modules that read it.
It is a function of the module's own text alone --- a projection of its item tree —
so it is derived in parallel with every other module's,
and it holds two things:

- the path the module is called by in its project:
  what its preamble declares, or the place of its file when the module declares none.
  Every module has a name: a file pushed at a place that names no file is not a module,
  and the driver lowers nothing for it. An interface therefore holds a path, not an option of one;
- the names the module exports, in the order it declares them.

```rust
/// What a module shows to the modules that name it.
pub struct Interface {
    /// The path the module is called by in its project.
    path: PlainPathId,
    /// The names the module exports, in the order it declares them.
    exports: IndexMap<Name, Export>,
}

/// What one exported name denotes inside the module that exports it.
pub struct Export {
    /// The entity the module declares under this name:
    /// a class is in the type namespace, a function, a value, and a constant in the value one.
    pub ty: Option<EntityLoc>,
    pub value: Option<EntityLoc>,
    /// What an import of the name brings in: the path, as the module wrote it, unresolved.
    pub reexport: Option<PlainPathId>,
}
```

Three rules make the interface what it is.

- **It holds no id of the item tree it was cut from, and nothing of another module.**
  An exported name is either a name of the module (`EntityLoc`) or the path a `pub use` wrote.
  The interface is therefore comparable, retainable, and serializable on its own,
  and no other module is read while it is built ([ADR-0008][0008-compiler-driver.md], [ADR-0010][0010-stable-entity-identity.md]).
- **It holds only the names the module exports**, and an entry in it is public by the fact of being
  in it. The rule that fills the namespaces is the one a local scope already has:
  what the module declares wins over what it imports, per namespace,
  and the import is what the name denotes in a namespace the module declares nothing in ([ADR-0010][0010-stable-entity-identity.md]).
  A module that writes an import of a name it declares is a mistake the lowering already reports
  (an import of a declared name), so in a module that compiles the two rules never have to be
  told apart: an exported name is a declaration of the module or a re-export of one name, not both.
- **A re-export is left unresolved**, as the path of the `use` the module wrote.
  Resolving it would mean reading the modules it names,
  and then the interface would no longer be a function of one module's text.
  The modules that read the interface resolve the path, and pay for the walk
  ([ADR-0004][0004-module-system.md]'s "the global pass works only with surfaces").

Today an interface holds names, and nothing behind them.
The signatures a check reads and the `impl`s a class search reads
join this value with the stages that read them;
both are additive, and a value that grows invalidates its readers once.

### The index of a project: which module a path names

The second value is per project: the **module index**, a map from a module path
to the module that declares it, built from the interfaces of the modules the graph assigns to
the project.

- A module's path is the one its interface records, as the module declares it:
  the path its preamble wrote, or the place of its file when it declares none.
  A module calls the project it is in by the keyword and by nothing else,
  so the path of a module of a project is rooted at the keyword:
  a path rooted at a name is a path of the project that name is,
  and records no entry in this index.
- The index answers two questions, and both are one step:
  _the module at this path_, and _the modules under this prefix_, which is what a name
  the module binds as a prefix is resolved against.
- The index is an **index, not a value**: a module contributes its own entry,
  a change to it replaces one entry, and a consumer is keyed by the entries it read ([ADR-0008][0008-compiler-driver.md]).
  A module added under a prefix that some module names changes the entries under that prefix,
  and that resolution is re-run — it is a walk over its own imports, and it back-dates
  unless the added module is the one the path was looking for.

### A resolution reads the closure of the modules it names, and walks the chains itself

The input of a resolution is the closure ([ADR-0009][0009-pass-contract.md]):
the modules a walk from the module's own paths reaches, and the index entries it reads.
The walk is what the pass will do, run once by the driver to assemble the input:

1. take a path the module wrote — an import, or the root of a type;
2. read its root: the keyword `project`, the name of a project the module's project depends on,
   or the name of a module of the module's own project;
3. walk the segments over the module index until they reach a module;
4. read the name the path ends at in that module's interface;
5. when the entry is a re-export, continue from step 2 with the path the re-export wrote,
   read in the project of the module that wrote it;
6. stop at an entity, at a module, or at a failure.

Two properties of the walk are the reason the option was chosen.

- **A resolution reads interfaces and never another resolution.**
  Every step of the walk is answered by a value that is a function of one module's own text,
  so the units of this level do not depend on each other,
  and the pull graph is stratified exactly as [ADR-0008][0008-compiler-driver.md] describes.
- **A chain is walked, not iterated.**
  Two modules may import each other's names, and a re-export chain may run through any number
  of modules. Nothing has to be resolved for the chain to be followed:
  it is a loop inside one pass, over values the input already holds.
  A chain that comes back to where it started resolves to nothing,
  and the cycle is a diagnostic of the module whose import closes it —
  not a state of the driver's table.

The walk reads no further than the answer it needs:
a prefix is decided by the entries under it that were examined, and those entries are
what the key of the resolution holds.

### What a name of the module denotes

The value of a resolution is a `ModuleScope`: for every name of the module,
what it denotes in the three namespaces of the language (`PerNs`).

- A name the module declares denotes its own entity, in the namespaces of its kind,
  with the visibility the declaration gave it.
- A name an import brings in denotes what the path resolved to:
  an entity with the visibility of _this_ name (the visibility of the import),
  or a module locator with the visibility of the import.
  A name nothing resolved to has no entry, and a diagnostic says why.
- The visibility recorded is the reach of the name in the module that holds it,
  which is what a module reading the name from outside has to check.

Reaching a `Ty`/`Value` target is the end of a walk; a path that continues after it
(`Map::insert`) is a segment of an entity, and the language has no members today.
Such a path is unresolved, and it is where the deferred naming of type classes
will enter: the name after a class is what a type decides, not what the text decides.

### Visibility, roots, and the prelude

- **Visibility is checked once per hop.**
  A module reads its own names whatever their visibility;
  reading a name of another module needs the name to be `pub` there.
  A `pub use` shows a name as the module's own, which is how a module decides
  which names of other modules it carries further — and the standard library's `prelude`
  is the case of it: a project's names reach another project through the names the project shows,
  and every hop on the way is a `pub` one.
  A private name is reachable from the module that declares it and from nowhere else.
- **A project is named by the graph.**
  A module of a project names the projects that project depends on, and calls its own project
  by the keyword `project`: the name a project is declared under is not one its own modules
  write, and nothing but the keyword stands for it.
  The projects are handed to the lowering of a module, so a path of the module's surface is born
  with the project a name at its root denotes: the lowering alone is what reads such a name, and
  no later stage has to guess at it.
  The root of a path an _import_ or a re-export wrote is read by the walk instead: such a path
  is kept as the module wrote it ([ADR-0010][0010-stable-entity-identity.md]), and what it names
  is what the stage that holds the graph and the indexes resolves.
  A module that belongs to no project — a file a host pushed, which no manifest claimed —
  names the projects of the graph, which is all there is to declare a dependency on;
  and it belongs to no project's index, so no path of any project can name it.
- **The prelude is imports like any other** ([ADR-0011][0011-module-prelude.md]),
  and resolution reads them as it reads the imports the module wrote.
  One difference is deliberate: an import the module did not write is not reported
  when it resolves to nothing, because it has no place to point at.
  A _use_ of a name such an import would have brought in is reported at the use,
  which is the place the module wrote.

### The diagnostics

One family of errors, one place each ([ADR-0009][0009-pass-contract.md], [ADR-0012][0012-split-the-dot-operator.md]):

| What went wrong                                                           | Where it is reported                            |
| ------------------------------------------------------------------------- | ----------------------------------------------- |
| a name of the surface is no name of the module and no project it may name | the name, in the type that writes it            |
| a path starts with a name that is no project the module may name          | the path, in the module that wrote it           |
| a path names no module of the project it is read in                       | the path                                        |
| a module a path reaches does not export the name the path ends at         | the path                                        |
| the name the path ends at is a name the module holds and does not show    | the path, and the look taken once a walk failed |
| a name of another namespace of the module is written where a type belongs | the name, in the type that writes it            |
| an import resolves to nothing and a name it brought in is used as a type  | the name, in the type that writes it            |
| a re-export chain returns to where it started                             | the import that closes the cycle                |
| two modules of one project declare one path                               | each module's preamble, or its file             |

The first of the rows is the lowering's rather than the resolution's: whether a name is one the
module may write at all is decided by the module and the names it is given with — nothing
outside is read for it — so the module is where a reader is told ([ADR-0004][0004-module-system.md]).
What a name the module knows denotes is the rest of the project's to decide.
A name a module keeps to itself is the one thing told beyond the interfaces of other modules:
the walk finds a name the module does not show as a name that is not there, and telling the two
apart is a look at the item tree of the module — one taken only after a walk has failed
(the `hidden_name` look of [mlkc-resolve]), and recorded, since it is what the rendering read.

A diagnostic of the surface names the entity, the type among the entity's types, and the path:
the driver turns the first two into a range with the map it already holds for a host
(the `TypePlaces` of [mlkc-driver]), and never asks a pass for a span.
The rendering of a resolution's places is a value of the driver's own, keyed by the resolution
it was made from and by the HIR of every module the look read: a resolution that resolved
cleanly widens its key by nothing, and a re-run of a stage renders its own mistakes
and leaves the values of the other stages where they were.

### The keys, and what an edit costs

| Unit                 | Key — the values the pass read                                | Value                                      |
| -------------------- | ------------------------------------------------------------- | ------------------------------------------ |
| `Interface(module)`  | the module's own text (the item tree)                         | the path, and the names the module exports |
| `Modules(project)`   | the interfaces of the project's modules, entry by entry       | the index: a path to a `ModuleId`          |
| `Resolution(module)` | the item tree, the interfaces reached, the index entries read | the `ModuleScope`, and the diagnostics     |
| `DefMap(project)`    | the resolutions of its modules, entry by entry                | the index of the scopes                    |
| `Rendered(module)`   | the resolution, and the HIR every look read                   | the places of the resolution, as ranges    |

A value is compared with the one the driver retained, and an equal one is back-dated:
the driver keeps the value it holds, and every key that names it stays where it was ([ADR-0008][0008-compiler-driver.md]).
How much an edit costs then follows from the table.

- **A body edit** changes the text, and the item tree the module is re-lowered to compares equal
  to the one the driver holds. The interface is the same value, and no resolution is re-run.
- **A private surface edit** makes a different item tree, and the interface of the module is
  re-derived and compares equal, since a private name is not in it. Again nothing else is re-run.
- **An exported name added, removed, renamed, or given a new signature** makes a different interface.
  The modules that read it re-resolve: a walk over their own imports, which usually ends at the same
  entities and compares equal, so the value and every key that holds it stay.
  What was paid is a walk over the imports of the direct readers — not a lowering, not a check,
  and not a rebuild of the def map.
- **An edit that changes what a reader's name denotes** re-derives that reader's scope,
  and the new value is what its own readers see: the propagation follows the edges that exist,
  and every import is named, so the edges are the ones the modules actually use ([ADR-0004][0004-module-system.md]).

Two costs of the mechanism are worth stating plainly.

- The closure is walked twice, once by the driver to assemble the input and once by the pass.
  A walk is a handful of lookups per path, and [ADR-0009][0009-pass-contract.md] leaves no
  alternative: an input is a value, and a pass cannot ask for more than it was handed.
- A resolution that reads a prefix of a project names more of the project's shape than
  a resolution that reads one name, and re-runs when a module appears under its prefix.
  That is what a prefix costs, and it is decided where the module wrote it.

### What a host is given

- The driver gains pulls for the resolution of a module, the module index of a project,
  and the def map of a project, next to the pulls [ADR-0008][0008-compiler-driver.md] lists.
  What a host shows for one buffer — positions, ranges, the reading of the HIR — does not change.
- A buffer's diagnostics are read per stage: the parse's, the lowering's, and the resolution's
  are three values, and a stage whose value did not change keeps the diagnostics it had.
  The places a resolution reports are turned into ranges when a host asks for them,
  and that rendering is the driver's, keyed by what it read (see above).
- Go-to-definition across modules is the join the driver already does for one:
  the resolution answers with an `EntityLoc`, `file_path` gives the file of the module
  and `syntax_loc` with `item_range` gives the place.
- The def map is what whole-project questions are asked of:
  completion of a path segment after `::`, and "who exports this name" ([ADR-0012][0012-split-the-dot-operator.md]).
  Such a question is a consumer value of its own, keyed by the entries it read.

### How it is read and tested

- A resolution reads as a dump next to the one of the item tree:
  a name per line, with what it denotes, in the order the module declares the names.
  A dump is what a snapshot of a project is reviewed by, and the def map reads the same way ([ADR-0006][0006-snapshot-testing.md]).
- The pass is tested by calling it: a test builds the item trees of a handful of modules
  ([mlkc-lower]), the graph, the module indexes, and the interfaces of the closure,
  and calls `resolve_module` — no driver and no file system in the way ([ADR-0009][0009-pass-contract.md]).
- A project of a test is written in one file: a line that marks a place and the text under it
  is one module of the project, which is how a fixture of several modules is written and read
  ([mlkc-fixture]). A fixture that outgrows the format is a directory of modules,
  read over the same code.
- The incremental behaviour is the conformance harness of [ADR-0008][0008-compiler-driver.md]:
  a broken comparison or a wrong diff shows up as a difference
  between a long-lived driver and a fresh one over a sequence of edits.

### Positive Consequences

- A module resolves in parallel with every other module, and reads nothing but values of its own
  and interfaces of the modules it names.
- The cost of an edit follows the edit and its direct readers, not the project.
- There is no fixpoint, no cycle handling at the driver, and no state to converge:
  a cyclic re-export is an error of one module, found by a loop inside one pass.
- The language's own names resolve, including through a re-export:
  `Int` in a file of no project is a walk through the interface of the `prelude` module of `std`
  to the entity of its `core`.
- A resolved name is a value: it can be logged, compared in a test,
  and written into a diagnostic, which is what [ADR-0010][0010-stable-entity-identity.md] built names for.
- The errors of a stage are its own, and each has one place: a name the module may not write
  is not walked for by the resolution, an unresolved import is not discovered again by a checker,
  and a checker reads a scope that is either right or absent.
- The def map, the module index, and the scope are the values [ADR-0004][0004-module-system.md] and
  [ADR-0008][0008-compiler-driver.md] already named; the code that holds them today is where they land.

### Negative Consequences

- A module now has a value beside its item tree, and a slot beside the ones the driver has;
  an interface that grows with signatures and `impl`s widens the keys that name it once.
- A boundary of the walk is a rule of the language that has to be written down once and kept:
  which root a path may write — the keyword, the name of a project the module's project depends
  on, or the name of a module of its own project — and what a prefix denotes when no module is
  under it.
- A resolution reads the whole index of a project it names, as a structure,
  so a change of the project's shape is visible to it; the entries it read are what its key holds,
  and the walk is what decides which those are.
- The record decides the naming of today, and a name that a type should decide —
  a segment after a class --- is unresolved until the naming of type classes is decided,
  which is a record of its own.

## Pros and Cons of the Options

### One table of the whole project

Assemble the item trees of every module into one table, resolve each module against that table,
and update the entry of a module when the module changes.

- Good, because the table is the whole picture: a module resolves against any other module,
  and a question about the project is a question about the table.
- Good, because the implementation starts as one map and one loop.
- Bad, because a resolution reads the table, so the table is what its key has to hold:
  an edit to any module's surface invalidates the resolutions of every module,
  and the cost of a keystroke follows the project, which is what [ADR-0004][0004-module-system.md] exists to prevent.
- Bad, because an item tree holds private names and positions in the syntax,
  so a resolution keyed by one is invalidated by edits no reader observes;
  this is exactly the case the interface of [ADR-0008][0008-compiler-driver.md] was introduced for.
- Bad, because an import that names a re-export needs a _resolved_ entry of another module,
  and a table of item trees does not have one:
  either every resolution walks the chains itself — which is the chosen option,
  with the item trees in the place of the interfaces —
  or the table is resolved against itself, which is the next option.

### The interfaces a module names

Chosen. Described above.

- Good, because the unit is local and its key is exact: an edit invalidates the resolutions
  that read what the edit changed.
- Good, because nothing of one level reads another unit of its level,
  so there is no fixpoint and no cycle to break ([ADR-0008][0008-compiler-driver.md]).
- Good, because the interface is the value [ADR-0008][0008-compiler-driver.md] and
  [ADR-0010][0010-stable-entity-identity.md] already make the cross-module key:
  a private edit does not reach it, and a reader back-dates when a public edit did not touch what it read.
- Good, because a resolved name is an `EntityLoc`, which is stable in time and safe to hold.
- Bad, because a resolution is a value per module, and a driver that keeps only interfaces
  answers whole-project questions by assembling the closure again.
- Bad, because the interface is one more value to define, to keep in step with the item tree,
  and to widen when checking arrives.
- Bad, because a prefix a module names ties its resolution to the shape of a project's module tree.

### Exports resolved to a fixpoint

Resolve the exported name of every module to an entity first, revisiting the modules until
nothing more resolves, so that a consumer reads a table of fully resolved exports.

- Good, because a consumer never walks a chain: one lookup answers what a name denotes.
- Good, because a completion of a path would read a table of the final answers.
- Bad, because a resolution then depends on the resolutions of other modules,
  which is a unit of one level reading another: the pull graph is no longer stratified,
  and the driver needs a fixpoint and cycle detection — exactly what [ADR-0008][0008-compiler-driver.md] rules out.
- Bad, because the fixpoint resolves the re-exports of every module eagerly,
  including the ones nobody ever names, and a change anywhere in a project
  can move work through the whole fixpoint.
- Bad, because a cycle has to be handled by the engine rather than reported by a module:
  it is a mistake of one import, and the module that wrote the import is where it belongs.

### Resolution on demand

No resolution value per module: the driver answers one question at a time —
what does this path denote — and memoizes an answer together with the entries it read.

- Good, because nothing is computed that nobody asked for,
  and a question about one path reads the parts of the project that path needs.
- Good, because it is the same mechanism at a narrower unit:
  [ADR-0008][0008-compiler-driver.md]'s granularity is a knob, and a value per path is the fine setting.
- Bad, because the unit a host asks about is a module, not a path:
  the diagnostics of a file are one value, and the scope of its module is what they are about,
  so the fine unit is assembled back into the coarse one at every question.
- Bad, because a consumer that needs a scope — a check reads the whole scope of its module —
  asks the fine questions in a loop, which is the coarse value with more slots.
- Kept as the refinement of this record rather than as its starting point:
  the unit can be narrowed without changing the mechanism,
  which is what makes it safe to start with the scope.

## Links

- Incrementality and the two-phase resolution: [0004-module-system.md]
- The stage this record fills in: [0005-compiler-pipeline.md]
- The driver, its slots, and its keys: [0008-compiler-driver.md]
- The contract of a pass, and how an input is assembled: [0009-pass-contract.md]
- Names, ids, and what crosses a revision: [0010-stable-entity-identity.md]
- The prelude, which is imports: [0011-module-prelude.md]
- Paths, and which stage owns what they mean: [0012-split-the-dot-operator.md]
- The library whose names are the first case: [0015-standard-library.md]
- How a value is dumped and a pass is tested: [0006-snapshot-testing.md], [0009-pass-contract.md]
- Precedent, and the contrast: rust-analyzer's `DefMap` resolves imports per module
  with a fixpoint over the module graph; this record keeps the per-module unit
  and walks the chains inside one pass instead.
  <https://github.com/rust-lang/rust-analyzer/blob/master/crates/hir-def/src/nameres.rs>
- Implementation: [mlkc-hir-def], [mlkc-resolve], [mlkc-lower], [mlkc-driver], [mlkc-stdlib]

[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[0011-module-prelude.md]: 0011-module-prelude.md
[0012-split-the-dot-operator.md]: 0012-split-the-dot-operator.md
[0015-standard-library.md]: 0015-standard-library.md
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-resolve]: ../../crates/mlkc-resolve
[mlkc-lower]: ../../crates/mlkc-lower
[mlkc-driver]: ../../crates/mlkc-driver
[mlkc-fixture]: ../../crates/mlkc-fixture
[mlkc-stdlib]: ../../crates/mlkc-stdlib
