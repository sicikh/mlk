# Use module-based granular incrementality instead of a query system

- Status: accepted
- Date: 2026-09-20

## Context and Problem Statement

`mlkc` must serve an IDE:
it reacts to a stream of small edits
with a budget of roughly 100ms per change.
The work per change must be proportional to the size of the change,
not to the size of the project.

The standard answer to this problem is a query-based compiler:
an incremental computation engine (salsa, rustc's query system)
that tracks dependencies at runtime
and recomputes only what changed.

A query system is a general mechanism,
and its generality is also its cost:

- the dependency graph is maintained at runtime,
  and it is hard to profile and debug;
- the effectiveness of queries is limited
  by the dependency structure of the language;
- languages with global dependencies (macros, unconstrained impls)
  force fine-grained tracking from the very start.

The conclusion we draw from the analysis in [against-query-based-compilers]:
instead of building a query engine for an unfriendly language,
design the language so that compilation decomposes
into coarse, independent, module-sized chunks,
and handle the few genuinely global steps
with direct, sequential passes.

MLK is our language,
so we can impose the restrictions that make this possible.

## Decision Drivers

- IDE responsiveness: an edit must cost O(size of the edit), not O(project).
- Simplicity: no runtime dependency graph; data flow is direct and visible.
- Parallelism: name resolution (and later lowering and type checking)
  run per module, in parallel.
- Predictability: what invalidates what must be statically known.
- Precedent: Zig resolves names per file
  because every name is explicit and glob imports do not exist.

## Considered Options

- **No incrementality** — recompile the whole project on every change.
- **Query-based compiler** — salsa-style incremental computation engine
  with runtime dependency tracking.
- **Module-based granular incrementality** — the language restricts
  cross-module dependencies,
  so compilation splits into independent per-module stages
  plus cheap sequential whole-project passes.

## Decision Outcome

Chosen option: "Module-based granular incrementality",
because it achieves the same responsiveness as queries
with a fraction of the machinery,
at the price of language restrictions we are free to impose.

### The language restrictions that buy independence

Three rules make a module's meaning
independent of other modules' bodies.

#### No glob imports; every import is named

With glob imports,
the set of names visible in a module depends on the _contents_ of the imported module:
any item added or removed there silently changes name resolution here.
With named imports,
a module's interface depends only on the imported names
and their signatures,
which change far more rarely than bodies.

#### Strict orphan rules for type classes

Without orphan rules,
an `impl` relevant to a method call
could live in any module of the project,
so checking any module would depend on all modules.

The rules: an `impl` may live only

- next to the definition of the type class,
- next to the definition of the implemented type,
- or next to the definition of a type used as a generic parameter.

Consequently,
the set of modules that may contain a relevant `impl`
is determined by the types in the signature alone —
a closed set, known statically.
There is no dependency on the _non-existence_ of impls elsewhere,
which is the dependency that poisons fine-grained tracking in Rust.

#### Public functions must have signatures

If a public function could infer its signature from its body,
then callers would depend on the body:
editing the body could introduce type errors in other modules.
Requiring an explicit signature makes the module interface
fully determined by the module's surface (signatures and exports).
Then changing a body (without touching its signature)
can never invalidate another module.

This is exactly the property the incremental scheme requires:
changing the body of `foo`
cannot introduce a type error in `bar`.

### Two-phase name resolution

Because of these rules,
name resolution runs in two phases:

1. In parallel, per module:
   every use of an entity resolves _locally_ —
   either to an entity defined in this module,
   or to an entry in this module's import table.
   No global knowledge is needed.

2. Sequentially, once per project:
   the import and export tables of all modules
   are joined into the global entity table.
   This pass sees the whole project,
   but works only with surfaces (names, signatures, visibilities),
   not with bodies.

The shape of this design is visible in [mlkc-hir-def]:
each module produces a `ModuleScope` mapping names to resolved entities,
and the sequential pass assembles them into a `ProjectDefMap`.

### What invalidates what

- A change inside a function body
  invalidates that body only.
- A change to a signature or to exports
  invalidates the module's surface,
  the global table,
  and the modules that import from it.
- An added or removed item
  updates the global table in place
  (a diff of the module's surface),
  rather than rebuilding it.

The full stage-by-stage picture
(parse, resolve, lower, check, link, codegen)
is the subject of [0005-compiler-pipeline.md].

### Positive Consequences

- Name resolution, lowering, and type checking parallelize per module.
- There is no runtime dependency graph
  to maintain, debug, or profile.
- Incremental behavior is predictable:
  the cost of an edit is proportional to the size of the edit.
- The global passes are few, direct, and operate on compact data
  (surfaces, not bodies).

### Negative Consequences

- The language pays for independence with restrictions:
  no glob imports,
  constrained impl placement,
  mandatory public signatures.
  Users get more boilerplate.
- The unit of parallelism is the module:
  a single enormous module serializes work.
- The design relies on the restrictions staying intact;
  a future language feature that breaks module independence
  would degrade the whole scheme.

## Links

- Against Query Based Compilers: <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>
- Three Architectures for a Responsive IDE: <https://matklad.github.io/2023/12/28/three-architectures-for-responsive-ide.html>
- Zig's incremental compilation internals: <https://mlugg.co.uk/posts/incremental-compilation-internals/>
- Pipeline: [0005-compiler-pipeline.md]
- Implementation: [mlkc-hir-def]

[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[mlkc-hir-def]: ../../crates/mlkc-hir-def
