# Architecture Decision Records

An ADR records one architecture-significant decision and the alternatives it rejected.
The process is defined in [ADR-0001](0001-adr-process.md);
a new record starts from [the template](0000-template.md).
Accepted records are immutable:
a decision that changes is superseded by a new record, never edited in place.

This file is the index and the entry point.
The map of the code is [`docs/architecture`](../architecture/README.md);
this index says _why_ the code has the shape it has.

## Reading order

New to the project:
[0001](0001-adr-process.md) (the process) →
[0005](0005-compiler-pipeline.md) (the pipeline and its vocabulary) →
[0008](0008-compiler-driver.md) (the driver) →
[0009](0009-pass-contract.md) (what a pass is) →
[0006](0006-snapshot-testing.md) (how tests are written).
Then read the cluster of the area you are about to touch.

## Index

### Process and overview

| #                                 | Decision                                                  | Status   |
| --------------------------------- | --------------------------------------------------------- | -------- |
| [0000](0000-template.md)          | MADR template                                             | template |
| [0001](0001-adr-process.md)       | Use ADR to record architecture decisions                  | accepted |
| [0005](0005-compiler-pipeline.md) | Define the compilation pipeline and its stage terminology | proposed |
| [0006](0006-snapshot-testing.md)  | Use snapshot testing as the primary test strategy         | accepted |
| [0008](0008-compiler-driver.md)   | Drive the compiler from a host-agnostic pull-based cache  | accepted |
| [0009](0009-pass-contract.md)     | Define the contract of a compiler pass                    | accepted |
| [0027](0027-scheduled-reports.md) | Run coverage, mutation, and fuzzing as scheduled reports  | accepted |

### Syntax and surface language

| #                                      | Decision                                    | Status   |
| -------------------------------------- | ------------------------------------------- | -------- |
| [0002](0002-lossless-syntax-tree.md)   | Use a lossless syntax tree                  | accepted |
| [0012](0012-split-the-dot-operator.md) | Split `.` into `@`, `.`, and `::`           | accepted |
| [0013](0013-pipeline-operator.md)      | Pipeline operator `\|>`                     | accepted |
| [0014](0014-syntactic-sugar.md)        | Lowering is the only stage that knows sugar | accepted |

### Modules, identity, and incrementality

| #                                       | Decision                                          | Status                                               |
| --------------------------------------- | ------------------------------------------------- | ---------------------------------------------------- |
| [0003](0003-id-based-ir.md)             | ID-based IR in per-owner arenas                   | superseded by [0010](0010-stable-entity-identity.md) |
| [0004](0004-module-system.md)           | Module-based granular incrementality              | accepted                                             |
| [0007](0007-vfs-file-state.md)          | The VFS owns file state and versions              | accepted                                             |
| [0010](0010-stable-entity-identity.md)  | Stable entity identity                            | accepted; supersedes [0003](0003-id-based-ir.md)     |
| [0011](0011-module-prelude.md)          | Module prelude                                    | accepted                                             |
| [0015](0015-standard-library.md)        | Standard library in-repo, built into the compiler | accepted                                             |
| [0016](0016-inter-module-resolution.md) | Inter-module resolution against interfaces        | accepted                                             |

### Types

| #                              | Decision                               | Status   |
| ------------------------------ | -------------------------------------- | -------- |
| [0017](0017-resolved-types.md) | Resolved types and a temporary checker | accepted |

### Backend

| #                                      | Decision                                       | Status   |
| -------------------------------------- | ---------------------------------------------- | -------- |
| [0018](0018-values-as-words.md)        | Values as one tagged word                      | accepted |
| [0019](0019-mir.md)                    | MIR: a uniform SSA CFG over words              | accepted |
| [0020](0020-wasm-backend.md)           | WASM backend in-tree                           | accepted |
| [0021](0021-translation-units.md)      | Translation units: one module, one WASM module | accepted |
| [0022](0022-wasm-lir.md)               | WASM LIR                                       | accepted |
| [0026](0026-closure-representation.md) | Closure representation                         | accepted |

### Debug information

| #                                         | Decision                           | Status                                                                                               |
| ----------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------- |
| [0023](0023-debug-information.md)         | Debug information, host-configured | superseded by [0025](0025-debug-information-formats.md)                                              |
| [0024](0024-browser-debug-information.md) | Browser source map                 | superseded by [0025](0025-debug-information-formats.md)                                              |
| [0025](0025-debug-information-formats.md) | One debug format per module        | accepted; supersedes [0023](0023-debug-information.md) and [0024](0024-browser-debug-information.md) |

## Notes on drift

The records below contain statements the code has outgrown.
The code wins; the notes are here so that a plan is not written against a stale sentence.

- [0005](0005-compiler-pipeline.md) is `proposed`.
  The front end it fixes is settled and the back end has largely landed;
  still open in it: the AIR question (an explicit ANF stage or not) and Thin-LTO.
- [0019](0019-mir.md) names `crates/mlkc-mir-opt`, which does not exist yet.
  The `mir-opt` stage is planned; `simplify-cfg` and the other `Body -> Body` passes have no crate.
- [0015](0015-standard-library.md), [0016](0016-inter-module-resolution.md), and
  [0017](0017-resolved-types.md) were written before the back end existed.
  Their "later" statements about MIR, codegen, layouts, and naming are partly outdated;
  treat them as history and check the code.
- [0011](0011-module-prelude.md) uses the pre-[0012](0012-split-the-dot-operator.md) path syntax
  (`use std.prelude.Int`); paths are written with `::` now.
  Its statement that the prelude is not read from a manifest is still true.

## Adding a record

1. Copy `0000-template.md` to `NNNN-short-title.md`, using the next number.
2. Fill it in MADR style, in English, with semantic linebreaks: one sentence or clause per line.
3. Link the code it touches and the ADRs it relates to.
4. Set the status: `proposed` until reviewed, then `accepted` or `rejected`.
5. Add it to this index, in the cluster it belongs to.
