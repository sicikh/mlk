# Define the contract of a compiler pass

- Status: accepted
- Date: 2026-09-22

## Context and Problem Statement

[ADR-0005][0005-compiler-pipeline.md] names the stages of the pipeline,
and [ADR-0008][0008-compiler-driver.md] says the driver calls them and memoizes them,
but neither says what a single _pass_ looks like from the inside:
what it takes, what it returns, and what it may know about the world.

Without a contract, every pass invents its own shape:
one fetches its own dependencies, one renders its own messages,
one keeps a handle to the file system,
and the driver accumulates a special case per pass.

The question:
what must a pass satisfy
so that it can be memoized by comparing inputs,
tested without a driver around it,
and added without touching anything but the driver's list of stages?

## Decision Drivers

- Direction of the crate graph:
  the driver calls the passes ([ADR-0008][0008-compiler-driver.md]),
  so a pass that reaches back into the driver makes the graph cyclic.
- Purity as a load-bearing property, not a style:
  the driver decides validity by comparing input values ([ADR-0008][0008-compiler-driver.md]),
  which is only meaningful if a pass is a function of its input and of nothing else.
- Testability:
  a pass must be testable with no driver, no editor, and no file system,
  which is what [ADR-0006][0006-snapshot-testing.md]'s snapshot strategy relies on.
- Diagnostics as data:
  a pass produces facts with spans, not text ([ADR-0007][0007-vfs-file-state.md]).
- Cheap to add:
  a new pass should be a function plus a description of what it reads,
  not an integration project.

## Considered Options

- **A pure function of an assembled input** —
  the driver gathers the values and calls `f(input) -> (output, diagnostics)`.
- **A function over a context** —
  salsa-style: the pass pulls what it needs from a database it is handed.
- **A resumable pass** —
  the pass runs until it needs a value, returns the request,
  and is called again once the driver has it.

## Decision Outcome

Chosen option: "A pure function of an assembled input",
because it is the only one of the three
that both keeps the crate graph acyclic
and makes the pass's input exactly the state the driver compares.

### The shape

```rust
// In the crate that owns the data. Nothing here knows that a driver exists.
pub fn lower_module(module: ModuleId, syntax: &SyntaxNode) -> (ItemTree, Vec<LoweringDiag>);

pub fn lower_body(
    owner: DefWithBodyKey,
    body: &SyntaxBody,
    item_tree: &ItemTree,
) -> (Body, Vec<LoweringDiag>);

pub fn check_body(
    owner: DefWithBodyKey,
    item_tree: &ItemTree,
    body: &Body,
    deps: &CheckDeps,
) -> (CheckedBody, Vec<TypeDiag>);

/// The part of a check's input that belongs to other units.
/// The driver retains it, and compares it when it asks whether this result is still current
/// ([ADR-0008][0008-compiler-driver.md]).
pub struct CheckDeps {
    /// The interfaces this check is allowed to read.
    pub interfaces: Interfaces,
    pub graph: Arc<ProjectGraph>,
}
```

A pass is:

- a free function in the crate that owns the data it produces;
- total over its input:
  invalid or partial input yields diagnostics, never a panic
  (bogus nodes exist for this, [ADR-0002][0002-lossless-syntax-tree.md]);
- deterministic, in the sense defined below;
- blind to everything but its input:
  no `Vfs`, no clock, no environment, no globals, no I/O, no threads;
- synchronous:
  parallelism is the driver dispatching one pass over a set of units,
  not a pass spawning work.

Two conveniences are deliberate:

- the input is cheap to build — it is `Arc`s and small maps, not a deep copy;
- the output is owned and does not borrow from the input,
  so the driver can store it and hand out `Arc`s ([ADR-0007][0007-vfs-file-state.md]).

### What goes into the signature, and what goes into the input struct

Three kinds of data reach a pass, and the difference between them
is whose value it is:

