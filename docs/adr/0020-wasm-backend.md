# Emit WASM in-tree, over wasm-encoder and gimli

- Status: accepted
- Date: 2026-10-01

## Context and Problem Statement

MIR exists ([ADR-0019][0019-mir.md]), values are words ([ADR-0018][0018-values-as-words.md]),
and the first target is WASM with the GC proposal:
`i31ref` for immediates, `eqref` for words,
GC structs and arrays for structures and strings,
and no erasure of those types.

The original plan was to generate WASM through the `waffle` crate.
That plan does not survive contact with the facts:

- upstream Waffle 0.3.1 has no GC types at all:
  its IR `Type` is `I32`, `I64`, `F32`, `F64`, `V128`, `FuncRef`, `TypedFuncRef`,
  so it cannot name `i31ref`, `eqref`, a struct type, or an array type,
  and a `Word` cannot be expressed
  ([waffle#13](https://github.com/bytecodealliance/waffle/issues/13));
- the fork `portal-co/waffle-`, published as the `portal-pc-waffle*` crates
  (0.6.0-alpha.1), does have GC structs and arrays,
  an SSA IR, and a backend pipeline (reducify, stackify, treeify, localify);
  but it is an alpha of an external fork,
  its own documentation records feature combinations that do not compile,
  and it offers no DWARF;
- neither emits debug information,
  and DWARF is a requirement, not a nicety:
  a compiled program must be steppable in a browser and in `wasmtime`.

At the same time, a WASM code generator is the piece of a compiler
where the distance between "valid" and "useful" is made of
control-flow structuring, local allocation, and debug tables —
all of which have to be ours anyway if DWARF is ours,
and none of which is large enough to justify a dependency
that would also own our optimizations.

The question:
what generates WASM from MIR,
and how do GC types, structured control flow, and DWARF get emitted?

## Decision Drivers

- GC types must be first-class:
  structs, arrays, and recursive groups, named deterministically.
- DWARF must be emitted, with line tables good enough for a debugger to step,
  and no dependency can do it for us.
- MIR optimizations are ours ([ADR-0019][0019-mir.md]);
  the backend must not smuggle in a second optimizer with different semantics.
- The pass contract ([ADR-0009][0009-pass-contract.md]):
  codegen is a pure, total, deterministic function of its input.
- Back-end seam:
  Cranelift is next, and the driver must be able to name either backend
  without MIR changing.
- Dependencies stay small and inspectable:
  `wasm-encoder` to write, `wasmparser` to validate in tests,
  `gimli` to write and read DWARF, an engine for execution tests.
- Educational clarity:
  the pipeline must be explainable,
  and each stage must be replaceable on its own.

## Considered Options

- **Upstream Waffle** — the original plan.
- **The `portal-pc-waffle` fork** — a Waffle with GC types and a backend.
- **A second WASM backend crate of our own**, emitting through `wasm-encoder`,
  with `gimli` for DWARF.

## Decision Outcome

Chosen option: "A second WASM backend crate of our own",
because no existing crate emits both our value model and our debug information,
and the parts that would remain to borrow — structuring and local allocation —
are exactly the parts DWARF forces us to own the layout of.

The fork stays a reference and a fallback:
its algorithms are Apache-2.0 WITH LLVM-exception, the license of this project,
so a structure that proves too costly to write from scratch
may be adapted from it with attribution,
behind the same backend seam.

### The crates

- `mlkc-codegen-wasm` — MIR to WASM functions, and functions to a module.
  It depends on `mlkc-mir`, `mlkc-hir-def` (entities), `mlkc-hir-ty` (signatures),
  `mlkc-span`, `mlkc-line-index` (debug line tables), `wasm-encoder`, and `gimli`,
  and on nothing that knows a driver exists.
- `mlkc-wasm` is already taken by the browser shim of the compiler itself;
  it is not this crate and does not move.

### The stages

Codegen is two stages, because a WASM module is assembled after its functions exist,
and because DWARF needs addresses that are only known after the layout:

```rust
// mlkc-codegen-wasm
/// One compiled function: its body bytes, and where its instructions came from.
pub struct FuncArtifact {
    /// The encoded body, without the size prefix.
    pub body: Vec<u8>,
    /// Source positions of instructions inside `body`, by byte offset.
    pub origins: Vec<Origin>,
    /// The refinement of every value, for the debugger and for the assembler.
    pub debug: Vec<ValueDebug>,
}

pub fn emit_function(mir: &Body, ctx: &FunctionCtx) -> (FuncArtifact, Vec<CodegenDiag>);

/// One module's checked bodies, their MIR, the structures the module declares,
/// and the signatures its entities have ([ADR-0017][0017-resolved-types.md]).
pub fn assemble_module(
    module: &ModuleMir,
    functions: &[FuncArtifact],
    debug: DebugLevel,
) -> (WasmModule, Vec<CodegenDiag>);
```

- `emit_function` runs per body, in parallel, like every other pass
  ([ADR-0005][0005-compiler-pipeline.md]).
  Its input is the SSA body, the signature of its owner, the layouts of the structures
  the body names, and, for the debug tables, the checked types of the body's nodes;
  its output does not borrow from any of them,
  so the driver stores it ([ADR-0009][0009-pass-contract.md]).
- `assemble_module` runs per module:
  it assigns type and function indices,
  lays out the code section,
  resolves the addresses of the debug tables,
  and writes the custom sections.
  It reads the interfaces of the modules the module imports from
  ([ADR-0016][0016-inter-module-resolution.md]) to name imports and exports.

### Value refinements

A word carries no kind ([ADR-0018][0018-values-as-words.md]),
but the emitter often knows one statically:
the result of `IntAdd` is an immediate,
the result of a `struct.new` is a specific structure,
a parameter of a function whose signature says `Int` is an immediate.

At the boundary of a function, a kind needs no recovery at all:
a parameter and a result of an immediate type cross as an `(ref i31)`,
and one of a type that is only a word crosses as an `eqref` —
the WASM signature is the shape of the checked types,
a parameter is the local the signature declares,
so nothing is copied at the entry either.
A value that is only a word is cast where something more precise needs it.

The emitter runs a forward dataflow over the SSA body
and computes, per value, a refinement ordered by precision:

```
Word             -- any word; no knowledge at all
Int | Bool |
Str | Struct(s)  -- known to be an immediate, a string, or a structure
Never            -- unreachable; the refinement of a value with no definition
```

A join takes the least upper bound: `Int` joined with `Bool` is `Word`,
and `Never` joined with anything is that anything.

- a WASM local for a value known to be `Int` is typed `(ref i31)`,
  so `i31.get_s` and `ref.i31` need no cast;
- a value known to be `Struct(s)` is typed `(ref null $s)`,
  so field access emits `struct.get $s` with no further cast;
- at a join, the refinement is the join of the predecessors';
- a value that is only `Word` is typed `eqref`,
  and an operation that needs more emits `ref.cast`/`ref.test` for the check.

The refinement is local to the backend.
It is what makes "types are not erased" pay:
the fields of a structure are read by an instruction that names the structure,
not by a dynamic walk over a word.

### Structured control flow

WASM control flow is structured; MIR control flow is a graph.
The emitter turns one into the other:

1. **A single-block body is emitted directly.**
   That is every body the current language lowers to,
   so the pipeline is end-to-end before any structuring exists.
2. **A reducible multi-block body is structured.**
   The CFG is split into a tree of single-entry regions
   (the algorithm of Ramsey, as in Waffle's `stackify`):
   a region with two exits that reconverge becomes an `if`,
   a region whose back edge returns to its head becomes a `loop`,
   and a branch whose target is inside the current region
   becomes a `br` to the label the emitter gave the region.
3. **An irreducible body falls back to a dispatch loop:**
   a `loop` with a `pc` local and a `br_table` over the block ids.
   The fallback is always correct;
   it is slower, and only a body that structuring cannot express pays for it.
4. Later, `localify`: block parameters that a WASM branch can carry
   are passed on the stack,
   and the rest are assigned to WASM locals by linear scan over live ranges.
   Before that, every parameter and value gets a local of its own.

The emitter never rejects a well-formed SSA body for its shape.

### Instructions

The lowering of a primitive is one or two instructions, or a small sequence
with a refinement that avoids the casts:

| MIR              | WASM                                                                                                 |
| ---------------- | ---------------------------------------------------------------------------------------------------- |
| `Const::Int(n)`  | `ref.i31` of the constant                                                                            |
| `Const::Bool(b)` | `ref.i31` of `0`/`1`                                                                                 |
| `Const::Unit`    | `ref.i31` of `0`                                                                                     |
| `IntAdd`         | unbox both, `i32.add`, `ref.i31` (truncation is the wrapping of [ADR-0018][0018-values-as-words.md]) |
| `IntLt`          | unbox both, `i32.lt_s`, `ref.i31`                                                                    |
| `IntDiv`         | unbox both, `i32.div_s`, `ref.i31`; a zero divisor traps, as `IntDiv` says                           |
| `IntNeg`         | `i32.sub` from zero, unboxed and reboxed                                                             |
| `BoolNot`        | unbox, `i32.eqz`, rebox                                                                              |
| `RefEq`          | `ref.eq`                                                                                             |
| `Goto`/`Branch`  | `br`/`br_if`, with arguments where parameters expect them                                            |
| `Return`         | `return`                                                                                             |
| `Unreachable`    | `unreachable`                                                                                        |

A `Call` is a direct `call` to a function of the module or to an import;
a `Callee::Local` names a function declared inside a body,
emitted like any other function of the module;
`Callee::Indirect` is a `CodegenDiag::Unsupported` until closures exist.
A generic callee never reaches a backend:
the lowering reports it ([ADR-0019][0019-mir.md]).

### Module assembly

One MLK module becomes one WASM module ([ADR-0021][0021-translation-units.md]).
The assembler lays out, deterministically:

1. **Types.**
   The GC struct type of every structure the module names,
   declared exactly as every other module declares it:
   fields in layout order, with their mutability, packedness, and referenced types;
   a structure alone is a recursion group of one,
   and a cycle of mutually recursive structures is one group —
   the group is intrinsic to the structures, not to the module
   ([ADR-0021][0021-translation-units.md]).
   The array type of `String` is declared the same way.
   Groups are emitted in dependency order,
   so a group references only types declared before it.
   Function signatures come after, shaped by the checked types:
   an immediate parameter or result is an `(ref i31)`,
   every other one a word, `eqref`,
   and signatures of one shape share a type.
   Indices are assigned in the order the module declares its entities,
   so two compilations of the same module agree byte for byte.
2. **Imports** — the functions this module calls in others,
   named as [ADR-0021][0021-translation-units.md] decides.
3. **Functions** — one per `FuncArtifact`, in declaration order.
4. **Exports** — the public functions of the module.
5. **The `name` section** — module, function, and local names,
   so stack traces are readable even without DWARF.
6. **Debug custom sections**, when the debug level asks for them.

`assemble_module` returns the bytes together with the import and export tables
the linker of [ADR-0021][0021-translation-units.md] consumes,
so the linker never parses what the assembler just wrote.

### DWARF

Debug information is emitted as WASM custom sections
(`.debug_info`, `.debug_abbrev`, `.debug_line`, `.debug_str`, and later
`.debug_ranges`/`.debug_loclists`),
written with `gimli`, in one deterministic pass of `assemble_module`.

- **Addresses.**
  A WASM DWARF address is an offset inside the code section,
  the convention LLVM and `wasm-ld` use.
  Because the size of every function body is a byte count,
  the code section layout is computed after the bodies are emitted,
  and the line program and the DIEs are written with resolved addresses.
  A test pins the convention against `wasmtime`'s `addr2line`.
- **Line tables first.**
  Every `Origin` of a function says which span an instruction range came from.
  A line program per source file is built from them,
  with line and column taken from the file's `LineIndex` ([mlkc-line-index]),
  which the driver memoizes per file version ([ADR-0007][0007-vfs-file-state.md]).
  The driver hands the line indexes of every file a body may name to codegen
  as part of the input ([ADR-0009][0009-pass-contract.md]);
  after inlining, that is more than the module's own file.
- **Then DIEs.**
  A compile unit per module, a subprogram per function
  (name, linkage name, low/high pc), and a parameter/local per word
  with `DW_AT_location` expressed as `DW_OP_WASM_location`,
  the vendor opcode the toolchain uses for WASM locals.
  The mapping from a word to a source type comes from `CheckedBody` and
  `ModuleTypes`, which codegen receives with the body.
- **Levels.**
  `DebugLevel::None` emits nothing;
  `DebugLevel::Lines` emits line tables only;
  `DebugLevel::Full` adds DIEs, parameters, and locals.
  The level is configuration, which is an input ([ADR-0008][0008-compiler-driver.md]),
  and the debug tables are therefore absent-or-equal for equal input,
  which is what the driver's comparison requires.
- **Consumers.**
  Chrome DevTools and `wasmtime` read WASM DWARF, so no source-map format is invented.
  What the debugger shows for a word is the source type the checker gave it;
  the representation of [ADR-0018][0018-values-as-words.md] is visible
  in the module's GC types and in the `name` section, not in a DIE.

### Testing

- Every emitted module is validated in tests
  with `wasmparser`'s validator, GC and function references enabled.
- The text form of a module (`wasmprinter`) is snapshotted
  for small programs, as [ADR-0006][0006-snapshot-testing.md] requires.
- Generated programs are executed in `wasmtime` in end-to-end tests,
  and the expected values are compared.
- The debug tables are read back with `gimli`:
  a test asks for the line of a known instruction and for the DIE of a function,
  and a test uses `wasmtime`'s `addr2line` once, to pin the address convention.
- A differential test against a MIR interpreter is the plan once an interpreter
  exists; it is not part of this record.

### The backend seam

The driver names the stages of a backend, not a trait object:
`emit_function` and `assemble_module` are free functions in the crate
that owns their output, as [ADR-0009][0009-pass-contract.md] requires.
When Cranelift arrives,

- MIR does not change;
- the driver's stage list gains a second pair of functions
  (or a configured choice between them, since configuration is an input);
- `mlkc-codegen-cranelift` consumes the same MIR bodies
  and produces object code instead of WASM bytes.

### Positive Consequences

- The GC types are exactly what the language's value model needs,
  and the value model is visible in the emitted module.
- DWARF is emitted by us, so the debug experience is a feature we can iterate on
  instead of a property of a dependency.
- The emitter is small enough to be read, tested, and replaced in stages:
  direct single blocks, then structuring, then localify.
- Nothing between MIR and the bytes owns an optimization we did not write
  ([ADR-0019][0019-mir.md]).

### Negative Consequences

- We own the hard parts: control-flow structuring and local allocation.
  Milestones mitigate the risk; the dispatch loop guarantees correctness meanwhile.
- The first shaped programs are not pretty:
  every value has its own local until `localify` exists.
- DWARF is a large format, and the first version covers line tables and functions,
  not every location and type a mature toolchain emits.
- The code section layout is computed twice over (bodies, then debug),
  which is cheap only because a module is small.

## Pros and Cons of the Options

### Upstream Waffle

The original plan: translate MIR to Waffle's SSA IR, let Waffle write the module.

- Good, because its IR is an SSA CFG with block parameters,
  close to what MIR already is.
- Good, because a Waffle module assembles, optimizes, and emits in one crate.
- Bad, because it cannot express a `Word`:
  no abstract heap types, no struct or array types, no `i31ref`, no `eqref`
  (bytecodealliance/waffle#13).
- Bad, because its debug story is source locations on IR values,
  which is not DWARF and not a debugger experience.

### The `portal-pc-waffle` fork

A Waffle with GC types and a real backend, published as `portal-pc-waffle*`.

- Good, because it has everything upstream lacks:
  GC structs and arrays, reducify, stackify, treeify, localify,
  and fuzzed round-trips.
- Good, because its license matches ours, so algorithms can be adapted.
- Bad, because it is an alpha of a single-organization fork;
  its own debugging notes record feature combinations that do not compile,
  and the API moves.
- Bad, because its IR is a WASM-level SSA, not a word-level one:
  our MIR would be translated into a second SSA IR with a second optimizer,
  and the semantics of our word operators would have to survive the translation.
- Bad, because it does not emit DWARF;
  we would still own the layout, and now over a module structure of another crate.

### An in-tree emitter over `wasm-encoder`

- Good, because the GC type section is written by us, in the order we decide,
  exactly for the value model of [ADR-0018][0018-values-as-words.md].
- Good, because DWARF is part of assembly, not a patch on someone else's output.
- Good, because the emitter is a pair of pure passes,
  testable by snapshot and by execution like every other pass.
- Bad, because we implement structuring and local allocation ourselves;
  the fallback dispatch loop is the correctness answer while they are written.
- Bad, because emission bugs are ours to find,
  mitigated by validating and executing every module in tests.

## Links

- Values as words: [0018-values-as-words.md]
- MIR: [0019-mir.md]
- Translation units and linking: [0021-translation-units.md]
- Pipeline and stages: [0005-compiler-pipeline.md]
- Resolved types: [0017-resolved-types.md]
- Pass contract: [0009-pass-contract.md]
- The driver whose input includes the line indexes: [0008-compiler-driver.md]
- The line index codegen reads: [mlkc-line-index]
- Snapshot testing: [0006-snapshot-testing.md]
- Source and versions: [0007-vfs-file-state.md]
- Waffle issue #13, the missing GC types: <https://github.com/bytecodealliance/waffle/issues/13>
- The fork: <https://github.com/portal-co/waffle->
- `wasm-encoder`: <https://docs.rs/wasm-encoder>
- `gimli`: <https://docs.rs/gimli>
- Ramsey, _Beyond Relooper: recursive translation of unstructured control flow_:
  <https://doi.org/10.1145/1292586.1292591>
- DWARF for WebAssembly is read by Chrome DevTools and by `wasmtime`;
  LLVM's description of the format: <https://llvm.org/docs/DebugInfo.html>

[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0016-inter-module-resolution.md]: 0016-inter-module-resolution.md
[0017-resolved-types.md]: 0017-resolved-types.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0019-mir.md]: 0019-mir.md
[0021-translation-units.md]: 0021-translation-units.md
[mlkc-line-index]: ../../crates/mlkc-line-index
