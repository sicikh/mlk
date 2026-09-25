# Define the compilation pipeline and its stage terminology

- Status: proposed
- Date: 2026-09-20

## Context and Problem Statement

The compiler's stages span many crates
and are referenced by many ADRs.
Without a canonical vocabulary,
words like "lowering", "resolution", and "IR" will drift in meaning
between the docs and the code.

The front-end of the pipeline is settled;
the back-end (from MIR onward) is not yet designed.

This ADR fixes the names and the order of the stages
and marks which of them run per module, in parallel.
It intentionally leaves the back-end questions open,
and it stays `proposed` until the back-end stabilizes.

## Decision Drivers

- One vocabulary shared by ADRs, code, and diagnostics.
- Stage boundaries must match the incrementality cuts
  established in [0004-module-system.md].
- The record must remain truthful
  about which parts of the pipeline are still open.

## Considered Options

- **Ad-hoc terminology** — every crate names its own stages;
  meaning is negotiated on the fly.
- **Canonical pipeline** — this ADR fixes the stage names,
  their order, and their parallelism;
  open points are marked as such.

## Decision Outcome

Chosen option: "Canonical pipeline",
because the pipeline is referenced by almost every other ADR
and by the diagnostic categories in the code.

### The stages

**Source text** —
the bytes of a file.

**Lexing** —
source text into tokens and trivia.

**Parsing** —
tokens into a lossless CST,
with bogus-node recovery (see [ADR-0002][0002-lossless-syntax-tree.md]).
The parser is event-based:
it emits events,
and a tree sink assembles the green tree.

**AST** —
not a separate tree,
but the typed view over the CST:
the generated node wrappers in `mlkc-syntax`.
The word "AST" always means this view.

**Local name resolution** —
per module, in parallel:
resolves every use of an entity
to an entity of this module
or to an entry of its import table
(see [ADR-0004][0004-module-system.md]).
Produces the HIR.

**HIR** —
the high-level IR:
the locally name-resolved tree,
ID-based, allocated in per-owner arenas
(see [ADR-0003][0003-ir-principles.md]).
`ItemTree` per module, `Body` per function,
in `mlkc-hir-def`.
The step from CST/AST to HIR is called **lowering**.

**Global name resolution** —
sequential, once per project:
joins the import and export tables of all modules
into the global def map.

**Type checking** —
per module, in parallel:
type-checks the module and its functions.

**MIR** —
the mid-level IR:
a per-function control-flow graph,
typed and layout-resolved.

**AIR** —
open question: an A-normal form tree
between the typed HIR and MIR.
Undecided whether lowering goes through
an explicit ANF stage,
or whether MIR is built directly from the typed HIR.

**Thin-LTO** —
sequential whole-program pass:
inlining decisions, purity,
and other call-graph-dependent attributes.
Provisional.

**Codegen** —
per function, in parallel:
MIR into WASM.
Diagnostic category: Codegen.
Provisional.

**Linking** —
sequential:
relocations and addresses are resolved,
the output is produced.
Provisional.

### Parallelism in one pass

- Lexing and parsing: per file, in parallel.
- Local name resolution and lowering: per module, in parallel.
- Global name resolution: sequential, whole project.
- Type checking and MIR construction: per module, in parallel.
- Thin-LTO: sequential, whole program.
- Codegen: per function, in parallel.
- Linking: sequential.

### Positive Consequences

- Every ADR and every diagnostic category
  refers to the same stages.
- The parallel/sequential annotation makes visible
  where the compile-time budget goes
  and where the serial bottlenecks are.

### Negative Consequences

- The back-end part is speculative
  and will likely need revision.
  The `proposed` status carries that warning.

## Links

- Against Query Based Compilers: <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>
- Incrementality: [0004-module-system.md]
- IR design: [0003-ir-principles.md]
- Syntax tree: [0002-lossless-syntax-tree.md]
- Parser: [mlkc-parser]
- HIR: [mlkc-hir-def]

[0004-module-system.md]: 0004-module-system.md
[0003-ir-principles.md]: 0003-id-based-ir.md
[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[mlkc-parser]: ../../crates/mlkc-parser
[mlkc-hir-def]: ../../crates/mlkc-hir-def
