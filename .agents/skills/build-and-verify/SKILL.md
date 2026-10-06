---
name: build-and-verify
description: "Use when choosing or running checks and tests in the MLK repository: the just recipes for iterating, the full verify gate, snapshot updates, and what to do when a tool such as obscura is missing."
---

# Build and verify

The `justfile` is the source of truth for commands (`just --list` shows every recipe).
`nix develop` provides every tool the recipes need; without it, `just install-tools` does.
This skill says which recipe a change needs, and how to read the result.

## The loop

1. `just doctor` once per session: it reports missing required tools and the optional checks that
   cannot run.
2. Iterate with the narrowest check:
    - `just test-crate mlkc-lower` — one crate of the compiler;
    - `just test-web` — the pure modules of the editor;
    - `just check-web` — the editor's lint, types, and templates;
    - `just test-crate mlkc-codegen-wasm` — the WASM back end.
3. Before finishing, run `just verify`.

## The gates

| Command           | Runs                                                                          | Notes                                                                                      |
| ----------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `just verify`     | `check-generated`, `lint`, `doc`, `test`, `test-doc`, `test-web`, `check-web` | Works on a dirty tree; writes only temporary files. The agent gate.                        |
| `just verify-web` | `verify` + `check-browser`                                                    | Needs `obscura`; run it when the editor, `mlkc-wasm`, or the wasm boundary changed.        |
| `just ready`      | clean-tree check, `gen-all`, `verify`, `check-browser`, clean-tree check      | The human's final gate. It fails on a dirty tree by design; do not commit to make it pass. |

If `obscura` is missing, say so instead of skipping silently.
Never claim a check passed unless it ran.

The dependencies have gates of their own, which CI runs rather than `verify`:
`just doc` builds the workspace documentation with rustdoc's warnings denied, and
`just deny` reads every crate of the lockfile for its license, its advisories, and its sources.
`cargo-deny` fetches the RustSec database on the first run, which is why it is not part of a
local `verify`; run it when a dependency or the lockfile changed.

## Reports

The slow checks report rather than gate ([ADR-0027](../../../docs/adr/0027-scheduled-reports.md)):
none of them is part of `just verify`, and CI runs them on a weekly schedule and by hand.
Run one when the change makes it interesting, and say what it reported.

| Command                        | Reports                                                                                            |
| ------------------------------ | -------------------------------------------------------------------------------------------------- |
| `just coverage`                | What the suite reaches (`cargo-llvm-cov`), as HTML and LCOV under `target/coverage`.               |
| `just mutants [crate]`         | Which mutants of one crate the tests do not notice (`cargo-mutants`); a test run per mutant.       |
| `just fuzz [target] [seconds]` | Arbitrary bytes through `parse` (losslessness) or `lower` (no panic); needs the nightly toolchain. |

`just lint` compiles the fuzz targets, so `just verify` does catch the API drift between
reports; what it never does is run a fuzzer or measure the suite.

## Snapshots

- `INSTA_UPDATE=always cargo test -p CRATE` rewrites the snapshots of a crate.
- `just test-review` runs the suite and offers to accept them interactively.
- Never hand-edit `.snap` files; never leave `.snap.new` behind.
- The fixture formats and the harnesses are in the `snapshot-tests` skill.

## Generated files

- `just check-generated` verifies the Rust syntax/AST generation and the editor's Lezer grammar
  without writing anything; it works on a dirty tree and is part of `just verify`.
- When it fails, run `just gen-all` and commit the regenerated files;
  never edit generated files by hand (`AGENTS.md` lists the paths).

## What a change needs

| Change                           | Focused check                                        |
| -------------------------------- | ---------------------------------------------------- |
| A pass's logic                   | `just test-crate CRATE` and its snapshot tests       |
| A diagnostic or a dump           | that suite's snapshot tests, then `just test-review` |
| Grammar or generated code        | `just check-generated`, then `just gen-all`          |
| A crate doc or an intra-doc link | `just doc`                                           |
| A dependency or the lockfile     | `just deny`                                          |
| Editor modules under `src/lib`   | `just test-web`                                      |
| Editor types, templates, script  | `just check-web`                                     |
| Editor behavior, wasm boundary   | `just verify-web`                                    |
| The shape of the suite           | `just coverage`, `just mutants CRATE` (reports)      |
| Anything, before reporting       | `just verify`                                        |

## When a tool is missing

- no `obscura`: `check-browser` and `verify-web` cannot run; say so, and note that CI runs them;
- no Node/pnpm: `check-web` and the browser check cannot run; the Rust checks still work;
- no `cargo-insta`: `just test` still runs; `just test-review` cannot;
- no `cargo-nextest`: `just test` and `just test-crate` cannot run; `cargo test` still does;
- no `cargo-deny`: `just deny` cannot run; say so, and note that CI runs it.
- no `cargo-llvm-cov`, `cargo-mutants`, or `cargo-fuzz`: the reports cannot run;
  say so, and note that CI runs them on a schedule.
