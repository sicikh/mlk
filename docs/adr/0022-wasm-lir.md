# Give the WASM backend a LIR of its own

- Status: accepted
- Date: 2026-10-03

## Context and Problem Statement

MIR is a uniform SSA over words ([ADR-0018][0018-values-as-words.md], [ADR-0019][0019-mir.md]),
and the WASM backend lowers it to bytes in one step ([ADR-0020][0020-wasm-backend.md]).
That step has grown three jobs:

- the refinement of every value — what a word is known to be;
- the plan of which values are emitted where they are read;
- instruction selection, in two forms at once:
  a boxed one for a consumer that wants a word,
  and an unboxed one for a consumer that wants the number inside it.

The third is the problem.
A box and the unbox that follows it are instructions,
and the sequence `ref.i31` / `i31.get_s` ought to be removable
by a rewrite over instructions, by definitions and uses.
There is no such rewrite today, because there are no instructions to rewrite:
the emitter must remember not to emit the pair —
`prim` next to `prim_i32`,
`constant` next to `constant_i32`,
`unboxed` next to `value` —
and every construct the language grows adds another pair of emitters.
The same holds for the other representation decisions:
a cast that is not needed, a value that need not live in a local,
a value that is best emitted where it is read.
Each is a transformation over instructions,
and none of them has an IR to be a transformation of.

Behind that stands a second question:
should MIR itself become concrete, carrying `(ref i31)`, `eqref`, and boxes?
The answer is no, and it is the reason this record exists.
The representation of a value is a property of the target, not of the language.
The word model is the ABI ([ADR-0018][0018-values-as-words.md]);
the interpreter reads it;
and the native backend will read the same MIR and represent a word its own way —
Cranelift keeps its values in registers with its own tagging,
so `(ref i31)` cannot be what a word is above the backend seam
([ADR-0020][0020-wasm-backend.md]).
Each backend owns its representation,
and a backend that writes its own code generator
owns the IR over which the representation is optimized.

The question:
where do representation, casting, boxing, and local allocation live before bytes are written?

## Decision Drivers

- **We write the WASM code generator ourselves** ([ADR-0020][0020-wasm-backend.md]).
  No optimizer runs after us,
  so representation-level optimization is ours or nobody's —
  and an optimization needs a representation to run over.
- **MIR stays target-independent.**
  It is shared with the interpreter and, later, with Cranelift
  ([ADR-0018][0018-values-as-words.md], [ADR-0020][0020-wasm-backend.md]).
- **`wasm-encoder` is an encoder, not an IR.**
  It has no values, no definitions and uses, and no passes.
- **The pass contract and the snapshot strategy**
  ([ADR-0009][0009-pass-contract.md], [ADR-0006][0006-snapshot-testing.md]).
  What we optimize with must be a pure, total, deterministic function
  with a reading and a diff, like every other pass of the pipeline
  ([ADR-0003][0003-id-based-ir.md]).
