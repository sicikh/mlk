---
name: snapshot-tests
description: "Use when adding, running, or updating snapshot tests in mlkc: the fixture formats, the spec harnesses of parser, lowering, driver, and codegen, and the insta update-and-review workflow (ADR-0006)."
---

# Snapshot tests

Snapshot testing is the primary test strategy (`docs/adr/0006-snapshot-testing.md`).
A snapshot captures a whole output — a dump, a rendered diagnostic, a printed module — so every
output change becomes a reviewable diff.
`insta.yml` sets `behavior.update: new`: a changed snapshot is written as `.snap.new` and fails
the run until it is accepted.

## The suites

| Suite                                                   | Fixtures                                                       | The snapshot holds                                                              |
| ------------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `crates/mlkc-parser/tests/specs/{valid,invalid}`        | one module per `.mlk` file                                     | source, AST, CST, parse diagnostics                                             |
| `crates/mlkc-lower/tests/specs/{valid,invalid}`         | one module per `.mlk` file                                     | item tree, bodies, lowering and parse diagnostics                               |
| `crates/mlkc-driver/tests/specs`                        | a file in the `mlkc-fixture` format, or a directory of modules | every driver stage: surfaces, scope, types, MIR in both forms, LIR, diagnostics |
| `crates/mlkc-codegen-wasm/tests/specs` (+ `tests/runs`) | one module per `.mlk` file                                     | the printed WASM module, and the output of a run                                |

Each suite has a harness (`*_test.rs`, `project_spec.rs`) and a list (`*_specs.rs`).
The list is the source of test names: **a new fixture needs a line there**, and the suite fails
when a fixture on disk has no entry.
The names are spelled out, not derived, so the review sees them when a fixture changes.

## The fixture format

- A single-module fixture is a plain `.mlk` file.
- A project fixture is one file in the `mlkc-fixture` format: every module is headed by its place,
  `//- /main.mlk`, and what follows is its source.
  A directory fixture holds one file per module instead; the path under the directory is the
  module's place.
- Fixtures are small and self-contained; a fixture that outgrows one file becomes a directory.

## Adding a fixture

1. Write the `.mlk` file under the right `specs/valid` or `specs/invalid` directory
   (`invalid` is for a mistake the parser or the lowering is expected to recover from).
2. Add a line to the suite's `*_specs.rs` with a one-line comment that says what the fixture
   shows; match the style of the other entries.
3. Run the suite; accept the new snapshot with `INSTA_UPDATE=always cargo test -p CRATE`.
4. Review the snapshot as part of the change: it is the test.

## Updating snapshots

- `INSTA_UPDATE=always cargo test -p CRATE` rewrites them in place.
- `just test-review` runs the suite and offers an interactive review
  (`cargo insta test --review`).
- Never hand-edit a `.snap`; never leave a `.snap.new` behind.
- A snapshot diff is a behavior change: if it is not the change you meant, fix the code, not the
  snapshot.

## Where snapshots do not belong

Small algorithmic units keep ordinary unit tests, and invariants over arbitrary inputs belong in
property tests (see the lexer's `quickcheck` tests in `crates/mlkc-parser/src/lexer/tests.rs`).
Snapshots are for the whole outputs of the pipeline.
