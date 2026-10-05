# AGENTS.md

How to work in this repository when you are a coding agent.
This file is the entry point: it points at the deep context instead of repeating it.

## What this repository is

MLK is a statically typed language.
`mlkc` is its compiler: a pipeline of pure passes driven by a memoizing driver,
with hosts for the CLI and the browser (WebAssembly).
The editor under `web/` runs the same driver compiled to wasm.
The pipeline is mostly written; expect it to grow sideways —
new passes, new language features, an LSP host —
more often than to be rearchitected.

## Read first

1. `README.md` — what is here, how to build and run it.
2. `docs/architecture/README.md` — the map: pipeline, crates, where to change what.
3. `docs/architecture/glossary.md` — the vocabulary the ADRs and the code use.
4. `docs/adr/README.md` — the decision index and its drift notes;
   read the ADRs of the area you are about to touch.

## Task playbooks

Skills under `.agents/skills/` are loaded on demand.
Read the one that matches the task before starting:

| Skill                | When                                                                   |
| -------------------- | ---------------------------------------------------------------------- |
| `build-and-verify`   | Choosing checks and tests, updating snapshots, a tool that is missing. |
| `pipeline-pass`      | Adding or changing a compiler pass.                                    |
| `syntax-and-sugar`   | Changing the grammar, the parser, generated nodes, or lowering.        |
| `parser-development` | Parse rules, recovery, lists, the lexer, inline specs.                 |
| `snapshot-tests`     | Adding or updating fixtures and snapshots.                             |
| `backend-wasm`       | MIR, LIR, codegen, linking, debug information.                         |
| `web-editor`         | The editor, the wasm host, `browser-check.mjs`.                        |
| `doc-comments`       | Writing or editing Rust comments and crate docs.                       |
| `delegating-work`    | A task large enough to split across subagents.                         |

## Commands

| Command                               | What it is for                                                                             |
| ------------------------------------- | ------------------------------------------------------------------------------------------ |
| `just doctor`                         | Check the environment; run it before blaming the build.                                    |
| `just lint`                           | `cargo clippy --workspace --all-features --all-targets -- --deny warnings`.                |
| `just test`                           | The workspace test suite.                                                                  |
| `just test-crate mlkc-lower`          | One crate; the fast loop while iterating.                                                  |
| `just test-doc`                       | Doc tests.                                                                                 |
| `just test-review`                    | Run the snapshot tests and review the pending changes interactively.                       |
| `just check-generated`                | Verify that generated files are up to date, without writing them. Works on a dirty tree.   |
| `just verify`                         | `check-generated` + `lint` + `test` + `test-doc` + `check-web`. Run this before finishing. |
| `just check-web`                      | Type-check the editor (svelte-check).                                                      |
| `just verify-web`                     | `verify` plus the editor in a real browser. Needs `obscura` on the path.                   |
| `just gen-all`                        | Regenerate every generated file; commit the result.                                        |
| `just format`                         | `cargo fmt` (nightly rustfmt) and `tombi format`.                                          |
| `just dev-web`                        | The editor on a dev server.                                                                |
| `cargo run -p mlkc-cli -- parse FILE` | Parse one file and print the trees.                                                        |
| `cargo run -p mlkc-cli -- run BUILD`  | Run a build: a directory or an archive.                                                    |

`just --list` shows everything.

## Rules

### Generated code

- Never edit files under `crates/mlkc-syntax/src/generated/`,
  `crates/mlkc-syntax-factory/src/generated/`, or `web/src/lib/grammar/mlk.ts` by hand.
  They are written by `xtask-codegen` and `lezer-generator`;
  run `just gen-all` and commit the result.
- The sources of truth are `xtask/codegen/mlk.ungram`,
  `xtask/codegen/src/mlk_kinds_src.rs`, and `web/src/lib/grammar/mlk.grammar`.
- `crates/mlkc-syntax/src/generated.rs` and `crates/mlkc-syntax-factory/src/generated.rs`
  are hand-written module glue despite the name.
- `just check-generated` must stay green; CI enforces the same thing.

### Snapshots

- Snapshot testing is the primary test strategy ([ADR-0006](docs/adr/0006-snapshot-testing.md)).
  A change to an output is accepted deliberately:
  `INSTA_UPDATE=always cargo test -p CRATE`, then `just test-review`.