| Data                                                     | Comes in as                                                     | Why                                                                                                                                                                                                        |
| -------------------------------------------------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the unit's identity — `ModuleId`, `DefWithBodyKey`       | a `Copy` argument                                               | it selects the unit; there is nothing to retain or to compare                                                                                                                                              |
| the unit's own data — the CST, the item tree, the body   | a plain `&` argument                                            | derived from this unit's own text, so its identity _is_ the text's identity: the driver's `FileVersion` for that file covers it ([ADR-0007][0007-vfs-file-state.md]), and a version needs nothing retained |
| data of other units — their interfaces, the module graph | fields of one struct of owning `Arc`s, which the driver retains | nothing cheap identifies it across revisions, so the value itself has to be held and compared ([ADR-0008][0008-compiler-driver.md])                                                                        |

In one sentence:
**one's own data is borrowed and keyed by a version;
another unit's data is owned, and is its own key.**

Why the third kind has to own:

- the key of the slot holds it, so it outlives the call and survives revisions;
  a reference cannot, and retention is also what keeps ids inside the value meaningful ([ADR-0003][0003-id-based-ir.md]);
- the driver may hand the input to a worker, and a moved input needs no scoped threads;
- hence `Arc` rather than `Rc`.

A borrowing variant of the third kind — the driver keeps the key
and the pass takes `CheckDeps<'a>` — was considered and rejected:

- it saves nothing: the key owns the same `Arc`s and pays the same clones, one struct earlier;
- it costs a lifetime parameter on the input type,
  which propagates into every helper of every pass that manipulates it;
- it splits one type into two, an owned key and a borrowing view, kept in step by hand;
- it makes an input awkward to build by hand, and building inputs by hand is what a snapshot test does.

Where a version stops being enough:
a version identifies a file, so it is an exact key for a slot whose unit is a file.
A slot finer than a file — a body, an item — needs a finer identity for its own data,
which is the deferred item of [ADR-0008][0008-compiler-driver.md].

### What a pass reads is the driver's business

A pass declares nothing about its dependencies.
The driver builds the input before the pass runs,
and what the pass read is recorded twice over:
by version for its own data, by identity for everything it was handed from elsewhere
([ADR-0008][0008-compiler-driver.md]).

The consequences are worth stating plainly:

- a key cannot be too small with respect to what the pass read,
  because the pass cannot reach anything else;
- the driver has to know what the pass will read, and that rule lives in the input function of the stage;
- a pass has no way to ask for more, so an input function is the only place where a missing dependency can hide.

How an input function is meant to be written:

- it starts from the unit's own data — a file's text, a module's item tree, a body — and nothing else;
- the cross-module part is the closure of the modules that data names,
  walked through the interfaces the walk itself reads;
- the closing rules of the language are part of it:
  named imports, and the closed set of `impl` providers that the orphan rules give ([ADR-0004][0004-module-system.md]);
- it lives in the driver,
  while the pure helpers it is built from — "which modules does this item tree name" —
  live next to the pass, in the pipeline crate.

A missing entry is a bug in the input function, and it fails loudly:
the pass cannot resolve something, so it reports an unresolved path,
and a snapshot test on a program that must compile cleanly turns that into a failure.

### A pass does not render

A pass returns diagnostics in the shape it knows them:
types with spans and the data a message needs, one family per pass.

- The driver turns them into the host's `Diagnostic`,
  because rendering needs the text of the file ([ADR-0007][0007-vfs-file-state.md]).
- No formatted strings, no colours, no line/column ranges, no protocol types inside a pass.
- One type family per pass is deliberate:
  a single project-wide error enum
  would make every pass depend on the diagnostics of every other pass
  (the tutorial linked below does it the other way,
  and pays with one enum that knows about every pass).
  Collecting diagnostics into one type for the host is the driver's job.

### A pass does not choose its own granularity

A pass covers the unit it was written for:
a file, a module, an item, a body, a function.

Finer units — a body slot inside a module, an interface per entity —
need stable identities across edits, which do not exist yet ([ADR-0008][0008-compiler-driver.md]).
Until they do, a pass is written for the unit the driver can key,
and splitting a pass into smaller ones is a change in the driver's stage list,
not in the pass's contract.

### Determinism, and what "the same output" means

Validity is decided by comparing input values ([ADR-0008][0008-compiler-driver.md]),
so "the same input gives the same output" has to be a property of the pass, not of luck:

