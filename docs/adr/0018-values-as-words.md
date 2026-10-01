# Represent every value as one tagged word

- Status: accepted
- Date: 2026-10-01

## Context and Problem Statement

[ADR-0017][0017-resolved-types.md] fixes what a type _is_ at the source level:
a type class, a function, or a parameter,
resolved to names and nothing else.
It says nothing about what a value _is_ in a machine.

The back end cannot start without that answer.
MIR, the calling convention, the code generator, and every future back end
must agree on the shape of a value;
if each answers for itself,
the answer is repeated, and the repetitions drift.

The design fixed here, stated as the constraint it is:

- every value is one word of the machine;
- the word is tagged: a number lives in it directly,
  and a heap object is referred to by it;
- on WASM the word is a reference, and the immediates are `i31ref`;
- the WASM GC heap is used,
  so the types of aggregates are not erased.

The question:
which representation does a `Word` take,
what belongs to the language and what belongs to a back end,
and how does a uniform word meet a back end
whose reference and immediate types are its own?

## Decision Drivers

- Uniformity:
  a value of a type parameter is representable without knowing the type,
  and one calling convention covers every function.
- Inline immediates:
  `Int`, `Bool`, and `Unit` must not allocate.
- Untouched types:
  an aggregate keeps a GC type the engine can trace
  and the code generator can name,
  so field access does not become a dynamic test on a bag of bits.
- Back-end independence:
  MIR must not say `i31ref` or `eqref`;
  WASM and Cranelift tag differently, and both are planned.
- Deterministic semantics:
  the width and the overflow behavior of `Int`
  must be the same on every back end.
- Small first step:
  the first implementation carries only what the language can build today.

## Considered Options

- **Machine-typed values** — every value has its own machine type
  (`i32`, `i64`, a struct, a pointer);
  uniformity is lost, and generics need monomorphization.
- **Boxed values** — every value is a heap object; immediates are boxed too.
- **A tagged uniform word** — every value is one word;
  an immediate lives inside it, a heap object is referred to by it.

## Decision Outcome

Chosen option: "A tagged uniform word",
because it is the only option that keeps a value of a generic type
representable without monomorphization
while letting the common types (`Int`, `Bool`, `Unit`)
stay inside the word,
and because both planned back ends can implement it with their own tagging.

### A value is a `Word`

`Word` is the type of every MIR operand.
It has two cases, no more:

- an **immediate**: the value itself is in the word
  (`i31` on WASM; a tagged 64-bit immediate on Cranelift);
- a **reference**: the word refers to a heap object
  (`eqref` on WASM; a tagged pointer to a GC-managed heap on Cranelift).

What lives where:

| Source type           | Word case | WASM representation                                  |
| --------------------- | --------- | ---------------------------------------------------- |
| `Int`                 | immediate | `i31ref`                                             |
| `Bool`                | immediate | `i31ref`                                             |
| `Unit`                | immediate | `i31ref`                                             |
| a structure           | reference | a GC struct type (fields deferred)                   |
| `String`              | reference | a GC array of bytes (UTF-8)                          |
| a closure (later)     | reference | a GC struct wrapping a `funcref` and the environment |
| a large `Int` (later) | reference | a GC struct holding the wide value                   |

The table is the whole of the representation.
A back end may keep a _refinement_ of a word
(a value known to be an immediate, a value known to be an instance of a structure)
to emit casts and unboxing only where the kind is not known,
but the refinement is a property of the back end,
not of the word ([ADR-0020][0020-wasm-backend.md]).

### `Int` is a signed 31-bit integer, for now

The `i31` payload is 31 bits,
and the first implementation makes that a property of the language:

- `Int` holds `[-2^30, 2^30 - 1]`;
- arithmetic wraps modulo `2^31`,
  which is what `ref.i31` does when it truncates;
- a literal outside the range is a lowering diagnostic,
  not a silent truncation.

The alternative — `Int` as a full 64-bit value with a boxed escape —
is deliberately not taken now:
every arithmetic operator would become an unbox/box sequence,
and the first back end has more interesting things to prove.

The decision is supersedable without touching `Word`:
a large integer becomes another heap kind,
the small case stays an immediate,
and the operators either specialize on the known kind
or are replaced by runtime helpers.
What may not change is that the small case is inline.

### Tagging is a back-end concern

MIR names neither tags nor reference types.
Its operators say what they compute (`IntAdd`, `IntLt`, `RefEq`),
and each back end lowers them to its own instructions:

- WASM: `ref.cast (ref i31)`, `i31.get_s`, `i32.add`, `ref.i31`
  for `IntAdd`; `ref.eq` for `RefEq`;
- Cranelift (later): tagged arithmetic on a 64-bit word.

