# Store file contents and versions in the VFS

- Status: accepted
- Date: 2026-09-21

## Context and Problem Statement

`mlkc-vfs` is vendored from rust-analyzer,
where the VFS is not a database but a channel:
it records the changes pushed to it by the loader (or by an editor),
together with a content hash and the new text,
while the real state of a file lives in salsa.

MLK has no query engine ([ADR-0004][0004-module-system.md]),
so nobody else stores the text,
and the state of a file --
its contents, its existence, its version --
has to be owned by some component of the compiler.

The question: where does the current state of a file live,
if there is no query database to put it in?

## Decision Drivers

- One source of truth:
  the contents of a file must not be stored twice,
  in a change log and in a store that can drift apart.
- External updates:
  state is pushed in from the outside,
  by a file watcher, by an editor, or by a WASM host,
  and none of them should need a different code path.
- Parallelism:
  parsing and analysis run per module in parallel ([ADR-0005][0005-compiler-pipeline.md]),
  so workers must be able to read the text of a file
  without holding a lock on the VFS.
- Cheap invalidation:
  "is this cached result still valid?" must be an integer comparison,
  not a re-hash or a re-read of the file.
- Precedent:
  the loader, the `FileSet` partitioning, and path interning
  are kept as they are; only the ownership of the state changes.

## Considered Options

- **Change channel only** — the vendored shape:
  the VFS keeps hashes and pending changes,
  a separate source database owns the text.
- **VFS owns the state** — the VFS stores the contents
  of every file it knows, and hands them out with a version.
- **No VFS** — the driver owns a map from paths to text,
  the crate is dropped.

## Decision Outcome

Chosen option: "VFS owns the state",
because with no query engine to hold the inputs,
splitting a file into a change channel and a separate store
would only recreate a query database without the queries.

### What the VFS stores

For every file it has ever seen:

- the `FileId` and the path it was interned under;
- what the file contains: its text, or that it is missing,
  cannot be read as text, or is excluded;
- a `FileVersion`, bumped on every change.

Existence and contents are one value,
so a file that cannot be read
cannot accidentally be treated as a file that happens to be empty.

The contents are read with `file_text`,
which hands out a shared handle that outlives the borrow of the VFS,
and they are immutable for as long as their version lives:
replacing the contents creates a new `Arc`,
it never mutates the old one.

`take_changes` is kept,
but it is now an event sink rather than the carrier of the text:
it reports the **net** effect per file since the last call,
which is exactly the work list of the driver.

### Versions instead of hashes

The vendored VFS stores a hash per file
and hands the hash and the contents to salsa,
which compares the hashes to decide what changed.

Once the VFS owns the contents, the hash is redundant:
deciding that the contents are the same
is comparing the text,
and the text has to be validated as UTF-8 anyway,
which is the same O(n) pass over the file.
The identity of a state is a version counter instead,
which cannot collide and is O(1) to compare.

The version is bumped on every change,
including a change back to contents that the compiler has already seen:
a content hash would let a cache recognize a file that returned
to a state it was in before,
while a version simply says "this is a different state now".
We accept the extra work in exchange for one identity per file,
not two.

### What invalidation looks like

- The driver drains `take_changes` and gets at most one `ChangedFile`
  per affected file, with the version the file has now.
- Every derived value (a parse, a module surface, an item tree)
  is stored next to the `FileVersion` it was derived from,
  and is revalidated by comparing versions.
- Nothing is hashed on the hot path,
  and a file whose contents were pushed again unchanged
  keeps its version: a watcher may re-read a whole project
  and invalidate nothing at all.

### Ranges are resolved inside the revision that produced them

A `Span` is a file id and a range, with no version attached:
the range belongs to the text the file had when the span was created.
That is sound as long as a span is resolved against the state it was produced from,
which the VFS makes cheap:

- a revision reads the world through a single `Snapshot` of the VFS:
  a shared borrow that keeps the state from changing
  while the spans are produced and resolved;
