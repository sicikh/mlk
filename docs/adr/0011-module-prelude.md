# Give every module a prelude, and let a module and a project say which

- Status: accepted
- Date: 2026-09-26

## Context and Problem Statement

The names of the standard library --- `Int`, `Unit`, and the many that will follow ---
are declared in `std` and are used by almost every module of every project.
Writing `use std.prelude.Int` in each of them is boilerplate,
and the language is expected to give those names to a module without the module asking,
the way the standard preludes of other languages do.

Three facts make this less obvious than it looks.

First, `std` is compiled by the same pipeline as everything else,
and the prelude cannot apply to the modules of `std` that way:
`std.prelude` is where the prelude's names are defined,
and it cannot import itself.
The `std` project needs a prelude of its own, one that names its own modules
(`use project.core.Int`), and it should not repeat that line in every module either.

Second, the incrementality model is built on the claim
that a module's meaning is a function of the module's own text ([ADR-0004][0004], [ADR-0008][0008]).
A prelude is project-wide configuration that changes what a module's text means,
so where it enters the pipeline decides what it invalidates and what has to be in a key.

Third, [ADR-0004][0004] forbids glob imports on purpose:
with a glob, the names visible in a module depend on the _contents_ of another module.
A prelude that is a glob would reintroduce exactly that dependency,
so whatever the prelude is, it must be a fixed list of names,
known to the compiler before it reads any module.

The question:
at which stage does the prelude enter a module,
and what does it mean for a name the module declares itself?

## Decision Drivers

- The prelude must be invisible to every stage but the one that injects it:
  name resolution, interfaces, and checking already have a notion for
  "a name the module uses and did not declare" --- an import.
- A name the module declares or imports itself must win over the prelude,
  and it must win silently: the prelude is a convenience, not a declaration the module made,
  so shadowing it is not a mistake to report.
- The parse must stay a function of the file's text alone,
  or the identity of a parse stops being the version of the text ([ADR-0007][0007], [ADR-0008][0008]).
- The prelude must be an input of the pass that reads it and of no other,
  and changing it must invalidate what depends on it without touching a parse ([ADR-0009][0009]).
