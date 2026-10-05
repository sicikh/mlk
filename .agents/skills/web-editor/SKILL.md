---
name: web-editor
description: "Use when changing the editor under web/ or the wasm host: the SvelteKit and CodeMirror layout, the driver worker, the wasm package build, and how to extend browser-check.mjs (ADRs 0024 and 0025)."
---

# The editor

The editor is a SvelteKit app that loads `mlkc-wasm` in a Web Worker and shows every stage of the
pipeline for the buffer.

## Layout

| Path                                                         | What it holds                                                                     |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `web/src/routes/+page.svelte`                                | the page: files, editor, panels, and most `data-*` hooks                          |
| `web/src/lib/components/*.svelte`                            | the panels: CST/AST/HIR/types/MIR/LIR/WAT, config, console, files, stats          |
| `web/src/lib/driver.ts`                                      | the typed wrapper over the wasm API                                               |
| `web/src/lib/driver.worker.ts`                               | the worker that owns the wasm instance (`useStd`, `setText`, `cst`, …, `stats`)   |
| `web/src/lib/{archive,wat,diagnostics,highlight,offsets}.ts` | the pure helpers behind the panels                                                |
| `web/src/lib/grammar/mlk.{grammar,ts}`                       | the Lezer grammar for highlighting (generated `mlk.ts`)                           |
| `packages/wasm/`                                             | the `wasm-bindgen` package built from `mlkc-wasm`; `lib/` is built, not committed |
| `web/scripts/browser-check.mjs`                              | the end-to-end check of the built site                                            |

## Commands

- `just dev-web` — the dev server; it builds the wasm package first.
- `just test-web` — the tests of the pure modules (Vitest); no browser, no wasm.
- `just check-web` — `oxlint` over the editor sources and `svelte-check` (types and templates).
- `just check-browser` — builds the site and drives it in a browser; needs `obscura`.
- `just verify-web` — the full gate: `verify` plus the browser check.
- `just build-web` — the static site.
  `packages/wasm` builds cargo plus `wasm-bindgen`; `wasm-bindgen-cli` must match the
  `wasm-bindgen` crate, and `just install-tools` derives the version from `Cargo.lock`.

## The browser check

The pure modules — `offsets.ts`, `archive.ts` (which writes and reads the ZIP of a build),
`wat.ts`, `diagnostics.ts`, `highlight.ts` — are values in and values out, and are tested as
them by `just test-web`; a fixture under `src/lib/fixtures/` is what a zipper other than the
editor's own wrote.

`web/scripts/browser-check.mjs` serves the built site (`vite preview` on a free port), starts
`obscura serve` when no browser answers at `--cdp`, and drives the page over CDP.
It is the only check that reaches the wasm boundary and what a person would see.
It is the part of the editor no other test can replace.

To extend it:

1. Give the DOM a stable `data-*` attribute — the check selects by data attributes, not classes.
2. Add the JavaScript step to `STEPS` and, when the page has to wait, a predicate to `WAITS`.
3. Add a row to `RUN` — `[capture, step, wait, settings?]` — so the step runs in order and its
   answer is kept under the capture name.
4. Add a check to `CHECKS` — `[what it asks, what the run read, what it expected]`. Both sides
   may be values or functions of the captures, and an expectation may be a matcher
   (`greater`, `includes`, `matches`, `allOf`, ...) where equality is too plain.
5. Run `just check-browser`; iterate with `--only TEXT` (the run is the same, the report is
   filtered) and `--list` to see the checks.

A new capture needs its shape written down in the `Page` types of the script: `just check-web`
type-checks the script (JSDoc with `checkJs`) and lints it (oxlint), and a check that reads a
capture the run no longer makes fails rather than crashing the report.

Constraints, documented at the top of the script:

- the check drives the built site; a dev server cannot be driven, because obscura runs the worker
  as a classic script whatever its type says;
- keystrokes are not sent: the buffer text goes in through the DOM with a synthetic `input`
  event, and key chords are not serialized;
- no fixed sleeps: a step waits for the thing it reads through a `WAITS` predicate;
- a failing check or step does not stop the run: every check is read, each failure prints what
  was expected against what the run saw, and the end is a tally;
- parts of the check are obscura-specific workarounds; do not weaken an assertion without
  understanding why it asserts it.

The check is long (about a hundred assertions) and CI runs it in the `web` job.
When touching the editor or the wasm boundary, run `just verify-web` — and say so if `obscura` is
missing instead of skipping silently.
