---
name: pipeline-pass
description: "Use when adding or changing a compiler pass in mlkc: the pass contract, where the driver defines inputs and keys, how the pass list and stats work, and what tests a pass needs (ADRs 0008 and 0009)."
---

# Adding or changing a compiler pass

Read `docs/adr/0008-compiler-driver.md` and `docs/adr/0009-pass-contract.md` first.
`docs/architecture/README.md` maps the stages to crates and lists the driver's passes.

## The contract in one paragraph

A pass is a free function `f(input) -> (output, diagnostics)` in the crate that owns the data it
produces.
It is total (invalid input yields diagnostics, never a panic), deterministic (no `HashMap`
iteration order, no addresses, no interning order in an output), and blind to everything but its
input: no file system, no clock, no globals, no threads.
It does not render diagnostics; it returns families of data with spans.
A pass handed input its contract forbids says so with `ice!` from `mlkc-diagnostics`.

## What goes into the input

| Data                                                 | How it enters                         | Why                                                                       |
| ---------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------- |
| the unit's identity (`ModuleId`, `BodyEntityLoc`, …) | a `Copy` argument                     | it selects the unit                                                       |
| the unit's own data (CST, item tree, body)           | a `&` argument                        | keyed by the unit's `FileVersion`                                         |
| other units' data (interfaces, graph)                | fields of one owning struct of `Arc`s | nothing cheap identifies it across revisions, so the value is its own key |

The third kind is retained by the driver and compared when it asks whether the slot is current.
See `CheckDeps` in `crates/mlkc-hir-ty` and its use in
`crates/mlkc-driver/src/driver/check.rs` for a worked example.

## Steps to add a pass

1. Write the function in the owning crate: `mlkc-lower` for the HIR, `mlkc-resolve` for
   resolution, `mlkc-mir-build` for MIR, `mlkc-lir-wasm` for the target IR.
2. Add the input struct if the pass reads other units; keep it next to the types it names, not in
   the driver.
3. Add the driver-side input function, slot, and key comparison in
   `crates/mlkc-driver/src/driver/`; `lower.rs`, `resolve.rs`, `check.rs`, and `mir_module.rs`
   are the models to copy.
4. Register the pass: add a variant to `Pass` in `crates/mlkc-driver/src/driver/stats.rs`.
   The test `every_pass_is_counted` there fails when a variant is missing from `ALL` or has no
   name. Add a `Unit` if the pass runs per body, module, file, or project. The pass table of
   `docs/architecture/README.md` is held to `Pass` by a test of `xtask/glue`, so add its row in
   the same change.
5. Never let a pipeline crate depend on `mlkc-driver`; the direction is driver → pipeline.
6. Test the pass directly: a snapshot test with a hand-built input, no driver and no file system
   (`docs/adr/0009-pass-contract.md`; the `snapshot-tests` skill). Snapshot invalid inputs too.

## Checklist

- [ ] the function is total, deterministic, and free of I/O;
- [ ] diagnostics are data with spans, one family for this pass;
- [ ] the input function covers everything the pass reads
      (a missing dependency shows up as an unresolved name, never as an unkeyed read);
- [ ] the pass is in `Pass::ALL` and has a name;
- [ ] `just test-crate CRATE` passes, and `just verify` passes before finishing;
- [ ] `docs/architecture/README.md` is updated when a stage or a pass is added or renamed;
      the map tests of `xtask/glue` fail until the table and the workspace agree again.
