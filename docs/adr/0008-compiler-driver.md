# Drive the compiler from a host-agnostic pull-based cache

- Status: accepted
- Date: 2026-09-22

## Context and Problem Statement

`mlkc` has to work in three environments, with the same analysis underneath:

- as a CLI, compiling a project from the real file system;
- as a language server, answering a stream of requests over a stream of small edits;
- as a service in the browser, compiled to WASM,
  where the editor's buffers take the place of a file system
  and the same requests are answered inside a tab, next to a viewer of syntax trees.

The pipeline is a sequence of pure functions ([ADR-0005][0005-compiler-pipeline.md]),
so only two things differ between the environments:
where the text comes from, and in what order values are asked for.

That makes the driver the boundary:
it owns the inputs and the derived values,
and it is the only component that may know what has to be recomputed.
A query engine for that job was rejected in [ADR-0004][0004-module-system.md],
which leaves the questions this ADR answers:

1. what is the driver's contract with its hosts,
   so that being a CLI, a language server, or a WASM module is a difference the driver cannot see;
2. how does it decide what to recompute without the runtime dependency graph a query engine would build.

The purity of the passes suggests the shape of the answer:
the driver keeps a table of memoized passes,
and re-runs a pass when the state it was built from is no longer the state of the world.
Such a table is a query system, but a language-specific one:
the structure it tracks is the module graph the language already describes,
not a graph discovered at runtime by a general engine.

What that shape does not settle is the key —
how a slot decides that what it recorded is still current — and the rest follows from it.
A concrete proposal of this family, a per-module slot table keyed by fingerprints,
motivated this record and appears in full under Considered Options.

This ADR decides the contract, the shape of the table, and the kind of key.
The diagnostics model, the back-end slots and the project manifest are separate records.

## Decision Drivers

- Host-agnosticism:
  the driver must not know whether the bytes it is handed came from
  a file system, an editor buffer, or a JS shim.
- Responsiveness: an edit must cost O(size of the edit), not O(project),
  with roughly 100ms per change as the budget ([ADR-0004][0004-module-system.md]).
- One analysis, three hosts:
  the CLI, the language server and the browser service must not each grow their own invalidation.
- Soundness: a stale answer is a wrong answer.
  A broken cache must recompute something unnecessarily, never return something old.
- Simplicity: the invalidation rules have to be visible to a human,
  with no runtime dependency graph to maintain, debug, or profile.
- Parallelism per module ([ADR-0005][0005-compiler-pipeline.md]),
  with a fallback that works when there is no threading at all, as in a WASM build.
- Cheap invalidation: "is this still valid?" must be a comparison,
  not a re-read or a re-hash of a value ([ADR-0007][0007-vfs-file-state.md]).
- Purity, and a one-way crate graph:
  a pass is a function of its input and of nothing else,
  so the pipeline crates must not know that a driver exists ([ADR-0009][0009-pass-contract.md]).
- Granularity is a knob, not an architecture:
  narrowing what a key covers must not change the mechanism.
- Precedent: interface checksums are how ML-family compilers
  (GHC's recompilation avoidance, OCaml's `.cmi` files) have avoided recompilation for decades,
  and rust-analyzer's item tree plays the same role in a language server.

## Considered Options

- **Pull-model memo table with fingerprint keys** —
  one slot per module, keyed by hashes of the text and of the interfaces it read.
- **Pull-model memo table with identity keys** —
  the same table, keyed by exact identities:
  the version of an input, the retained value of a derived one.
- **Push-model worklist driver** —
  a dirty set, recomputed eagerly in topological order after every change.
- **Query engine** — salsa-style: arbitrary queries, dependencies recorded at runtime.
- **File-level incrementality** —
  reuse a result only while the text it came from is unchanged; no interfaces in the keys.
- **No cross-revision cache** —
  re-run the pipeline per request, memoizing only within a revision.
- **Separate compilation on disk** —
  the compiler compiles one module against serialized interfaces, and a build tool schedules.

## Decision Outcome

