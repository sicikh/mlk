# Compile one module to one WASM module, and link by function imports

- Status: accepted
- Date: 2026-10-01

## Context and Problem Statement

[ADR-0004][0004-module-system.md] makes the module the unit of incrementality,
and [ADR-0005][0005-compiler-pipeline.md] makes codegen per function and linking sequential,
but neither says what a compiled _program_ is on disk or in memory:
how many WASM modules a project becomes,
what a module exports and imports,
and how the pieces find each other.

The intended shape is known:

- a module of the language becomes a module of WASM;
- modules are linked dynamically, not statically;
- the ABI is uniform because every value is a `Word`
  ([ADR-0018][0018-values-as-words.md]);
- the GC types of structures are declared where they are named,
  and unified by the proposal's canonicalization
  ([ADR-0020][0020-wasm-backend.md]).

The question:
what exactly crosses a module boundary,
how is it named,
who resolves the names,
and which of the several meanings of "dynamic linking" is meant?

## Decision Drivers

- Incrementality:
  editing one body must recompile one function and one module,
  not the project.
- Parallelism:
  modules compile in parallel; only the resolution of names between them is sequential
  ([ADR-0005][0005-compiler-pipeline.md]).
- Simplicity of the ABI:
  a uniform word representation makes the calling convention uniform;
  a structure's type is re-declared where it is named
  and unified by the GC proposal's canonicalization,
  so no layout is negotiated at link time.
- Host simplicity:
  the browser playground, the CLI, and tests must be able to load a project
  without a linker format of our invention.
- Debuggability:
  a debugger must be able to attribute a function to a module of the language.
- Stability:
  the standard library ([ADR-0015][0015-standard-library.md]) is compiled by the same
  pipeline, and may one day ship precompiled.

## Considered Options

- **One WASM module for the whole program** — the classic static link.
- **One WASM module per function** — maximal separation.
- **One WASM module per module, linked by core WASM imports and exports.**
- **The WebAssembly dynamic-linking proposal** — a shared library format with
  `dylink` sections and a runtime linker.

## Decision Outcome

Chosen option: "One WASM module per module, linked by core WASM imports and exports",
because it is exactly the incrementality unit of [ADR-0004][0004-module-system.md],
and because the uniform word ABI removes everything that makes
separate compilation hard in other languages:
no layout has to be negotiated between modules,
no generic instantiation crosses,
and the only signature that ever exists between modules is
words in, a word out.

"Dynamic linking" here means _separate compilation with resolution at instantiation_,
the ordinary meaning of core WASM imports;
it does not mean the WebAssembly dynamic-linking proposal,
which is neither implemented by the target engines for GC types
nor needed for what this record asks.

### The artifact

`assemble_module` ([ADR-0020][0020-wasm-backend.md]) produces:

```rust
pub struct WasmModule {
    /// The encoded module.
    pub bytes: Vec<u8>,
    /// What it needs from other modules: the module, the name, and the arity.
    pub imports: Vec<ImportDecl>,
    /// What it offers: the module, the name, and the arity.
    pub exports: Vec<ExportDecl>,
}
```

The import and export tables are returned, not parsed back out of the bytes:
the assembler knows them, and the linker consumes them.

### The ABI

- **Every function is `(eqref, ...) -> eqref`.**
  Parameters are words, the result is a word, and `Unit` is a word.
  There are no multi-value results yet, and no `i32` in a signature.
- **A word is the only thing that crosses.**
  `i31ref` is a subtype of `eqref`,
  so an immediate passes where a word is expected;
  the converse does not hold,
  which is why the signature is `eqref` and not `i31ref`:
  one word type covers an immediate and a reference alike.
  A structure crosses as a word too, and its type is not in the signature:
  the reader declares the type itself (the next section).
  A callee that needs a kind recovers it once, where the value enters:
  `ref.cast (ref i31)` for an immediate, `ref.cast (ref $s)` for a structure.
- **A word is not marshalled.**
  Both modules run in one engine and one store,
  so a word handed across a boundary is an ordinary reference
  to the shared GC heap:
  nothing is copied, encoded, or registered.
