# Let every stage own its spec suite

- Status: accepted
- Date: 2026-10-07

## Context and Problem Statement

The snapshot suite of `mlkc-driver` compiled every project of the corpus and dumped every stage
into one snapshot per project ([ADR-0006](../adr/0006-snapshot-testing.md)).
One file mixed the lowering, the resolution, the types, the MIR, the LIR, and the diagnostics,
so the snapshot of a project ran to a thousand lines,
and a change to one stage showed its diff in a crate the change never touched.

Where does the snapshot of a stage live, and how does the test that makes it reach the pipeline?

## Considered Options

- **Keep the mixed snapshot**, splitting the file per stage inside the driver suite.
- **Move each stage's snapshot to the crate that owns the stage**,
  with the driver as a dev-dependency and one shared corpus of projects.
- **Move the snapshots, and let each crate keep its own corpus of fixtures.**
- **Build a test-support crate** that holds the harness and the corpus.

## Decision Outcome

Chosen option: "Move each stage's snapshot to the crate that owns the stage",
because a stage's diff then belongs to the crate that changed,
and because two suites already worked that way (`mlkc-codegen-wasm`, `mlkc-interp`).

### What the suites look like

- The projects the pipeline suites compile are one corpus, `crates/mlkc-fixture/projects`,
  read by `mlkc_fixture::{projects, read}`.
  Every suite asserts its own stage over every project of the corpus,
  and fails when a project has no test,
  so a project added there is covered by every stage at once.
- A suite drives the corpus through `mlkc-driver`, a dev-dependency of the crate:
  the _library_ of a pipeline crate never reaches back into the driver,
  the spec suite does (the `pipeline-pass` skill).
- `mlkc-lower`, `mlkc-resolve`, `mlkc-typeck`, `mlkc-mir-build`, and `mlkc-lir-wasm` keep the
  suite of their stage, next to the direct tests of the pass;
  the driver keeps what it renders, the diagnostics, and the check that no pass bugged.

### Positive Consequences

- A change to a stage diffs the snapshots of that stage's crate, and nothing else.
- No snapshot mixes stages: a file holds one stage of one project.
- The corpus has one home,
  so a project cannot be covered by one stage and missed by another.
- A suite can grow fixtures of its own once a stage outgrows the corpus.

### Negative Consequences

- The dependency graph gains test-only cycles:
  a pipeline crate's dev-dependencies reach the driver that depends on it.
  Cargo allows this; the library direction stays what it was.
- The harness --- the driver, the standard library, the corpus, the ICE check, about a hundred
  lines --- is repeated in every suite.
- The suites compile the pipeline once each instead of once in total.

## Pros and Cons of the Options

### Split the file per stage inside the driver suite

- Good, because it touches no dependency and no crate layout.
- Bad, because the snapshots stay in a crate whose author does not open them,
  and the driver keeps rendering dumps it does not produce.

### One corpus per crate

- Good, because a stage's fixtures are tailored to the stage.
- Bad, because the same project is copied into several crates and drifts;
  the shared corpus is what keeps every stage asserted over every project.

### A test-support crate

- Good, because the harness lives once.
- Bad, because a crate exists only for tests,
  and every suite depends on it instead of on the driver directly.

## Links

- [ADR-0006](0006-snapshot-testing.md): snapshot testing, the strategy these suites follow.
- [ADR-0009](0009-pass-contract.md): the pass contract the direct tests assert.
- [The map of the tests](../architecture/README.md), and the `snapshot-tests` skill.