Chosen option: "Pull-model memo table with identity keys",
because both requirements that kill the remaining alternatives —
O(edit) cost for the IDE and one code path for all three hosts —
are met by a mechanism whose structure is already described
by [ADR-0003][0003-id-based-ir.md], [ADR-0004][0004-module-system.md] and [ADR-0005][0005-compiler-pipeline.md],
without a runtime dependency graph and without a new dependency in the most central crate;
and because between the two ways to key that mechanism,
the exact one cannot silently under-cover the value it stands for,
which is the one failure mode a compiler cache must not have.

The pipeline is the one from [ADR-0005][0005-compiler-pipeline.md];
this ADR only fixes how the driver drives it.

### The driver is the boundary between the hosts and the pipeline

The driver does three things, and nothing else:

- **takes inputs by push:**
  text and its absence, the file set, the manifest, the options.
  It never opens a file, never reads a clock, never writes anything anywhere.
- **hands out values by pull:**
  host asks for a value, the driver returns it,
  computing whatever is missing on the way.
- **serves reads without computation:**
  the text of a file, its version, its path, its state ([ADR-0007][0007-vfs-file-state.md]).

It owns the `Vfs`, the module graph, the memo table, and the parallel dispatch of a stage,
and it is the one that assembles the input of every pass.
It does not own threads as a policy, debouncing, priorities, the protocol,
or the decision of what to ask for.
Those are the host's, and they are the only thing that differs between the three.

| Host            | Pushes                                 | Pulls                                    | Owns                                      |
| --------------- | -------------------------------------- | ---------------------------------------- | ----------------------------------------- |
| `mlkc-cli`      | the file set read from the file system | diagnostics, the artifact                | file system, process, exit code           |
| language server | buffers and file events                | diagnostics, trees, the def map          | protocol, priorities, debouncing, workers |
| browser service | editor buffers and a virtual file list | the same, plus dumps for the tree viewer | editor, Web Worker, UI                    |

The flow of a revision is the same in all three:
push what changed, pull what is needed, publish the answer.
Nothing in the driver branches on the environment the code was compiled for,
except a single switch that turns parallel stage dispatch into a plain loop.

### The host contract

```rust
impl Driver {
    // Inputs. The driver never reaches for any of this.
    // The file API is the `Vfs` one, re-exposed ([ADR-0007][0007-vfs-file-state.md]).
    pub fn set_file_contents(&mut self, path: VfsPath, contents: Option<Vec<u8>>) -> bool;
    pub fn set_file_text(&mut self, path: VfsPath, text: Option<String>) -> bool;
    pub fn set_file_set(&mut self, files: FileSet);
    pub fn set_config(&mut self, config: Config);

    // Pulls. May compute; never return a value from an invalid slot.
    pub fn parse(&mut self, file: FileId) -> Option<Arc<Parse>>;
    pub fn diagnostics(&mut self, file: FileId) -> Option<Vec<Diagnostic>>;
    // The units that follow are more of the same:
    // `item_tree`, `interface`, `body`, `def_map`, `emit`.

    // Reads. No computation, no `&mut self`.
    pub fn file_id(&self, path: &VfsPath) -> Option<FileId>;
    pub fn file_text(&self, file: FileId) -> Option<Arc<str>>;
    pub fn file_version(&self, file: FileId) -> FileVersion;
    pub fn file_state(&self, file: FileId) -> FileState;
    pub fn file_path(&self, file: FileId) -> &VfsPath;
}
```

A pull answers `None` when the driver holds nothing to compute from,
which for the parse means no text: the file was never pushed, it is gone, or it is not text.
The `file_state` read tells those apart, and a host that needs the net changes of its pushes
can drain `take_changes` on the `Vfs` ([ADR-0007][0007-vfs-file-state.md]).

Three consequences fall out of this shape:

- A pull takes `&mut self`, because it may compute.
  The driver is owned by one thread and is not `Sync`;
  what crosses thread boundaries are the `Arc`s it hands out.
- A host that must render an answer without waiting
  uses the read-only side and the `Arc`s it already holds;
  if a document is no longer at the version an answer was computed for,
  the answer is discarded and pulled again ([ADR-0007][0007-vfs-file-state.md]).
- The browser viewer of syntax trees needs no new machinery:
  a dump of a tree is a pulled value,
  and the dumps [ADR-0006][0006-snapshot-testing.md] mandates for snapshots are the same dumps.

