# Use snapshot testing as the primary test strategy

- Status: accepted
- Date: 2026-09-20

## Context and Problem Statement

A compiler is a text transformation machine:
source code goes in,
and trees, types, and diagnostics come out.

The outputs are large, structured, and text-heavy:
CST dumps, HIR dumps, rendered diagnostics.
Asserting them field by field is tedious,
easy to get subtly wrong,
and blind to the parts we forgot to assert.

We need a test strategy where

- capturing a whole output is nearly free;
- every change to an output becomes a visible, reviewable diff;
- updating an expected output is a deliberate decision, not an accident.

## Decision Drivers

- Cheap to write.
- Complete: the entire output is captured, not a few hand-picked fields.
- Reviewable: output changes surface in the diff and are reviewed like code.
- Deliberate updates: expected outputs change only through explicit acceptance.
- Precedent: rust-analyzer and biome snapshot-test
  their syntax trees and diagnostics with insta.

## Considered Options

- **Hand-written assertions** — compare a few selected fields or properties.
- **Golden files maintained by hand** — plain text files compared in tests.
- **Snapshot testing with cargo-insta** — macro-captured snapshots
  with a review-and-accept workflow.

## Decision Outcome

Chosen option: "Snapshot testing with cargo-insta",
because it makes capturing whole outputs as cheap as a single macro call
and turns every output change into a reviewable diff.

### What is snapshot-tested

Snapshot tests are the primary strategy
for everything the pipeline emits:

- CST dumps from the parser
  (and the typed AST view over them);
- HIR dumps: item trees, bodies, resolved paths;
- rendered diagnostics: messages with spans and severity;
- textual dumps of MIR and of generated WASM.

Test inputs are small, self-contained MLK snippets,
so that every snapshot diff stays readable.

### What is not snapshot-tested

- Small algorithmic units (interner, arena, text sizes)
  keep ordinary unit tests.
- Invariants that hold for arbitrary inputs
  are better served by property tests (quickcheck),
  not by snapshots.

### The workflow

- A test captures its output with `assert_snapshot!`
  (or `assert_debug_snapshot!` for debug dumps).
- `cargo insta test --review` runs the suite
  and offers to accept pending changes interactively.
- Accepted changes land in the repository like code,
  reviewed as part of the diff.
- CI runs the suite with snapshot updates disabled:
  any difference is a failure.

### Positive Consequences

- Writing a test is a single macro call,
  so tests actually get written.
- A regression shows up as a readable diff
  of a complete dump,
  not as a cryptic assertion message.
- Accepting or rejecting a change
  becomes an explicit review decision.
- The format is uniform across all crates,
  which eases onboarding.

### Negative Consequences

- Large snapshots make diffs noisy;
  keeping test inputs small mitigates this.
- Snapshots assert the _current_ behavior:
  a bug can be blessed into a snapshot
  if the review is careless.
- The workflow depends on the cargo-insta tool
  (installed via `just install-tools`).
- Snapshots can rot when an output format changes
  without a semantic change;
  the review step is where this is caught.

## Links

- cargo-insta documentation: <https://insta.rs/docs/>
- Configuration: [insta.yml]
- Tooling: [justfile]

[insta.yml]: ../../insta.yml
[justfile]: ../../justfile
