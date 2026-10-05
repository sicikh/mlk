---
name: backend-wasm
description: "Use when working on MIR, the WASM LIR, code generation, linking, or debug information: the stage boundaries and crates, the value model, and how to validate output with wasm-tools, wasmtime, and the interpreter (ADRs 0018-0026)."
---

# The WASM back end

The back end turns a checked program into WASM modules and a manifest a host runs.
Read `docs/adr/0018-values-as-words.md`, `0019-mir.md`, `0020-wasm-backend.md`,
`0021-translation-units.md`, and `0022-wasm-lir.md` before changing a stage;
debug information is `0025-debug-information-formats.md`, closures are
`0026-closure-representation.md`.

## Stages and crates

| Stage             | Where                                         | Produces                                                                                                                                                                          |
| ----------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MIR construction  | `crates/mlkc-mir-build`                       | `mlkc_mir::Body`, the CFG form                                                                                                                                                    |
| SSA               | `crates/mlkc-mir-build/src/ssa.rs`            | the SSA form of a body (block parameters, no phis)                                                                                                                                |
| Module MIR        | `crates/mlkc-driver/src/driver/mir_module.rs` | `ModuleMir` (defined in `crates/mlkc-codegen-wasm/src/module.rs`): a module's functions and the calls between them                                                                |
| LIR               | `crates/mlkc-lir-wasm`                        | target instructions in SSA (`body.rs`, `ty.rs`, `cfg.rs`, `structure.rs`, `verify.rs`)                                                                                            |
| LIR to module     | `crates/mlkc-codegen-wasm`                    | selection (`select.rs`), refinement (`refine.rs`), locals (`allocate.rs`), structuring (`structure.rs`), collapsing (`collapse.rs`), emission (`emit.rs`), assembly (`module.rs`) |
| Linking           | `crates/mlkc-driver`                          | `LinkPlan` and the `Manifest` a host runs (`driver/link.rs`, `manifest.rs`)                                                                                                       |
| Debug information | `crates/mlkc-codegen-wasm`                    | `dwarf.rs`, `sourcemap.rs`; one `DebugInfo` per module                                                                                                                            |

The interpreter `crates/mlkc-interp` is the reference semantics: what it computes is what the WASM
module of the same program has to compute.

## The model worth remembering

- Every value is a word: an immediate or a reference (`0018`).
  The backend refines what it knows: a value known to be an immediate gets an `(ref i31)` local,
  a value that is only a word gets an `eqref` one.
- A module's ABI is the shape of a signature (`FnShape`): immediate types cross as `(ref i31)`,
  everything else as an `eqref`.
- `assemble_module` must be deterministic: two compilations of one module agree byte for byte.
- One MLK module compiles to one WASM module; imports and exports are the canonical names of
  entities, and a host reads the JSON `Manifest`.
- A construct the backend cannot lower reports `CodegenDiag::Unsupported` rather than emitting
  something wrong.

## Checking a change

1. `just test-crate mlkc-mir-build`, `just test-crate mlkc-lir-wasm`, or
   `just test-crate mlkc-codegen-wasm`.
2. The codegen snapshots print the assembled module with `wasmprinter`;
   `INSTA_UPDATE=always cargo test -p mlkc-codegen-wasm` rewrites them, and the diff is the
   review.
3. The execute and run tests instantiate the module with `wasmtime` and compare against the
   interpreter (`crates/mlkc-codegen-wasm/tests/`, `crates/mlkc-interp/tests/`).
4. To look at real output: `cargo run -p mlkc-cli -- run BUILD` (a directory or an archive);
   `-g`/`--debug` runs it under a native debugger.
   `wasm-tools print` and `wasm-tools validate` work on the `.wasm` files of a build.
5. The editor's inspector shows MIR, SSA, LIR, and WAT for a buffer, and the browser check
   asserts their shape; run `just verify-web` when the wasm boundary changed.

## Gotchas

- ADR-0022 expects the LIR to re-earn the output the emitter already produced: an optimization
  lands only when the snapshots show it is correct.
- Debug formats are exclusive per module (`0025`); `DwarfFull` is a promise that is still
  partial (there are no variable DIEs yet).
- Closures are already implemented (ADR-0026) even though the record is still `proposed`;
  `docs/adr/README.md` tracks such drift.