### Slots: what is memoized

The units of memoization are the units of invalidation, which [ADR-0003][0003-id-based-ir.md] already fixed:
an arena lives with the smallest owner whose invalidation unit contains it,
and the driver's slots mirror that hierarchy.
Each slot holds one value of [ADR-0005][0005-compiler-pipeline.md]'s pipeline.

| Unit                                               | Value                                        | Depends on                                               |
| -------------------------------------------------- | -------------------------------------------- | -------------------------------------------------------- |
| `Parse(file)`                                      | the CST and the parse errors                 | text                                                     |
| `Module(module)`                                   | the `ItemTree`, the `Interface`              | the CST                                                  |
| `Body(owner)`                                      | the lowered HIR of one body, its diagnostics | the `ItemTree`, the body's syntax                        |
| `Check(owner)`                                     | the inference result, its diagnostics        | the `Body`, the interfaces it consults                   |
| `DefMap(project)`                                  | the joined scopes (see below)                | every module's interface                                 |
| `Mir(function)`, `Codegen(function)`, `Link(unit)` | —                                            | not designed yet ([ADR-0005][0005-compiler-pipeline.md]) |

Three properties of the table:

- A slot may be empty: nothing is computed until a host asks for it,
  and what the hosts ask for is what is worth keeping.
- No slot is an input, so dropping one costs a recomputation and never correctness.
- A slot is never read while invalid:
  the pull validates and, if needed, recomputes before returning,
  so a host cannot observe a stale diagnostic, because it cannot observe an invalid slot.

The key of a slot is the identity of what the pass that fills it read:
its own unit's version, plus the values it was handed from other units ([ADR-0009][0009-pass-contract.md]),
and a pass is a function of its input and of nothing else.

One nuance of [ADR-0007][0007-vfs-file-state.md]:
`take_changes` is not what drives the rebuild.
The driver records the files a host pushes as a frontier to start validation from,
and every slot then decides its own validity.
Draining `take_changes` remains useful to a host that wants to know which documents to re-publish.

### What a key is: identities, not fingerprints

The two memo tables differ in one thing, so the key is what this decision turns on.
A fingerprint is attractive for reasons that have nothing to do with invalidation:
a `u64` is small, cheap to compare, easy to print in a trace and to log,
and it is the shape ML-family compilers put on disk (GHC's `ModIface`, OCaml's `.cmi`),
so the same keys could serve a cross-run cache later without a conversion.

| Key of          | Fingerprint variant                                  | Identity variant (chosen)                                                                 |
| --------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| an input        | `text_hash: u64`, a hash of the text                 | `FileVersion`, the version the `Vfs` already assigns ([ADR-0007][0007-vfs-file-state.md]) |
| a derived value | `interface_hash: u64`, a hash of the observable part | the retained `Arc<Interface>` itself                                                      |

The fingerprints lose on three counts.

First, [ADR-0007][0007-vfs-file-state.md] already made the identity of a file its `FileVersion`,
O(1) to compare and unable to collide;
a second identity for the same input, produced by hashing it on the hot path,
is exactly what that decision rejected.

Second, and worse, a hash is a _summary_ of a value,
and a summary can be wrong in the direction that matters:
if it misses a part of the value that a consumer observes,
the dependent is not invalidated and is served a stale answer —
the one failure mode a compiler cache must not have.

Third, a summary has to be canonical, and this workspace's values are not:
`Symbol` is interned and compares and hashes by address,
so `#[derive(Hash)]` on a pipeline value depends on allocation addresses,
and an address freed with one name and reused by another
can make a changed value hash exactly like the old one.
A hand-written `StableHash` per value would fix it,
at the price of one more thing to keep in sync with the value it summarizes.

So a key is an exact identity, never a hash:

- **an input** is identified by its version —
  `FileVersion` for text, a version for the file set and for the options;
- **a value that comes from anywhere else** — another module's interface, the module graph —
  is identified by the value itself:
  the key holds the `Arc` it was built from, and validity is a pointer comparison.

A stage whose reads are all its own data therefore needs no retained value in its key at all:
the version of that data is enough, and the pass is handed the data itself by reference.
Only what comes from elsewhere has to be held,
which is the one place where a key stores a value instead of a version ([ADR-0009][0009-pass-contract.md]).