- **Names are canonical.**
  The import module field is the canonical path of the source module
  (`std::core`), and the name field is the entity's name,
  with the disambiguator of [ADR-0010][0010-stable-entity-identity.md] when there is one.
  A function `double` of `std::core` is imported as `("std::core", "double")`
  and exported the same way.
  The mapping from a name back to a module of the language is the module's path,
  so a stack trace and a debugger have the same attribution a resolution has.

### Structures across modules

A structure is declared in one module of the language,
and its layout — the order, the types, and the mutability of its fields —
is part of that module's surface,
read by the modules that name the structure the way `ModuleTypes` is read
([ADR-0017][0017-resolved-types.md]).

Its WASM GC type is another matter.
Every module that names the structure declares the type itself,
byte for byte as the declaring module does:

- a structure that does not take part in a recursive cycle is a recursion group of one;
- a cycle of mutually recursive structures is one recursion group,
  and every module that names a member emits the whole group,
  because the members of a cycle are a property of the program, not of a module.

Groups are emitted in dependency order,
so a group references only groups that are already declared before it.

The GC specification closes every recursion group before comparing types:
the type indices internal to a group are replaced by recursive type indices,
so two groups of the same recursive structure become syntactically equal,
and equivalence is that equality.
A defined type is a projection from the group it belongs to,
so the group is part of the type's identity:
the same structure declared in a group of one and declared inside a larger group
are two different types.
That is why the grouping above is a rule and not a choice of the assembler.

The closed representation is independent of a module's type index space,
which is what makes it meaningful across module boundaries:
two identical declarations of a recursion group are the same type in an engine,
for any pair of modules loaded into one store.
A `struct.get $s` therefore reads a value another module created,
and the entry cast of a word to `(ref $s)` succeeds,
with no accessor call and no layout negotiation at link time.

What this costs:

- every module carries the declarations of the structures it names,
  a few bytes per structure and nothing else;
- the declaration must be a pure function of the structure
  (the surface gives the fields and their order), or the declarations diverge;
- a divergence surfaces as a failing cast at run time,
  which is why the assembler derives the declaration from one shared function,
  and why the tests compare the declarations two modules emit
  for a structure they both name.

The alternative, an accessor call per foreign field access, is rejected:
it turns a field read into a call,
and it makes a structure behave differently depending on where it is read from.
Type imports would give shared nominal declarations,
but they are a separate proposal, outside the GC MVP,
and not what the target engines implement.

### The link stage

The driver's linking stage is sequential ([ADR-0005][0005-compiler-pipeline.md]):

1. collect the `WasmModule` of every module of the project;
2. build the import graph from the import tables;
3. for every import, find the provider and check the arity;
   a missing provider or a wrong arity is a diagnostic, not a runtime failure;
4. order the modules topologically and hand the order and the artifacts to the host.

The import graph is acyclic by design:
a module never takes part in an import cycle,
so a topological order always exists and no lazy resolution is needed.

The stage produces a _manifest_, not a merged binary:
a value a host can read,

```rust
pub struct LinkPlan {
    pub order: Vec<ModuleId>,
    pub modules: BTreeMap<ModuleId, Arc<WasmModule>>,
    pub entry: Option<(ModuleId, String)>,
}
```