- the contents of a version never change,
  so an `Arc<str>` handle taken from a snapshot
  stays the text of that version for whoever holds it,
  including a worker thread that parses it in parallel;
- derived data that outlives the revision is keyed by file version,
  so it is revalidated or recomputed,
  and diagnostics that outlive it are published with the version of the document.

A version inside `Span` was considered and rejected:
it would enlarge every span and every comparison of spans
to protect against a mismatch that must not happen,
and it still would not say which text to resolve against,
only that the resolver holds the wrong one.
rustc solves the same problem by keeping the source map immutable for a session,
and rust-analyzer by making every derived value a function of an input revision.

### What a file that cannot be read looks like

MLK sources are text, so contents that are not valid UTF-8
leave the file in the `Unreadable` state,
which the driver can report as a diagnostic.
It is not silently treated as a missing file:
the VFS knows the file is there, and the compiler cannot compile it.

### What is deliberately not a change

Excluding a file (`insert_excluded_file`)
is a property of the configuration of the loader,
not of the contents of the file,
so it is not reported by `take_changes`:
the driver is the one that excluded the file, and it knows it.

### Positive Consequences

- One source of truth:
  there is no second copy of the text to keep in sync,
  and no component except the VFS decides what a file contains.
- Parallelism is free:
  `Arc<str>` can be cloned into a worker thread,
  and the main loop keeps updating other files meanwhile.
- Invalidation is an integer comparison.
- Pushing identical contents costs one comparison and invalidates nothing,
  which makes "reload the project" a cheap operation downstream.
- Natively, in an editor, and in WASM the flow is the same:
  somebody outside pushes text.
- The loader, the `FileSet` partitioning, and interning stay as vendored.

### Negative Consequences

- The VFS holds all sources in memory.
  For a compiler this is acceptable --
  diagnostics and the LSP need the text anyway --
  but it is a real cost for a project with huge sources.
- The VFS is mutable state that is not thread safe:
  mutation happens in the driver,
  and only the `Arc<str>` handles cross thread boundaries.
- The content hash is gone.
  If we later want content-addressed caches,
  or a way to skip the O(n) comparison on save,
  the hash has to come back.
- The change stream is a net-effect log, not a journal:
  a file that appears and disappears between two drains
  is not reported at all,
  and the history of a file cannot be replayed from it.
- `FileId`s are never reused and deleted files keep their slot,
  so the id space grows with the number of paths ever seen.
- Nothing in the type of a `Span` says which revision it belongs to:
  the discipline is carried by the snapshot and by version-keyed invalidation,
  not by the span itself.

## Pros and Cons of the Options

### Change channel only

- Good, because the vendored VFS stays as small as it is.
- Good, because a separate store can be shaped by its consumer
  (for example, to keep derived data next to the text).
- Bad, because two components hold the state of a file
  and have to be kept in agreement.
- Bad, because every reader needs the store:
  the VFS alone no longer answers "what does this file contain?".
- Bad, because the store is an input table with change tracking --
  a query database without the queries.

### VFS owns the state

- Good, because the state of a file is decided in exactly one place.
- Good, because `Span`s ([mlkc-span]) can be resolved to text
  with the same handle that parses the file.
- Bad, because the VFS grows from a channel into a store,
  and the vendored crate diverges from its upstream.

### No VFS

- Good, because it is the fewest moving parts while the compiler is small.
- Bad, because path interning, `FileId`s, the loader,
  the watcher and the file set partitioning
  would have to be re-implemented in the driver.

## Links

- No query system: [0004-module-system.md]
- Pipeline and parallelism: [0005-compiler-pipeline.md]
- Implementation: [mlkc-vfs]
- `Span`: [mlkc-span]
- Vendored from: <https://github.com/rust-lang/rust-analyzer/blob/master/crates/vfs/src/lib.rs>

[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[mlkc-vfs]: ../../crates/mlkc-vfs
[mlkc-span]: ../../crates/mlkc-span