- `std` must be able to compile without the language prelude and with its own,
  and a single module must be able to refuse a prelude
  (the module that defines the prelude's names is the one that has to).

## Considered Options

- **Text before parsing** --- prepend `use std.prelude.Int` to the file's text and parse that.
- **CST nodes after parsing** --- parse the module, then splice synthetic `use` items into the tree.
- **A fallback table in name resolution** --- the local scope keeps the prelude as a list of
  names to paths, and an unresolved name becomes an anchor that names that path.
- **Implicit imports in the item tree** --- lowering declares the prelude's imports
  into the item tree and its scope, as if the module had written them.

## Decision Outcome

Chosen option: "Implicit imports in the item tree",
because it is the only one of the four
that leaves every stage after lowering untouched:
the prelude becomes an import, and imports are what name resolution already resolves,
what an interface already carries, and what a check already reads.

### The prelude is a list of imports, and lowering is what reads it

A `Prelude` is an ordered list of imports, each a name and the path it is the name of.
The language has a standard one --- `use std.prelude.Int` and `use std.prelude.Unit` today,
more later --- and a project may replace it with its own,
which is how `std` compiles with `use project.core.Int` and the like.

The prelude is an argument of `lower_module`:

```rust
pub fn lower_module(module: ModuleId, root: &ModuleRoot, prelude: &Prelude) -> LoweredModule;
```

Lowering declares one `use` entity per import, with the path of the import as its data,
after it has declared the module's own items.
The order is the whole of the shadowing rule:
what a module declares or imports first is what its names denote ([ADR-0010][0010]),
so a prelude entry that finds a name already taken is not recorded and nothing is reported.
A module that declares `type Int` keeps meaning its own `Int`;
a module that writes `use std.core.Int` keeps meaning that import;
and in both cases the prelude's entry is quietly not what the name denotes.
The prelude entity still exists in the item tree ---
a path that names it is anchored to its `UseLoc`, and a name has to exist to be named ---
but the scope of the module does not hold it.

A module refuses the prelude as a whole when its preamble carries the attribute:

```mlk
@no-prelude
module project.prelude

pub use project.core.Int
pub use project.core.Unit
```

The attribute is read from the preamble and recorded in the item tree,
and lowering declares nothing when it is there.
`@no-prelude` is the first attribute a module has of its own:
an attribute of a module that the language has no meaning for is a lowering mistake,
as one written in front of a declaration is.
A project-level refusal is what the prelude configuration already is:
a project whose prelude is empty gets nothing.

### A prelude entity is not written anywhere, and that is a property of the item tree

The entities of a module are found in its syntax by a path of child slots ([ADR-0010][0010]),
and a prelude import has no syntax.
`Entity` therefore holds an `Option<ItemSyntaxLoc>`:
`None` for an entity the module did not write, which today is exactly the prelude imports.

Three things follow, and all three are wanted:

- the driver computes no text range for a prelude entity,
  so a host marks no place for it and nothing points at nothing;
- the diagnostic that a name imported and declared in one module is a mistake
  has no place to point at for a prelude entry, so shadowing a prelude name is silent
  without a special case being written for it;
- a dump reads the entity as `prelude` where it would read `@2.3`,
  so a reader of an item tree sees what the module wrote and what it was given.

### What the prelude is not

It is not a glob. The list is fixed and named, and a scope that holds it
does not depend on the contents of another module.
It is not a second kind of import either:
nothing after lowering can tell a prelude import from a written one,
which is the point, and which is why the interface of a module carries
the prelude imports its public signatures depend on, as it carries private imports.

### A prelude belongs to a project, and the driver holds the module graph

The prelude is what a project gives its modules, so it is part of what a project is:
a `ProjectData` carries the module the project starts from, what it depends on, and its
prelude, and the driver holds the graph of projects together with the mapping of a module to
its project.

Lowering reads the prelude of the project the module belongs to,
and a module that belongs to no project --- a file a host pushed on its own,
which no manifest claimed --- is compiled with the prelude of the language ([`Prelude::standard`]).

Changing what a project says invalidates what its modules were lowered to, and nothing else:
crossing a project boundary is crossing a configuration boundary,
so the HIR of a module of another project, or of no project,
is not reread because a prelude it does not use changed.
A parse is never invalidated by a project at all,
since a parse is a function of the text and of nothing else.

What a host records and what the driver drops is defined by the driver ([mlkc-driver]);
a prelude is not read from a manifest yet,
because the loader that reads one is a stage the pipeline does not have.

### Positive Consequences

- No stage after lowering knows that a prelude exists:
  name resolution, the interface, and type checking see imports.
- A prelude name behaves exactly like an import of it,
  including being shadowed by a declaration of the same name,
  being visible wherever an import is visible, and being nameable across modules.
- The language's restrictions stay intact: the prelude is a named list, not a glob,
  and a module's meaning is still a function of its own text and the project's configuration.
- The `std` case is the same mechanism seen from the other side:
  a project prelude that names `project.core.Int`, and `@no-prelude` where it would be circular.

### Negative Consequences

- A prelude entry that no module uses still appears in every item tree of the project,
  so adding a name to a prelude invalidates the item trees of every module,
  used or not. Prelude edits are configuration edits and rare, and the parse is not invalidated;
  the alternative --- recording in the item tree only the entries a module read ---
  is deferred until the cost is measured.
- An entity with no syntax is a shape everything that walks an item tree has to consider,
  starting with the driver's ranges and the dump.
- A module without a preamble cannot refuse the prelude:
  the attribute belongs to the preamble, and a file that declares no path has nowhere to write it.
  Such a module can still shadow the prelude's names one by one.
- A module that belongs to no project is compiled with the prelude of the language,
  and a host that wants a file read under another prelude records a project for it.
- The prelude is part of what a project is, but what a project is is a value a host hands over:
  reading a prelude out of a manifest waits for the loader that reads manifests.

## Pros and Cons of the Options

### Text before parsing

Prepend the text of the prelude's imports to the source and parse that.

- Good, because it is trivial to implement.
- Bad, because the parse would no longer be a function of the file's text,
  which is what its key --- the `FileVersion` --- claims ([ADR-0007][0007]).
- Bad, because every range, every diagnostic, and every node of the tree
  would move by the length of the injected text, and a lossless tree
  would hold text the file does not ([ADR-0002][0002]).
- Bad, because every host that shows the source would have to know
  that what is parsed is not what is written.

### CST nodes after parsing

Parse the module, then splice synthetic `use` items into the tree.

- Good, because the pass that injects reads a tree, not text,
  and lowering needs no new input.
- Bad, because a green tree is a function of the text it covers:
  a node with no text of its own has no offsets to be at,
  and inserting one shifts every range after it.
- Bad, because the parse would still stop being a function of the text alone,
  one stage later.

### A fallback table in name resolution

The local scope holds the prelude as names to paths;
a name the module does not declare resolves to an anchor that names the path directly.

- Good, because the item tree stays a function of the module's text exactly,
  so an unused prelude entry invalidates nothing.
- Good, because nothing that walks the item tree has to learn about entities without syntax.
- Bad, because every consumer of an anchor learns a new case,
  and the case is not an import: it is a path that has to be resolved on the spot.
- Bad for incrementality: a signature that mentions a prelude name
  would have to carry either the resolved entity --- foreign data,
  which [ADR-0010][0010] forbids an interface to hold ---
  or the defining module's prelude configuration,
  which puts another module's configuration into every dependent's key
  ([ADR-0008][0008]'s locality).
- Bad, because the module namespace, the import table, and the interface
  would each need a second way to say "this name comes from elsewhere".

### Implicit imports in the item tree

Chosen. Described above.

- Good, because the prelude is spelled in the one way the language already has
  for "a name that comes from elsewhere": an import.
- Good, because shadowing, visibility, and cross-module resolution
  are the rules imports already have, and no new rule is invented.
- Good, because the pass that reads the prelude is exactly the pass that builds the import table,
  so the prelude is in the input of the stage it belongs to.
- Bad, because an item tree holds entities with no syntax.
- Bad, because an unused prelude entry is recorded in every item tree of the project.

## Links

- Module rules the prelude must not break: [0004-module-system.md]
- Keys, configuration as an input: [0008-compiler-driver.md]
- What a pass may read: [0009-pass-contract.md]
- Entities, names, and syntax locators: [0010-stable-entity-identity.md]
- Versions and losslessness: [0002-lossless-syntax-tree.md], [0007-vfs-file-state.md]
- Implementation: [mlkc-hir-def], [mlkc-lower], [mlkc-driver]

[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[0004-module-system.md]: 0004-module-system.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-lower]: ../../crates/mlkc-lower
[mlkc-driver]: ../../crates/mlkc-driver
