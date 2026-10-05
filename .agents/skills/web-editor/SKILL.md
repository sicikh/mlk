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
- `just check-web` — `svelte-check` (types and templates).
- `just check-browser` — builds the site and drives it in a browser; needs `obscura`.
- `just verify-web` — the full gate: `verify` plus the browser check.
- `just build-web` — the static site.
  `packages/wasm` builds cargo plus `wasm-bindgen`; `wasm-bindgen-cli` must match the
  `wasm-bindgen` crate, and `just install-tools` derives the version from `Cargo.lock`.

## The browser check

`web/scripts/browser-check.mjs` serves the built site (`vite preview` on a free port), starts
`obscura serve` when no browser answers at `--cdp`, and drives the page over CDP.
It is the only check that reaches the wasm boundary and what a person would see.
It is the part of the editor no other test can replace.

To extend it:

1. Give the DOM a stable `data-*` attribute — the check selects by data attributes, not classes.
2. Add the JavaScript step to `STEPS` and, when the page has to wait, a predicate to `WAITS`.
3. Run the step from `main()` and capture what the later steps need.
4. Add a `[label, predicate]` entry to `checks` in `report()`, so a failure names the check.
5. Run `just check-browser`.

Constraints, documented at the top of the script:

- the check drives the built site; a dev server cannot be driven, because obscura runs the worker
  as a classic script whatever its type says;
- keystrokes are not sent: the buffer text goes in through the DOM with a synthetic `input`
  event, and key chords are not serialized;
- prefer polling with `until` over adding `sleep`;
- parts of the check are obscura-specific workarounds; do not weaken an assertion without
  understanding why it asserts it.

The check is long (about a hundred assertions) and CI runs it in the `web` job.
When touching the editor or the wasm boundary, run `just verify-web` — and say so if `obscura` is
missing instead of skipping silently.