- In the browser, the host (the playground's worker) passes the exported functions
  of already-instantiated modules as the import object of the next,
  and calls the entry point.
- In `wasmtime`, the same plan is walked with a `Linker`.
- In the CLI, the plan is written as `.wasm` files and a manifest.

Nothing merges modules, and no engine extension is required.

### What is compiled when an edit happens

- **A body edit** re-runs `emit_function` for that body
  and `assemble_module` for its module.
  Other modules are not touched: their import tables name entities, not bodies.
- **An interface edit** (a signature, an export) invalidates the modules that read it,
  by the key rules of [ADR-0008][0008-compiler-driver.md]:
  their bodies recompile only when their values change,
  and a change in the arity of an import forces their module to be re-assembled.
- **The linker index** is rebuilt from the interfaces, cheaply, the way
  [ADR-0016][0016-inter-module-resolution.md] rebuilds the module index.

Cross-module inlining does not happen in the backend:
it is the Thin-LTO stage at MIR level ([ADR-0005][0005-compiler-pipeline.md]),
planned as a release-build optimization,
so a debug build keeps the module boundary and the imported calls.

### The standard library

The library is compiled like any other project
([ADR-0015][0015-standard-library.md]):
its modules become WASM modules with the same ABI,
and the project's modules import them by canonical path.

Shipping it precompiled is possible precisely because this record fixes the ABI
and the names;
the first implementation compiles it in process,
the way it compiles user code.

### Debugging

DWARF is per module ([ADR-0020][0020-wasm-backend.md]):
each module's debug info references its own code section and its own source files.
A debugger that loads a `LinkPlan` knows which instance a frame belongs to
from the manifest, which is why the manifest carries module identities
and not only bytes.

### Positive Consequences

- The compilation unit of the compiler is the unit of the output;
  there is no second model to keep in step.
- The ABI is one sentence:
  words in, a word out, no layout in a signature.
- The host does the linking, and every host already knows how to:
  function imports and exports are the only feature used.
- A structure's GC type is a pure function of the structure,
  so two modules that name it agree on it without a linker step.

### Negative Consequences

- A project is many small modules, each with its own type and function sections;
  the structure types a module names are part of that duplication,
  and it costs bytes and instantiation time.
- A cross-module call is an imported function call;
  inlining across modules is the Thin-LTO stage, planned for release builds,
  not part of this record.
- Precompiled libraries must be recompiled when the ABI or the codegen changes;
  there is no stable binary interface beyond this record's word ABI.

## Pros and Cons of the Options

### One WASM module for the whole program

- Good, because it is the simplest to run: one module, one instantiation.
- Good, because cross-module calls are ordinary calls and can be inlined.
- Bad, because every edit recompiles the world,
  which contradicts [ADR-0004][0004-module-system.md] and the IDE budget.
- Bad, because the linkage information (which function of which module)
  is thrown away exactly where the debugger wants it.

### One WASM module per function

- Good, because invalidation is as fine as it gets.
- Bad, because a call between two functions of one module becomes an import,
  and the module structure of the language disappears from the output.
- Bad, because the number of instantiations and imports grows with the program.

### One module per module, linked by imports

- Good, because it matches every incrementality decision made so far.
- Good, because the ABI is uniform and a GC type is re-declared, not negotiated.
- Good, because every host can link it with the tools it already has.
- Bad, because a project is a set of modules, and a host must walk the plan.
- Bad, because startup is per module, not per program.

### The dynamic-linking proposal

A relocatable object format, `dylink` sections, a runtime linker,
and shared memories and tables.

- Good, because it is designed for C-like shared libraries,
  including symbol interposition and lazy binding.
- Bad, because it is not implemented for GC types by the engines we target,
  and its future is uncertain.
- Bad, because its features solve problems this language does not have:
  no layout is negotiated, no symbol is interposed, no library is loaded lazily.

## Links

- Module system and incrementality: [0004-module-system.md]
- Pipeline and stages: [0005-compiler-pipeline.md]
- Driver keys: [0008-compiler-driver.md]
- Inter-module resolution: [0016-inter-module-resolution.md]
- Resolved types: [0017-resolved-types.md]
- Values as words: [0018-values-as-words.md]
- WASM codegen: [0020-wasm-backend.md]
- Standard library: [0015-standard-library.md]
- Stable identity of entities, which makes a name canonical: [0010-stable-entity-identity.md]
- Recursive types, rolling up, and type equivalence:
  <https://github.com/WebAssembly/spec/blob/main/document/core/valid/conventions.rst>
- Matching (subtyping and equivalence) of defined types:
  <https://github.com/WebAssembly/spec/blob/main/document/core/valid/matching.rst>
- WebAssembly core, imports and instantiation:
  <https://webassembly.github.io/spec/core/exec/modules.html>

[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[0015-standard-library.md]: 0015-standard-library.md
[0016-inter-module-resolution.md]: 0016-inter-module-resolution.md
[0017-resolved-types.md]: 0017-resolved-types.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