Values are retained anyway, since a module that imports another needs its interface to check itself,
so keeping them as keys costs an `Arc` clone per entry, not a copy;
and because the driver compares values instead of replacing them,
validity is decided with `Arc::ptr_eq` —
as cheap as the integer comparison [ADR-0007][0007-vfs-file-state.md] asked for.
The price is retention:
a key holds the value it names, and the record does not survive the thing it records.

Invalidation then looks like this.
When a pull needs a value whose slot no longer matches the text it was built from,
the driver re-runs the local, cheap stages that lead to it —
the parse, the item tree, the interface, the scope —
at a cost of O(file), which is the edit that was paid for anyway.
Each recomputed value is compared with the retained one,
and if the two are equal the driver back-dates:
it keeps the retained value and drops the new one.
A dependent is then valid exactly when the values in its input
are the ones the slots currently hold, which is a set of pointer comparisons;
nothing was recomputed for the dependents, and nothing had to be.
A module that disappears invalidates them by the same rule.

This is the "recompute and back-date" trick a query engine needs `Eq` values for;
here it applies to a short, hand-picked prefix of the pipeline, the part that is cheap to re-run,
and the expensive values are guarded by recorded inputs.
A canonical fingerprint is still worth having, for tracing, for `--stats`,
and for a future on-disk cache; it is not part of the validity decision,
and it must be computed by an explicit function next to the value,
hashing names by their text rather than by their address.

### The interface is the cross-module key, and that is what makes incrementality pay

An **interface** is the observable projection of a module:
the names it exports, the shape of the items behind them, and the `impl`s it declares.
It is computed from the module's own text alone ([ADR-0004][0004-module-system.md], local name resolution),
which is why the whole scheme is stratified.

The interface is the only thing another module may read,
so a body edit cannot invalidate anything outside its own module:

- the body is not part of the interface,
  and a public signature is mandatory ([ADR-0004][0004-module-system.md]),
  so changing an implementation cannot change what a dependent sees;
- the `ItemTree` a body was lowered from does not contain the body,
  so the module's own `Interface` is re-derived but compares equal,
  and every dependent's recorded key still points at the `Arc` it pointed at before.

That property is paid for by the language restrictions [ADR-0004][0004-module-system.md] already accepted:
no glob imports, so a scope does not depend on another module's contents;
strict orphan rules, so the set of modules that can hold a relevant `impl` is closed;
mandatory public signatures, so a body is not observable.

Three constraints keep it true:

- the `ItemTree` stays a function of the module's items only:
  neither the body nor anything a body edit shifts may enter it,
  which leaves text ranges of items out of it
  (rust-analyzer keeps the same line: an item tree plus an `AstIdMap`, never the ranges);
- an interface holds keys, not foreign data:
  a name or a type that belongs to another module is stored as a reference,
  never a copy of what it refers to,
  and no other module is read while an interface is built;
- an id that points into another module therefore lives only inside a value
  whose key holds the interface that gives it meaning,
  and the value is re-derived when that interface changes.
  This is how [ADR-0003][0003-id-based-ir.md]'s "ids must not be kept across rebuilds"
  becomes something the driver enforces:
  the retained interface in the key is the generation counter.

### Granularity: the module first, the entity when it pays

Two nested knobs decide how much an edit re-runs:
which unit a slot covers, and how wide a single entry of a key is.

