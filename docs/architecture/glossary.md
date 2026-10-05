# Glossary

The words the ADRs and the code use with a fixed meaning.
When a term has an owning ADR, the ADR is the source of truth;
this page is only the short form.

## The pipeline and its stages

**Source text** — the bytes of a file.

**Lexing** — source text into tokens and trivia.

**CST** — the lossless concrete syntax tree: every byte of the source, trivia included
([ADR-0002](../adr/0002-lossless-syntax-tree.md)).

**Trivia** — whitespace and comments, attached to tokens in the CST.

**Bogus node** — a node the parser inserts while recovering from an error.
Because bogus nodes exist, the parser is total: it never fails, it reports
([ADR-0002](../adr/0002-lossless-syntax-tree.md)).

**AST** — not a separate tree but the typed view over the CST:
the generated node wrappers in `mlkc-syntax`
([ADR-0005](../adr/0005-compiler-pipeline.md)).

**Lowering** — the step from the CST/AST to the HIR.
The only stage that knows syntactic sugar
([ADR-0014](../adr/0014-syntactic-sugar.md)).

**HIR** — the high-level IR:
the locally name-resolved tree of one module, ID-based,
in per-owner arenas ([ADR-0003](../adr/0003-id-based-ir.md)).

**ItemTree** — the surface of one module in the HIR:
its items and their signatures.

**Body** — the HIR of one function, as a list of expressions and statements, owned per function.

**Local name resolution** — resolving every use inside one module to an entity
of that module or to an entry of its import table;
it produces the HIR.

**Global name resolution / resolution** — once per project:
joining the import and export tables of all modules into the def map, against their interfaces
([ADR-0004](../adr/0004-module-system.md), [ADR-0016](../adr/0016-inter-module-resolution.md)).

**Interface** — what one module shows to the modules that import it;
resolution reads interfaces and never the bodies of other modules
([ADR-0016](../adr/0016-inter-module-resolution.md)).

**ModuleIndex** — the value that says which path of a project names which module.

**ModuleScope / ProjectDefMap** — the resolution result per module,
and the project-wide fold of the scopes ([ADR-0004](../adr/0004-module-system.md)).

**Type checking / check** — checking one body at a time against declared signatures;
the checker is explicitly temporary ([ADR-0017](../adr/0017-resolved-types.md)).

**ModuleTypes** — the types a module writes on its surface, resolved; a cross-module value.

**CheckedBody** — what a check leaves behind for one body: the type of every node it checked.

**MIR** — the middle IR: a uniform control-flow graph over words;
one representation serves as both CFG and SSA (block parameters, no phis)
([ADR-0018](../adr/0018-values-as-words.md), [ADR-0019](../adr/0019-mir.md)).

**Word** — an MIR operand: an immediate or a reference to a heap object;
every value is one word ([ADR-0018](../adr/0018-values-as-words.md)).

**SSA** — the static single-assignment form of a MIR body, produced by `mlkc-mir-build`;
block parameters are the joins ([ADR-0019](../adr/0019-mir.md)).

**LIR** — the WASM LIR: the target's instructions in SSA form,
between MIR and `wasm-encoder` ([ADR-0022](../adr/0022-wasm-lir.md)).

**Codegen** — MIR into WASM, per function; the module assembly above it
([ADR-0020](../adr/0020-wasm-backend.md)).

**Translation unit** — one MLK module compiles to one WASM module,
linked through core imports and exports ([ADR-0021](../adr/0021-translation-units.md)).

**LinkPlan / Manifest** — the plan that orders the compiled modules,
and the JSON a host reads to instantiate and run them ([ADR-0021](../adr/0021-translation-units.md)).

## The driver and its passes

**Driver** — the component that owns the inputs, memoizes the passes,
and decides what has to be recomputed
([ADR-0008](../adr/0008-compiler-driver.md)).
`mlkc-driver`.

**Pass** — a pure, total, deterministic function of an assembled input:
no I/O, no clock, no globals, no threads
([ADR-0009](../adr/0009-pass-contract.md)).

**Input function** — the driver-side function that assembles everything a pass reads,
so the pass cannot reach anything else
([ADR-0009](../adr/0009-pass-contract.md)).

**Slot / key** — a memoized value and the identity that decides its validity:
a `FileVersion` for a unit's own data,
the retained values for what the pass read elsewhere
([ADR-0008](../adr/0008-compiler-driver.md)).

**FileVersion** — the integer identity of the current text of a file in the VFS
([ADR-0007](../adr/0007-vfs-file-state.md)).

**Snapshot (VFS)** — a frozen view of the file states, inside which spans are resolved
([ADR-0007](../adr/0007-vfs-file-state.md)).

**Span** — a file identity plus a byte range.
It does not carry a version;
validity is the driver's business
([ADR-0007](../adr/0007-vfs-file-state.md)).

**Host** — what owns the file system, the process, or the browser;
the CLI and the wasm host are the two that exist.
A host never appears inside a pass
([ADR-0008](../adr/0008-compiler-driver.md)).

**ICE (internal compiler exception)** — a panic with a payload that says what was assumed,
where and with what backtrace;
it means a pass was handed input its contract forbids
([ADR-0009](../adr/0009-pass-contract.md)).

## Names, identity, and versions

**Idx** — a newtyped index into a per-owner arena (`mlkc-la-arena`).

**ItemLoc / EntityLoc / BodyLoc** — stable structural identities:
kind, name, and disambiguator, so an entity keeps its identity across edits
([ADR-0010](../adr/0010-stable-entity-identity.md)).

**Name** — an interned string;
`Symbol` compares and hashes by address, which is why interning order must never reach an output.

**Prelude** — the fixed list of implicit imports injected by lowering;
`#[no-prelude]` refuses it ([ADR-0011](../adr/0011-module-prelude.md)).

**DebugInfo** — the debug format of a module:
`None`, `SourceMap`, `DwarfLines`, or `DwarfFull`;
exactly one per module, chosen by the host
([ADR-0025](../adr/0025-debug-information-formats.md)).

## Language surface

**Spelling** — what the source writes;
**construct** — what it means.
Sugar stops at lowering
([ADR-0014](../adr/0014-syntactic-sugar.md)).

**`@`, `.`, `::`** — the split of the old dot:
`receiver@field` is a field read, `receiver.function(args)`
is a call,
`path::segment` names an entity
([ADR-0012](../adr/0012-split-the-dot-operator.md)).

**Pipeline operator `|>`** — `x |> f(a, _)` means `let v = x in f(a, v)`
([ADR-0013](../adr/0013-pipeline-operator.md)).

**Closure** — a lifted function plus one word pointing at its environment;
called through a typed `call_ref`
([ADR-0026](../adr/0026-closure-representation.md)).