- **Being close to the target is the point.**
  The layer is the target's own instruction set in SSA form,
  not a second general IR that would have to be taught WASM anyway.
  Waffle's IR is a WASM-level SSA for the same reason
  (<https://github.com/bytecodealliance/waffle>).
- **Debug information is ours** ([ADR-0020][0020-wasm-backend.md]).
  Line tables are spans on instructions,
  so they want an instruction stream that is a value of the compiler
  and not a side effect of an encoder.

## Considered Options

- **Keep lowering straight into the encoder** —
  the emitter keeps its analyses and its peepholes.
- **Make MIR concrete** — representation, casts, and boxes in the shared IR.
- **Borrow a WASM-level IR and its optimizer** —
  Binaryen, or the `portal-pc-waffle` fork rejected in [ADR-0020][0020-wasm-backend.md].
- **A LIR of our own between MIR and the encoder** — a WASM-level SSA the backend owns.

## Decision Outcome

Chosen option: "A LIR of our own between MIR and the encoder",
because the transformations we need are rewrites over instructions,
the instructions we target are WASM's,
and the place between MIR and `wasm-encoder` is empty.

### What the LIR is

The WASM LIR is a _low-level IR_: the target's own instructions in SSA form, which the backend owns.

- A **per-body SSA over machine values**:
  every value has a WASM type — `i32`, `(ref i31)`, `eqref`,
  and, when structures and strings are lowered, their GC types —
  and never the checker's `Ty`.
- **Value-based**: an instruction reads values and produces one.
  The stack is not in the IR; the stack is what encoding does.
- **No locals of its own**:
  a value is virtual until a pass decides that it has to live in a WASM local;
  the parameters of the function are the locals the ABI declares.
- **Blocks with parameters**, as in MIR ([ADR-0019][0019-mir.md]):
  a join takes a parameter,
  so there are no phi nodes and no second construction algorithm.
- **The target's instructions**:
  `i32.add`, `ref.i31`, `i31.get_s`, `ref.cast`, `ref.eq`, `struct.get $s`,
  calls by index, constants, and the terminators.
  A box and an unbox are instructions like any other —
  which is exactly what makes them removable by a pass.
- **Spans on instructions**, for the line tables of [ADR-0020][0020-wasm-backend.md].
- **A crate of its own**: `mlkc-lir-wasm`.
  The target is part of the name, because a LIR is a target's instruction set:
  the backend crate is `mlkc-codegen-wasm` for the same reason,
  and a native code generator of our own — a research project of its own,
  not the Cranelift path — would have a LIR named for its target too.
  The crate has its own model of the target's types and instructions,
  the way `mlkc-mir` owns the word-level ones,
  in the conventions of [ADR-0003][0003-id-based-ir.md]:
  arenas, ids, a `verify`, and a `dump`.
  It does not depend on `wasm-encoder`:
  encoding maps its types and instructions, and the IR stays a value of the compiler.
  The selection and the passes stay with the backend that owns the target
  until their weight asks for crates of their own,
  the way MIR has `-build` and `-opt`.
- **A stage of the driver**:
  the LIR of a body is keyed and memoized per body like `mir` and `mir-ssa`
  ([ADR-0008][0008-compiler-driver.md]),
  and computed against the module's layout,
  because the shape of a call is the signature of its callee.
  This is what lets the editor show a lowered body next to its CFG and SSA forms,
  and a snapshot of it be a reading a review can diff ([ADR-0006][0006-snapshot-testing.md]).

### The stages

MIR → **selection** → LIR → **passes** → **encoding**.
Selection and the passes are one stage of the driver:
`lir`, a pure function of the SSA body and the context of its function,
keyed and memoized per body like `mir` and `mir-ssa` ([ADR-0008][0008-compiler-driver.md]).
Encoding is the codegen stage, and it reads the LIR rather than the MIR body.

**Selection** is mechanical.
The refinements ([ADR-0020][0020-wasm-backend.md]) say what every MIR value is:
a value of an immediate kind is closed by `ref.i31`,
and an instruction that reads a number opens it with `i31.get_s`
(a structure, with `ref.cast`).
Selection decides nothing else:
it does not inline, does not keep a value virtual, and does not skip a box.
What it emits is what the types say, and the passes make it small.

**Passes** are where the decisions live, each a pure function with a dump
([ADR-0009][0009-pass-contract.md]):

1. remove a box that is opened at once,
   and a cast of a value that is already of the type it is cast to —
   a rewrite by definition and use, not a second emission mode;
2. copy propagation and dead code elimination over values;
3. local allocation:
   which values live in a WASM local, and which are emitted where they are read.
   The rule the emitter keeps today — one read, in the defining block,
   with no effect in between ([ADR-0020][0020-wasm-backend.md]) — becomes one pass here,
   and the invariants that rule protects become the pass's own;
4. later, on the same foundation:
   structuring the dispatch loop into `if`/`loop` regions
   ([ADR-0020][0020-wasm-backend.md]),
   coalescing locals by live range, and folding constants in LIR instructions.

**Encoding** walks the blocks and writes `wasm-encoder` calls:
no decisions, and the `Origin`s of the artifact are the spans the passes did not drop.
It produces the same `FuncArtifact` [ADR-0020][0020-wasm-backend.md] describes,
so `assemble_module` does not change;
`emit_function` is handed the LIR of a body instead of its MIR body.

### What stays on MIR

Everything semantic: simplifying the CFG, propagating copies, folding constants,
eliminating dead code, and inlining functions across modules
([ADR-0019][0019-mir.md]).
Those passes serve the interpreter and both backends and know nothing of WASM.
The LIR is where a decision that exists only because the target is WASM belongs.

### The native backend

This record does not apply to Cranelift.
The driver names a backend and MIR does not change ([ADR-0020][0020-wasm-backend.md]);
Cranelift _is_ the low-level IR, with its own representation, its own local allocation,
and its own optimizer.
We lower MIR to CLIF and stop.
A LIR of our own is the price of writing our own code generator,
and borrowing one is exactly what the native backend is for.

### Positive Consequences

- The LIR is a value of the driver:
  the editor shows a body lowered next to its CFG and SSA forms,
  and the counters time the stage like any other.
- A representation decision becomes a rewrite over instructions:
  the box/unbox pair, the redundant cast, and the value that need not live in a local
  are the same kind of program as everything else the compiler optimizes,
  with a dump to review ([ADR-0006][0006-snapshot-testing.md]).
- The emitter stops needing a second form of every construct:
  one selection pass replaces `prim`/`prim_i32`, `constant`/`constant_i32`, and `unboxed`.
- `localify` and control-flow structuring — the milestones of [ADR-0020][0020-wasm-backend.md] —
  get the home they lack today.
- Line tables get their source:
  spans live on LIR instructions, and encoding only assigns offsets.
- The backend becomes explainable in three stages a reader can test on their own.

### Negative Consequences

- A crate and a driver stage more:
  the driver keys, compares, invalidates, and drops the LIR of a body
  like every other value it holds ([ADR-0008][0008-compiler-driver.md]).
- A new IR to define, verify, dump, and test,
  with a pass pipeline that must stay pure and deterministic ([ADR-0009][0009-pass-contract.md]).
- The LIR is WASM-specific and is not reused by Cranelift;
  the word-level passes on MIR must therefore carry the weight common to both backends.
- Being one-to-one with the target means the LIR owes its shape to another specification:
  when WASM grows a feature, the LIR follows it, and not the other way around.
- Until the passes exist, the LIR is a step that costs work and buys correctness only;
  the first version must re-earn the output the emitter already produces.

## Pros and Cons of the Options

### Keep lowering straight into the encoder

- Good, because nothing new is defined and the pipeline is as short as it gets.
- Good, because the current output is reviewed snapshot by snapshot.
- Bad, because every representation decision has to be expressed as
  "how to emit this construct", and a construct needs as many emission modes
  as it has kinds of consumers.
  The modes multiply, and the pairs they leave behind are collapsed by hand.
- Bad, because an optimization over boxing and casting has no values to rewrite,
  only an encoder that must keep remembering not to emit.
- Bad, because `localify` and structuring have no representation between MIR and bytes
  to be computed over.

### Make MIR concrete

- Good, because one IR stays in step,
  and the interpreter would read the same types the backend does.
- Bad, because representation is target-specific:
  Cranelift's word is not `(ref i31)` and its tagging is not WASM's,
  so a concrete MIR would be a WASM-only MIR with a Cranelift-shaped exception in it.
- Bad, because it erodes the uniform word model
  that the ABI, the checker, and the interpreter are written against
  ([ADR-0018][0018-values-as-words.md]).

### Borrow a WASM-level IR and its optimizer

Binaryen, or the `portal-pc-waffle` fork rejected in [ADR-0020][0020-wasm-backend.md].

- Good, because the IR exists and so do its passes.
- Bad, because it takes over our optimizations and our debug information,
  which [ADR-0020][0020-wasm-backend.md] rejected for its own reasons,
  and those reasons have not changed.
- Bad, because we would translate MIR into a second SSA IR with a second optimizer,
  and then read that IR's output to find our line tables.

### A LIR of our own between MIR and the encoder

- Good, because it is the smallest layer that makes representation decisions data:
  the target's instructions, values, and passes.
- Good, because it keeps MIR target-independent and the backend seam where
  [ADR-0020][0020-wasm-backend.md] put it.
- Good, because being one-to-one with WASM is intended:
  instruction selection stays mechanical, and the passes do the thinking.
- Good, because it is a crate and a stage a host can read and show,
  not a private detail of the encoder.
- Bad, because it is another IR to own,
  and its first version must re-earn the output the emitter already produces.

## Links

- Values as words: [0018-values-as-words.md]
- MIR: [0019-mir.md]
- WASM backend: [0020-wasm-backend.md]
- ID-based IRs and their conventions: [0003-id-based-ir.md]
- The driver whose stages these are: [0008-compiler-driver.md]
- Pass contract: [0009-pass-contract.md]
- Snapshot testing: [0006-snapshot-testing.md]
- Pipeline and stages: [0005-compiler-pipeline.md]
- Waffle, whose IR is a WASM-level SSA: <https://github.com/bytecodealliance/waffle>
- Cranelift, the low-level IR of the native backend:
  <https://github.com/bytecodealliance/wasmtime/tree/main/cranelift>

[0003-id-based-ir.md]: 0003-id-based-ir.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0019-mir.md]: 0019-mir.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