Today both sit at the coarsest value that still works:
a slot per file, per module and per owner ([ADR-0005][0005-compiler-pipeline.md]'s stages),
and inputs whose entries are whole interfaces rather than single entities of them.
When a module's public surface changes,
every module that read that interface is re-checked,
including the ones that never touched the item that changed.

Three things bound that cost:

- the propagation stops at the direct dependents.
  A module's interface is a function of its own text alone,
  because an interface holds keys rather than foreign data,
  so a module re-checked because of a neighbour's surface keeps its own interface,
  and nothing beyond its dependents is re-checked either;
- only checks are re-run — not parses, not lowerings, not the interfaces themselves;
- every import is named ([ADR-0004][0004-module-system.md]),
  so a module is imported for what it is used for,
  and its dependents are the modules that actually use it.

The fine setting is the same mechanism with narrower units and entries:

- a slot per item and per body, and inputs that name entities instead of whole interfaces;
- the blast radius of a signature edit becomes the readers of that entity.

Why it is not the starting point:

- it needs a stable identity for an item across edits,
  and `ModuleItemKey` is `(ModuleId, arena index)` ([ADR-0003][0003-id-based-ir.md]):
  positional, and explicitly not valid across rebuilds,
  so a slot keyed by one today would silently change meaning.
  The trick that produces such an identity — an id derived from the shape of the item tree,
  the way rust-analyzer's `AstIdMap` does it — does not exist here yet;
- the questions about a set — does this module export this name,
  can it answer an `impl` search — cannot be asked of a single entity,
  so the module-wide value does not disappear:
  the fine setting adds a record rather than replacing one;
- the coarse setting is what makes the mechanism itself
  — the input function, the back-dating, the conformance test —
  verifiable with the least machinery.

Both knobs belong to the driver:
narrowing them means putting narrower values into an input,
and the input function of a stage is the only thing that changes — not the passes, not the slots.
The conformance harness below is also the place to measure whether the coarse setting is enough.

### The pull graph is stratified, so there are no cycles to break

Ordered by what can read what:

1. A file's text determines its CST, and the CST determines the module's `ItemTree` and `Interface`.
   No other module is read.
2. The interfaces and the module graph determine each module's scope and the def map.
3. The def map and the interfaces determine the checked bodies.
4. The checked project determines thin-LTO and codegen ([ADR-0005][0005-compiler-pipeline.md], provisional).

There are no edges inside a level, and none that point back up:
the language forbids the constructs that would create them —
modules resolve names locally, `impl`s live in a closed set of modules,
and only interfaces are observable.
A module cycle is therefore a non-event for the driver:
two modules may import each other's names,
because lowering one needs nothing from the other.

Consequences:

- no cycle detection, no fixpoint iteration, no "query cycle" API;
- a per-module pull is genuinely local:
  it touches the bodies of no other module,
  and what it reads of other modules is their interfaces, which are small and usually already current;
- the driver never has to know the whole dependency graph,
  only the inputs it assembled for the units it was asked about.

### Global passes are indexes, not values

The def map is the one place where the pull model needs care.

If it were a single slot whose key is "every interface in the project",
then any change to any interface would invalidate everything that reads it,
and the locality of the previous sections would be gone.

Instead the def map is an index of per-module contributions
(the `ModuleScope` of [mlkc-hir-def]):

- each module's scope carries its own validity, decided by that module's own `Interface`;
- a consumer of the index — a host asking a whole-project question —
  is keyed by the modules whose entries it read,
  and a resolution that observes the shape of the project, such as an unresolved path or a virtual module,
  is keyed by the module graph it was resolved against;
- an update applies the diff of the changed scope
  rather than rebuilding the map ([ADR-0004][0004-module-system.md]).

The index is an accumulator rather than a value recomputed from scratch,
which is allowed, but with one rule:
an accumulator must equal what a rebuild would produce.
The conformance test below is what enforces that.

Note what the index is _not_ for:
a check does not read it.
A check resolves names against the interfaces in its own input,
which is what keeps a check's key local to what it actually uses.
The index answers whole-project questions — who exports a name, what a path resolves to —
and those questions are priced accordingly.

The same shape applies to the module graph and to the def map of a dependency project.

### Passes are pure functions of the inputs the driver assembles

The driver calls the passes, so a pass must not depend on the driver,
not even through a context type, or the crate graph stops being acyclic.
A pass takes data and returns data, and the driver builds the data.
The full contract — what a pass may do, what it must not, how an input is assembled —
is [ADR-0009][0009-pass-contract.md].

Two consequences matter for the table this ADR describes.

**The key of a slot names what the pass read.**
A pass cannot reach anything the driver did not hand it,
so what it read is covered by the key by construction:
by version for its own data, by identity for what came from other units.
Validity is then a set of pointer comparisons over values the driver owns;
there is no ledger, and nothing inside a pass has to be trusted to keep one.

**The input can be too small, and that is the obligation the driver takes on.**
What a stage reads is computed before the pass runs:
the closure of the modules the unit's own data names,
walked through the interfaces the walk itself reads,
with the language's closing rules applied on top —
named imports, and the closed set of `impl` providers the orphan rules give ([ADR-0004][0004-module-system.md]).
A module reachable only through a type reference is in the input
even if the pass never asks about it, which is conservative: a change there re-runs the check.

How a mistake here fails is worth being precise about.
The input is the only thing a pass can see,
so an omission is not a stale answer but a _missing_ one —
an unresolved path, a spurious diagnostic —
caught by the diagnostics snapshots on programs that must compile cleanly.

### Configuration is an input

Options, the target, and the file set change what a value means,
so they cannot be left out of the keys.

Today they enter as a single `ConfigVersion`:
a counter, bumped whenever a pushed change makes the configuration differ,
with a bump that drops the whole memo table.
Nothing is computed against two configurations at once,
and no key carries the configuration around.
Putting the configuration into each key waits for a host
that needs two configurations alive at the same time, which none does today.

### Cancellation, partial progress, and the write-once rule

A long pull has to be interruptible:
the next keystroke makes it obsolete, and a browser tab must not block.
In this model that is almost free.
Every value is written to its slot as soon as it is complete,
so an abandoned pull leaves a table full of valid values behind
and the next pull continues from there — nothing to unwind, no work to redo.
The matching rule is that a slot is written only when its value is complete:
a half-computed value is never published.

### How we know it is correct

Two things hold by construction:
a pull validates before returning, so an invalid value is never observed;
and the driver, not the pass, writes the key,
so a unit cannot read anything the key does not cover.
What does not hold by construction is the other direction:
the input has to contain what the pass needs,
and that rule of the language is written down once per stage ([ADR-0009][0009-pass-contract.md]).

Empirically, two kinds of mistake, with a different test for each.

A driver bug that serves a value that should have been recomputed —
a wrong comparison, a diff applied wrong in the index,
a back-date that kept a value whose input had changed — is caught by the conformance test:
build a project with a fresh driver, apply a generated sequence of edits to a long-lived one,
and after every step assert that every value the incremental driver hands out
equals the corresponding value of the fresh driver.
The edit sequences come from the patterns that stress the rules:
a body edit, a signature edit, an item added, removed and renamed,
a change to a type that another module's signature names,
an `impl` added in the third module the orphan rules allow,
a module added and deleted, and an edit that reverts an earlier edit.

A missing input is caught by the ordinary test suite instead:
the input is the only thing a pass can see,
so an omission turns into an unresolved path,
and the diagnostics snapshots on programs that must compile cleanly fail loudly.

The same harness gives the "no cache" option a job:
it is the oracle the conformance test compares against.

### Open questions, deliberately deferred

- Narrower units and entries: slots per item and per body, and inputs that name entities;
  the stable identity this needs, and why it comes second, are above.
- How wide the closure of a check turns out to be in practice,
  and whether entity-granular entries are needed to keep it small.
- Verify-once-per-revision:
  the standard engines mark a value as verified for a revision,
  so that validating one revision walks each edge once instead of once per question
  (salsa's red-green algorithm; the tutorial linked below is a readable implementation).
  Our comparisons are cheap and a revision is one pass over the table,
  so this waits for a profile to ask for it.
- The configuration in each key, instead of dropping the table when it changes.
- An on-disk cache for the CLI: canonical fingerprints, stable serialization,
  and never the source of truth the in-memory table is.
- A reverse index from a module to its dependents,
  so that "which files should the server re-publish" is answerable without validating the whole graph.
  Not needed for correctness; measure first.
- Memory policy under pressure, and the threading policy of the WASM build,
  including whether a cooperative, budgeted pull is worth having on top of a Web Worker;
  and whether a host that answers requests concurrently needs a shared, interior-mutable database
  — the shape a query engine typically takes, and the one the linked tutorial implements —
  rather than one owner thread that pulls and hands out `Arc`s.

### Positive Consequences

- One analysis, one driver, three hosts;
  what a host owns is exactly what is genuinely host-specific.
- The cost of an edit is bounded by the language rules, not by the shape of the dependency graph:
  a body edit re-parses and re-lowers one module, and invalidates nothing else.
- No runtime dependency graph, no cycle handling, no fixpoint solving, no query plumbing.
- Invalidity is decided by a pointer comparison,
  which is as cheap as the integer comparison [ADR-0007][0007-vfs-file-state.md] asked for.
- Cancellation is free, and partial progress is never lost.
- A pass is a function of a value, so it is tested by calling it,
  with no driver and no database in the way ([ADR-0009][0009-pass-contract.md]).
- The crate graph stays one-way, so the pipeline is reusable outside the driver.
- The failure mode of the language restrictions is visible:
  if a future feature makes an interface depend on another module's body,
  the cost shows up as O(project) rebuilds, not as a silent wrong answer.

### Negative Consequences

- The cutoff is as wide as a module interface:
  adding a public item re-checks every dependent of the module,
  including the ones that never used it.
  The propagation stops at the direct dependents,
  and narrower units are the recorded way out, postponed until stable identities exist.
- The input of a stage is a closure, so it names more interfaces than the pass will read,
  and a change anywhere in it re-runs the unit.
- The driver carries language knowledge the passes do not:
  what a stage reads is decided by the input function,
  and an omission there is a bug the passes cannot catch.
- Keys hold values, so a dependent pins the interfaces its input names,
  and a module cannot be evicted while dependents are alive.
- There is no cross-process or cross-run reuse, which the CLI will eventually want for cold builds.
- The model is implicit in a way the push model is not:
  what gets computed follows from what the host asks for,
  and a host that forgets to ask for something never gets it.
- `Eq` over the pipeline's values is pointer equality for interned names,
  exact only while the recorded value is retained:
  "equal" and "the same instance" are not separable concepts here.
- The def map is an accumulator, so it can drift,
  and only the conformance test sees a diff that was applied wrong.

## Pros and Cons of the Options

### Pull-model memo table with fingerprint keys

The shape that motivated this ADR:

```rust
struct Slot {
    text_hash: u64,
    parsed: Option<Parsed>,
    lowered: Option<Lowered>,
}

struct Lowered {
    hir: Arc<HirModule>,
    interface: Arc<Interface>,
    interface_hash: u64,                // hash of the public part only
    dep_hashes: HashMap<ModuleId, u64>, // fingerprints of the dependencies
    diagnostics: Vec<Diagnostic>,
}
```

Both memo tables share the pull model — work proportional to the edit and to the request,
a stratified graph, laziness, the host contract —
so the two entries weigh the keys alone.

- Good, because a key is eight bytes: small, printable, cheap to compare,
  and a rebuild reads like "the interface of `data` changed".
- Good, because a key does not pin the value it summarizes:
  the record survives an eviction of the value it names,
  at the price of recomputing that value to re-check it.
- Good, because it is the shape ML-family separate compilation puts on disk,
  so the same keys could serve a cross-run cache without a conversion step.
- Bad, because a summary can silently miss a part of the value,
  and that failure is a stale answer rather than extra work.
- Bad, because the fingerprints must be canonical and hand-written,
  since `#[derive(Hash)]` over this workspace's interned `Symbol` hashes an address, not a name.
- Bad, because a collision is a missed rebuild.

### Pull-model memo table with identity keys

Chosen. The same table, with the key of a slot being an exact identity,
back-dated when a recomputation produces an equal value.

- Good, because there is nothing to under-cover: the key _is_ the value.
- Good, because nothing has to be canonical:
  equality is the value's own `Eq`, and a value that cannot compare equal costs a recomputation.
- Good, because "why did this rebuild" can name what changed,
  since the driver holds both the old and the new value.
- Bad, because a key holds a value, so a dependent pins what it read.
- Bad, because `Eq` over interned names leans on retention to stay exact.
- Bad, because a compact, loggable, persistable key must be derived separately where one is wanted.

### Push-model worklist driver

Keep a dirty set; after every change, recompute it eagerly in topological order.

- Good, because the state of the world is always known, and "check everything" is one call.
- Good, because parallelism is the frontier of a topological order.
- Good, because reads are always cheap and never compute.
- Bad, because a keystroke in a large project starts work nobody asked for,
  and the first diagnostic waits for all of it.
- Bad, because a long sweep has to be checkpointed, cancelled and resumed to stay interactive,
  which is the pull model with more state.
- Bad, because the dirty set needs a reverse-dependency index —
  the runtime graph [ADR-0004][0004-module-system.md] set out to avoid —
  or a conservative superset, which invalidates more than necessary.
- Bad, because the language server needs the pull side anyway.

### Query engine

Salsa-style: arbitrary queries, dependencies recorded at runtime.

- Good, because cutoff granularity is the granularity of the queries, finer than an interface.
- Good, because dependency recording is automatic.
- Good, because cycle handling, memoization GC, and concurrent queries exist already.
- Bad, because [ADR-0004][0004-module-system.md] rejected it:
  the runtime graph is hard to profile and to reason about.
- Bad, because its hardest features — cycles, fixpoints, nested parallel queries, edge GC —
  are the ones the stratified pipeline above makes unnecessary.
- Bad, because the engine does not remove the design work:
  where the queries are cut is still a hand decision,
  and the engine's default cuts are not the language's cuts.
- Bad, because it is a large dependency in the crate everything else depends on.

### File-level incrementality

Reuse a value while the text it came from is unchanged; no interfaces in the keys.

- Good, because it is much simpler: no interface concept, no inputs to assemble.
- Good, because `FileVersion` already gives exact identities for free ([ADR-0007][0007-vfs-file-state.md]).
- Bad, because a body edit changes the text, so every transitive dependent is re-checked,
  and the cost of an edit follows the module's fan-out, not the edit.
- Bad, because it does not use the language restrictions [ADR-0004][0004-module-system.md] paid for.

### No cross-revision cache

Re-run the pipeline over the whole project per request; memoize within one revision only.

- Good, because it is obviously correct and needs no keys.
- Good, because it is the oracle the conformance test compares against.
- Bad, because a keystroke costs a full rebuild, which is the requirement at the top of this ADR.
- Bad, because a language server on top of it would grow its own ad-hoc invalidation.

### Separate compilation on disk

The compiler compiles one module against serialized interfaces; an external tool schedules.

- Good, because the interface is a real artifact:
  checksummed, portable, reusable across runs and machines.
- Good, because it has decades of precedent (OCaml's `.cmi`, GHC's `--make`).
- Bad, because the language server cannot use it: the analysis happens in-process, on unsaved buffers.
- Bad, because the browser cannot use it either: no file system, no process to schedule.
- Bad, because it needs a stable serialization format for interfaces now.
- Bad, because the same rules would be implemented twice, in the build tool and in the server.

Its idea, the interface checksum, is the one this ADR adopts in memory.

## Links

- No query system: [0004-module-system.md]
- Pipeline and stages: [0005-compiler-pipeline.md]
- File contents and versions: [0007-vfs-file-state.md]
- Arenas and invalidation units: [0003-id-based-ir.md]
- Lossless syntax tree: [0002-lossless-syntax-tree.md]
- Tree dumps: [0006-snapshot-testing.md]
- Pass contract: [0009-pass-contract.md]
- A query engine and a language server built out of passes, as a tutorial:
  the red-green algorithm, the two cursor queries every IDE feature builds on
  (position into a node, node into a typed node),
  and diagnostics published with the document version: <https://thunderseethe.dev/posts/lsp-base/>
- Against Query Based Compilers: <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>
- Three Architectures for a Responsive IDE: <https://matklad.github.io/2023/12/28/three-architectures-for-responsive-ide.html>
- Zig's incremental compilation internals: <https://mlugg.co.uk/posts/incremental-compilation-internals/>
- Implementation: [mlkc-driver], [mlkc-vfs], [mlkc-hir-def], [mlkc-diagnostics], [mlkc-cli]

[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[0003-id-based-ir.md]: 0003-id-based-ir.md
[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0009-pass-contract.md]: 0009-pass-contract.md
[mlkc-driver]: ../../crates/mlkc-driver
[mlkc-vfs]: ../../crates/mlkc-vfs
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-diagnostics]: ../../crates/mlkc-diagnostics
[mlkc-cli]: ../../crates/mlkc-cli
