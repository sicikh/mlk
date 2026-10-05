---
name: build-and-verify
description: "Use when choosing or running checks and tests in the MLK repository: the just recipes for iterating, the full verify gate, snapshot updates, and what to do when a tool such as obscura is missing."
---

# Build and verify

The `justfile` is the source of truth for commands (`just --list` shows every recipe).
This skill says which recipe a change needs, and how to read the result.

## The loop

1. `just doctor` once per session: it reports missing required tools and the optional checks that
   cannot run.
2. Iterate with the narrowest check:
    - `just test-crate mlkc-lower` — one crate of the compiler;
    - `just check-web` — the editor's types and templates;
    - `just test-crate mlkc-codegen-wasm` — the WASM back end.
3. Before finishing, run `just verify`.

## The gates

| Command           | Runs                                                                     | Notes                                                                                      |
| ----------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| `just verify`     | `check-generated`, `lint`, `test`, `test-doc`, `check-web`               | Works on a dirty tree; writes only temporary files. The agent gate.                        |
| `just verify-web` | `verify` + `check-browser`                                               | Needs `obscura`; run it when the editor, `mlkc-wasm`, or the wasm boundary changed.        |
| `just ready`      | clean-tree check, `gen-all`, `verify`, `check-browser`, clean-tree check | The human's final gate. It fails on a dirty tree by design; do not commit to make it pass. |

If `obscura` is missing, say so instead of skipping silently.
Never claim a check passed unless it ran.

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

| Change                         | Focused check                                        |
| ------------------------------ | ---------------------------------------------------- |
| A pass's logic                 | `just test-crate CRATE` and its snapshot tests       |
| A diagnostic or a dump         | that suite's snapshot tests, then `just test-review` |
| Grammar or generated code      | `just check-generated`, then `just gen-all`          |
| Editor types or templates      | `just check-web`                                     |
| Editor behavior, wasm boundary | `just verify-web`                                    |
| Anything, before reporting     | `just verify`                                        |

## When a tool is missing

- no `obscura`: `check-browser` and `verify-web` cannot run; say so, and note that CI runs them;
- no Node/pnpm: `check-web` and the browser check cannot run; the Rust checks still work;
- no `cargo-insta`: `just test` still runs; `just test-review` cannot.