- Do not hand-edit `.snap` files, and do not leave `.snap.new` files behind.
- Fixtures use the one-file project format of `mlkc-fixture` (`//- /path.mlk` headers).
  A new fixture needs an entry in the `*_specs.rs` of its suite;
  the suite fails on an unlisted fixture.

### Architecture decisions

- A decision with project-wide consequences gets an ADR as part of the change that makes it
  ([ADR-0001](docs/adr/0001-adr-process.md)): MADR, English, semantic linebreaks
  (one sentence or clause per line), status `proposed` until reviewed.
- Accepted ADRs are immutable; a changed decision is superseded by a new record.
- Add the record to `docs/adr/README.md` and link the code it touches.

### Passes

- A pass is a pure, total, deterministic function of an assembled input
  ([ADR-0009](docs/adr/0009-pass-contract.md)).
  No file system, no clock, no globals, no threads inside;
  invalid input yields diagnostics, never a panic
  _(except for a bug in the compiler itself, ICE)._
- The driver owns dependencies: a pass sees only what its input function assembled.
  If a pass starts reading something new, the input function and the driver's keys change with it.
- Determinism: no `HashMap` iteration order, no addresses,
  no interning order may reach an output.
- A pass does not render: diagnostics are data with spans, one family per pass.

### Verification

- While iterating: `just test-crate NAME`, `just check-web` for the editor.
- Before finishing: `just verify`.
  When the editor or the wasm boundary changed, also `just verify-web` —
  and if `obscura` is missing, say so instead of silently skipping it.
- Never claim a check passed unless you ran it.

### The working tree

- Do not commit, push, or create branches unless the user asks.
- Do not touch `target/`, `node_modules/`, `.pnpm-store/`, or `packages/wasm/lib` (built).
- `just ready` is the human's final gate: it requires a clean tree and a browser.
  Agents use `just verify`.

## Environment

- Rust 1.99.0, edition 2024, pinned in `rust-toolchain.toml`.
  Node from `.nvmrc`, pnpm through corepack, `just`.
- The Nix flake provides the whole toolchain: `nix develop` gives the pinned Rust,
  the nightly rustfmt the formatter needs, wasm-bindgen-cli
  (the version nixpkgs builds, which `Cargo.toml` pins the crate to),
  wasm-tools, wasmtime, cargo-insta, just, Node and pnpm through corepack, and obscura.
  Inside the shell `just install-tools` installs only the pnpm dependencies.
- Without Nix, `just install-tools` installs the rest: cargo-insta,
  wasm-bindgen-cli (its version is derived from `Cargo.lock` and must match the `wasm-bindgen` crate),
  wasm-tools, wasmtime, and the pnpm dependencies.
- `obscura` is needed only by `just check-browser` and `just verify-web`;
  the dev shell and CI both provide it.
- `just doctor` reports what is missing.

## Delegation

The `delegating-work` skill is the full playbook: when to split, the brief a subagent needs,
which files have one writer, and how to integrate the pieces.
The short version:

When a task is big enough to split, split it and keep the pieces independent:

- Good splits: by pipeline stage or crate,
  by artifact (Rust change / editor change / docs), or by test area.
  A split that makes two agents edit the same file is not a split.
- Give each subagent: the goal, the exact paths it may write, the relevant ADR numbers,
  the commands that will verify it, and what "done" means.
- Shared files have exactly one writer: `Cargo.toml`, `Cargo.lock`, `justfile`, `AGENTS.md`,
  `docs/architecture/*`, `docs/adr/README.md`, `.github/workflows/ci.yml`.
- After merging the pieces, run `just verify` on the combined result.
- Small tasks are cheaper with one agent; delegation costs context.

## Gotchas

- `mlkc-driver` is the only crate that knows the passes;
  pipeline crates never reach back into it.
- `Symbol` from `mlkc-intern` compares and hashes by address:
  never let interning order affect an output.
- The type checker is explicitly temporary ([ADR-0017](docs/adr/0017-resolved-types.md));
  do not design around its internals.
- `dead_code` is allowed workspace-wide, so the compiler will not flag unused items.
- The vendored crates are `mlkc-rowan`, `mlkc-text-size`, `mlkc-text-edit`,
  `mlkc-string-case`, `mlkc-ungrammar`;
  their headers say where they came from and how they were stripped.
  Prefer adapting to them over rewriting them.
- The editor's browser check drives real wasm-GC and has obscura-specific workarounds.
  Do not weaken what a check asserts without understanding why it asserts it.