- no iteration over a `HashMap` may reach the output in an order that matters;
- nothing derived from an address, a pointer,
  or the order in which names happened to be interned may enter the output
  (this workspace's `Symbol` compares and hashes by address, [ADR-0008][0008-compiler-driver.md]);
- a pass must not depend on the order in which the driver runs other passes.

### How a pass is tested

- Snapshot tests ([ADR-0006][0006-snapshot-testing.md]) over hand-built inputs:
  the pass is called directly, with no driver and no file system in the way.
- Invalid inputs are snapshots too: a parser's recovery, a checker's errors.
- The conformance harness of [ADR-0008][0008-compiler-driver.md] is not the pass's test:
  it tests the driver's validity decisions, not the pass's logic.

### Positive Consequences

- A new pass is a function, an input struct, and an entry in the driver's stage list.
- Passes are testable in isolation, which keeps snapshot tests cheap to write.
- The input of a pass documents exactly what it depends on,
  in a form both reviewers and the driver can read.
- No trait, no database object, no context plumbing in the pipeline crates.
- The pipeline crates stay reusable outside the driver,
  by tests, by a batch tool, or by a future consumer.

### Negative Consequences

- The driver carries knowledge about the passes:
  what each stage reads is written outside the pass that reads it,
  and keeping the two in step is a review obligation, not a type-checked one.
- An input is a closure and therefore wider than what the pass uses:
  a change anywhere in it re-runs the unit even if the pass never looked there.
- Assembling an input costs a walk over the named modules
  before the pass is even called.
- Some passes are awkward to express without a context:
  a pass that would naturally explore a graph on demand
  has to be given the graph up front or split into stages.

## Pros and Cons of the Options

### A pure function of an assembled input

Chosen. `f(input) -> (output, diagnostics)`.

- Good, because the crate graph stays one-way: driver → pipeline, never back.
- Good, because the key of a slot is the identity of what the pass read:
  a version for its own data, the retained values for what it read elsewhere.
- Good, because a test constructs an input and calls the function.
- Bad, because the dependency set has to be known before the pass runs,
  so it is conservative and lives outside the pass.
- Bad, because a pass cannot ask a question in the middle of its work;
  a pass whose exploration is genuinely demand-driven must be reshaped.

### A function over a context

The pass takes `&dyn Db` and pulls what it needs, as rust-analyzer's `hir-def` and rustc's `TyCtxt` do.

- Good, because reads are exact and lazy:
  nothing is gathered that the pass does not use.
- Good, because no input function has to know the dependency rules in advance.
- Good, because it scales to passes that chase types to an unknown depth.
- Bad, because the context type has to live below the passes,
  and every pass's needs become part of one shared interface that grows with the language.
- Bad, because the pass is no longer a function of a value:
  testing it means implementing the context, and reviewing it means knowing what it may pull.
- Bad, because the driver loses the property that a key is what the pass was handed:
  what was read becomes a claim of the pass rather than a fact about its input.

### A resumable pass

The pass runs until it needs a value, returns the request, and resumes with the answer.

- Good, because reads are exact and the pass stays a pure step function of `(input, answers)`.
- Good, because no context object and no shared database interface are needed.
- Bad, because a pass becomes a state machine or a re-entrant function,
  and the driver has to drive the request loop.
- Bad, because a naive implementation redoes work on every resume,
  and a careful one is significantly more machinery than a function call.

## Links

- Pipeline and stages: [0005-compiler-pipeline.md]
- Snapshot testing: [0006-snapshot-testing.md]
- Spans, versions, and what the driver renders: [0007-vfs-file-state.md]
- The driver that calls passes: [0008-compiler-driver.md]
- Arenas, ids, and why a retained value is what keeps an id meaningful: [0003-id-based-ir.md]
- Bogus nodes and recovery: [0002-lossless-syntax-tree.md]
- Module rules the input functions rely on: [0004-module-system.md]
- A pass-shaped query engine, written as a tutorial: <https://thunderseethe.dev/posts/lsp-base/>
- rust-analyzer's context approach: <https://github.com/rust-lang/rust-analyzer/blob/master/crates/hir-def/src/lib.rs>

[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[0003-id-based-ir.md]: 0003-id-based-ir.md
[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