The same source program has the same semantics on both
because the same operations are lowered,
not because the words are laid out the same way.
The precise tag encoding,
and whether a native heap uses a tracing or a reference-counting collector,
are decided with the native back end, not here.

### The shape of an aggregate keeps its type

"Types are not erased" means, concretely:

- a structure has a WASM GC struct type of its own,
  and that type is what field access uses;
  a word is not a bag of bytes with fields poked into it;
- `String` is a GC array, and its length is the array's length;
- the type of a structure is declared identically in every module that names it,
  in a recursion group intrinsic to the structure
  (the singleton group of one structure, or the group of a mutually recursive cycle),
  so the structural canonicalization of the GC proposal
  makes the declarations one type ([ADR-0021][0021-translation-units.md]);
- a value crosses a module boundary as a `Word`, and its type does not have to:
  the reading module re-declares the type and reads the fields directly.

The engine can therefore trace every heap object,
and the code generator emits `struct.get`/`struct.set`
with a concrete type whenever a value's refinement proves it.

### Equality is per type, and the lowering chooses

The words are uniform, so `==` is not one operation.
The MIR lowering reads the type the checker gave the operands
and picks the primitive:

- two immediates: compare by value (`IntEq`, `BoolEq`);
- two structures: identity, `RefEq`, for now
  (structural equality of structures is not designed yet);
- two strings: content comparison through a helper,
  when strings arrive.

`ref.eq` is used where the operands are known to be references,
and never where the language means value equality of immediates.

### The invariants

- No word is ever null.
  WASM locals may be nullable references for a simpler encoding,
  but every word read is defined by construction;
  the verifier of a back end that relies on this checks it.
- A word is read as the type the source gave it.
  The checker is the only place a confusion is caught;
  the back ends may refine kinds, but not contradict them.

### Positive Consequences

- A generic value is a word: no monomorphization for representation,
  and one calling convention for everything ([ADR-0021][0021-translation-units.md]).
- `Int`, `Bool`, and `Unit` are free of allocation.
- The WASM GC heap does the allocation and the collection;
  no runtime library for memory is needed for the first subset.
- The types of aggregates are visible to the engine and to the debugger
  ([ADR-0020][0020-wasm-backend.md]).
- Cranelift arrives without a change to MIR or to the front end:
  only a new lowering of the same operators.

### Negative Consequences

- `Int` is limited to 31 bits until boxed integers are designed;
  the limitation is in the language, not only in one back end.
- A float, when it arrives, will be boxed;
  nothing in this record makes it inline on every back end.
- Uniformity hides static information:
  a back end must reconstruct kinds (a refinement pass on MIR)
  or pay casts and dynamic checks.
- Every word is a reference on WASM,
  so a value held in a WASM local is traced by the GC;
  a native word will need its own rooting discipline.

## Pros and Cons of the Options

### Machine-typed values

Each value carries the machine type its source type implies;
generics are eliminated by monomorphization or by boxing.

- Good, because the common operations are direct (`i32.add`),
  with no unboxing.
- Good, because a back end does not reconstruct kinds.
- Bad, because it contradicts the uniformity the language wants:
  a value of a type parameter has no machine type,
  so the representation decision moves into the checker.
- Bad, because every generic call site needs an instantiation,
  which the checker cannot produce today
  (its `Ty::Param` is a parameter, not an argument).

### Boxed values

Every value is a heap object; `Int` is a box holding an `i32`/`i64`.

- Good, because the representation is trivially uniform
  and the GC owns everything.
- Good, because equality is uniform when boxing is canonical.
- Bad, because every `1 + 1` allocates and compares pointers;
  the common case pays for the general one.
- Bad, because `i31ref` exists precisely to avoid this,
  and not using it wastes the target's design.

### A tagged uniform word

One word; the tag says immediate or reference;
the payload says which immediate or which object.

- Good, because the common types are inline
  and the general case still fits.
- Good, because one calling convention covers all functions
  and no call site is generic in representation.
- Good, because both targets have a natural implementation:
  `i31ref`/`eqref` on WASM, tagged 64-bit words on Cranelift.
- Bad, because every operation needs the tag convention
  to stay in step across back ends;
  the operators of MIR are the only allowed meeting point.

## Links

- WASM back end: [0020-wasm-backend.md]
- Translation units and the ABI: [0021-translation-units.md]
- Resolved types: [0017-resolved-types.md]
- Stable identity, which makes an entity's name a value: [0010-stable-entity-identity.md]
- The pipeline MIR belongs to: [0005-compiler-pipeline.md]
- WebAssembly specification, GC included: <https://webassembly.github.io/spec/core/>
- The GC proposal's overview, written before the final iso-recursive design:
  <https://github.com/WebAssembly/gc/blob/main/proposals/gc/Overview.md>

[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[0017-resolved-types.md]: 0017-resolved-types.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
[0021-translation-units.md]: 0021-translation-units.md
