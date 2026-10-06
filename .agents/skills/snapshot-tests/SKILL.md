---
name: snapshot-tests
description: "Use when adding, running, or updating snapshot tests in mlkc: the fixture format, the project corpus, the spec harnesses of every stage, and the insta update-and-review workflow (ADR-0006)."
---

# Snapshot tests

Snapshot testing is the primary test strategy (`docs/adr/0006-snapshot-testing.md`).
A snapshot captures a whole output — a dump, a rendered diagnostic, a printed module — so every
output change becomes a reviewable diff.
`insta.yml` sets `behavior.update: new`: a changed snapshot is written as `.snap.new` and fails
the run until it is accepted.

## The suites

| Suite                                                   | Fixtures                                     | The snapshot holds                                                          |
| ------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------- |
| `crates/mlkc-parser/tests/specs/{valid,invalid}`        | one module per `.mlk` file                   | source, AST, CST, parse diagnostics                                         |
| `crates/mlkc-lower/tests/specs/{valid,invalid}`         | one module per `.mlk` file                   | item tree, bodies, lowering and parse diagnostics                           |
| `crates/mlkc-codegen-wasm/tests/specs` (+ `tests/runs`) | one module per `.mlk` file                   | the printed WASM module, and the output of a run                            |
| `crates/mlkc-lower/tests/specs`                         | the corpus of `crates/mlkc-fixture/projects` | item tree, bodies                                                           |
| `crates/mlkc-resolve/tests/specs`                       | the corpus of `crates/mlkc-fixture/projects` | the scope every module resolved to                                          |
| `crates/mlkc-typeck/tests/specs`                        | the corpus of `crates/mlkc-fixture/projects` | type surfaces, checked bodies                                               |
| `crates/mlkc-mir-build/tests/specs`                     | the corpus of `crates/mlkc-fixture/projects` | MIR in both forms                                                           |
| `crates/mlkc-lir-wasm/tests/specs`                      | the corpus of `crates/mlkc-fixture/projects` | LIR, and the structure of a body                                            |
| `crates/mlkc-driver/tests/specs`                        | the corpus of `crates/mlkc-fixture/projects` | the diagnostics the driver renders, and the pipeline running without an ICE |

Each suite has a harness (`*_test.rs`, `project_spec.rs`) and a list (`*_specs.rs`).
The list is the source of test names: **a new fixture needs a line there**, and the suite fails
when a fixture on disk has no entry.
The names are spelled out, not derived, so the review sees them when a fixture changes.

## The project corpus

The projects the pipeline suites compile are shared: they live in `crates/mlkc-fixture/projects`
and `mlkc_fixture::{projects, read}` reads them.
Every suite of the pipeline asserts its own stage over every project of the corpus, so a project
added there is covered by every stage at once; each suite's snapshot holds only the stage it owns.
A suite of the pipeline drives the corpus through `mlkc-driver`, which is a dev-dependency of the
crate: the library of a pipeline crate never reaches back into the driver, but its spec suite does.
The dumps of a stage belong to the crate that owns the stage (`mlkc-lower`, `mlkc-resolve`,
`mlkc-typeck`, `mlkc-mir-build`, `mlkc-lir-wasm`), and the diagnostics to `mlkc-driver`, which
renders them for a host.

A harness lives in `tests/`, so cargo compiles it as a test target of its own as well, where
nothing calls it: it carries an `#[allow(dead_code, reason = ...)]` for that.
It is the one place a blanket allowance is honest; elsewhere an item that nothing uses is
removed or kept with an `#[expect(dead_code, reason = ...)]`.

## The fixture format

- A single-module fixture is a plain `.mlk` file.
- A project fixture is one file in the `mlkc-fixture` format: every module is headed by its place,
  `//- /main.mlk`, and what follows is its source.
  A directory fixture holds one file per module instead; the path under the directory is the
  module's place.
- Fixtures are small and self-contained; a fixture that outgrows one file becomes a directory.

## Adding a fixture

1. A fixture of a suite that keeps its own corpus (the parser, the lowering, the back end) is
   written under the right `specs/valid` or `specs/invalid` directory
   (`invalid` is for a mistake the parser or the lowering is expected to recover from).
2. A project of the shared pipeline corpus is written in `crates/mlkc-fixture/projects`:
   a file in the `mlkc-fixture` format, or a directory that holds one file per module.
3. Add a line to the suite's `*_specs.rs` with a one-line comment that says what the fixture
   shows; match the style of the other entries.
   A project of the shared corpus needs the line in **every** suite of the pipeline, and each
   suite says so by failing until it has one.
4. Run the suite; accept the new snapshot with `INSTA_UPDATE=always cargo test -p CRATE`.
5. Review the snapshot as part of the change: it is the test.

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
