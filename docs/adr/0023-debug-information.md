# Emit debug information, and let a host configure it

- Status: superseded by [ADR-0025](0025-debug-information-formats.md)
- Date: 2026-10-03

## Context and Problem Statement

[ADR-0020][0020-wasm-backend.md] makes DWARF a requirement of the WASM backend,
and [ADR-0022][0022-wasm-lir.md] gives the backend an IR to lower from;
but the backend of today emits only the `name` section,
and the levels of `DebugLevel` differ in nothing.
Two questions are open at once:

- what each level of debug information must contain
  for the consumers that exist, and
- how a host asks for a level, and for the optimizations that trade against it.

The consumers are not one kind of tool.
A browser debugger, a native debugger attached to a runtime,
a symbolicator of crash reports, and a profiler
each read a different part of the same custom sections.
The answer therefore cannot be "DWARF on or off":
it is a small set of levels, each a contract with named consumers.

## Decision Drivers

- **Line tables are the floor.** Stepping through source,
  a stack trace that names a file and a line,
  and `addr2line`-style symbolication are what every consumer reads,
  and what no consumer can do without.
- **The browser is a consumer with a narrow door.**
  Chrome reads WASM DWARF only through the
  C/C++ DevTools Support (DWARF) extension
  (<https://developer.chrome.com/docs/devtools/wasm/>,
  <https://developer.chrome.com/blog/wasm-debugging-2020/>);
  Firefox does not read DWARF at all and offers source maps only
  (<https://emscripten.org/docs/porting/Debugging.html>).
- **The extension reads values through the runtime, and memory is what it reads.**
  The extension parses DWARF with the LLDB parser and asks V8 for the data
  (<https://jonasdevlieghere.com/post/wasm-debugging/>);
  a value whose location is a WASM local
  (`DW_OP_WASM_location 0x00`, the vendor opcode of the
  [DWARF for WebAssembly][dwarf-wasm] specification)
  is not what it shows reliably,
  and a GC reference has no address to be read at all:
  our structures are `struct` types of the GC proposal,
  not bytes in linear memory ([ADR-0018][0018-values-as-words.md]).
  Showing variables in a browser would need a debug-only shadow representation
  in linear memory, which this record does not build.
- **Wasmtime is the consumer that reads locals.**
  Wasmtime translates WASM DWARF to the addresses of its JIT code,
  so `gdb` and `lldb` attached to it step through source and read values
  (`wasmtime run -D debug-info`, with `-O opt-level=0` recommended for locals,
  <https://docs.wasmtime.dev/examples-debugging-native-debugger.html>).
  Its own debugger is a plan, not a tool: the accepted RFC adds core dumps
  (already generated on a trap) and a DAP server later
  (<https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-debugging.md>).
- **Addresses are code-section offsets.**
  DWARF for WebAssembly measures code addresses
  from the start of the contents of the code section
  ([dwarf-wasm]; the same convention `wasm-tools addr2line` implements,
  <https://github.com/bytecodealliance/wasm-tools>).
- **Configuration is an input of the driver.**
  [ADR-0008][0008-compiler-driver.md] already names the options among the inputs
  that are pushed, versioned, and part of what a value is a function of.
- **Optimization is ours, and so is its price.**
  The target's own optimizer is the LIR passes ([ADR-0022][0022-wasm-lir.md]),
  and a level that gates them must be configuration
  rather than a second pipeline.

## Considered Options

- **Emit DWARF always, at one level** — no configuration, no levels.
- **Emit line tables only, and stop there** — no variables anywhere.
- **Levels, with the lowest useful one first** — lines now,
  variables when a pass can emit locations, and a record of what each consumer reads.
- **Source maps instead of DWARF** — the format Firefox reads.

## Decision Outcome

Chosen option: "Levels, with the lowest useful one first".

### The levels

`DebugLevel` ([`mlkc-codegen-wasm`][codegen]) is the knob; the name section is
emitted at every level, because a stack trace without names is not readable.

- **`None`.** The `name` section only: module, function, and local names.
  This is what a release build ships.
- **`Lines`.** The name section, a compile unit per module, a subprogram per
  function with `DW_AT_name` and `DW_AT_low_pc`/`DW_AT_high_pc`, and a line
  program per module mapping code-section offsets to file, line, and column.
  This is what Chrome steps through,
  what `wasm-tools addr2line` reads,
  and what a truncated backtrace needs.
- **`Full`.** `Lines`, plus the DIEs of parameters and locals with
  `DW_AT_location` as `DW_OP_WASM_location`, and the source types the checker
  gave them. The consumers are `gdb`/`lldb` on top of Wasmtime and any tool
  that implements the vendor opcode; the browser extension does not, and a GC
  value has nothing to expand anyway. `Full` and `Lines` agree until the pass
  that gives a value a location exists.

A debug-only shadow representation of values in linear memory, so that a
browser can show them, is deliberately not in this record: it is a change to
what a running program is, not to what its debug sections say.

### The options

The driver gains `Options`, pushed by the host like text:

```rust
pub struct Options {
    /// How much debug information the assembled modules carry.
    pub debug: DebugLevel,
    /// How hard the passes try to make the program small and fast.
    pub opt: OptLevel,
}
```

- `OptLevel::None` runs only the passes the pipeline needs to be correct ---
  selection, collapsing, structuring, and allocation
  ([ADR-0022][0022-wasm-lir.md]) --- and no optimization.
  `OptLevel::Full` is where the passes to come --- constant folding, copy
  propagation, dead code elimination, inlining ([ADR-0019][0019-mir.md]) ---
  will be gated. The levels produce the same code today, because no optional
  pass exists yet; the field is the point of extension, not a promise.
- The options are a versioned input ([ADR-0008][0008-compiler-driver.md]):
  a push that changes them invalidates what was built under them.
  The link of a project is a function of its modules **and** of the options,
  because the assembled bytes carry the debug tables;
  the LIR of a body will join that key when a pass reads `opt`.
- The editor and the CLI are hosts and set the same record;
  the editor shows the debug sections a level produced.

### What the tables are made of

The DWARF is written with `gimli` by `assemble_module`, in one deterministic
pass, after the code section is laid out:

- **Addresses** are offsets from the start of the code section contents,
  computed from the bodies the assembler writes;
  a function begins at its body content --- the locals declaration that its
  length prefix introduces --- and an `Origin` of it contributes that plus the
  offset of the instruction inside the body.
- **A line program** is built from the `Origin`s of every function
  ([ADR-0020][0020-wasm-backend.md]): the driver hands codegen the path and the
  `LineIndex` of every file a body may name, as part of the input of the stage,
  and codegen never reads a file.
- **A compile unit** per module is named by the module's canonical path,
  and holds a subprogram per function, named by the name the function is
  exported under, with `low_pc` at the body content and `high_pc` its length.
- **The custom sections** are `.debug_abbrev`, `.debug_info`, `.debug_line`,
  `.debug_str`, and `.debug_line_str` where `gimli` needs it;
  the engine ignores them at runtime, and a host may strip them.

A module assembled at `Lines` or `Full` therefore differs byte for byte from
one assembled at `None`, and nothing but the tables differs.

### What each consumer reads

| Consumer                                    | `None`                  | `Lines`                                               | `Full`                                            |
| ------------------------------------------- | ----------------------- | ----------------------------------------------------- | ------------------------------------------------- |
| Chrome DevTools + DWARF extension           | names in stack traces   | source stepping, breakpoints, line-level stack traces | as `Lines`; locals and GC values are out of reach |
| `wasmtime` + `gdb`/`lldb`                   | names, line-less frames | symbolication, stepping                               | locals and scalars, per the location expressions  |
| `wasm-tools addr2line`, crash symbolication | function names          | file, line, column                                    | same                                              |
| Wasmtime core dumps ([RFC][wasmtime-rfc])   | frames                  | symbolized frames                                     | values, once core dumps carry them                |

[dwarf-wasm]: https://yurydelendik.github.io/webassembly-dwarf/
[codegen]: ../../crates/mlkc-codegen-wasm
[wasmtime-rfc]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-debugging.md

### Positive Consequences

- A host chooses what a build carries:
  a release carries names, a debug build carries the line tables,
  and the level is visible in the bytes and in the editor.
- The line tables have one source of truth, the `Origin`s the emitter already
  records, so a pass that moves an instruction moves its line with it.
- The configuration lives in the driver, next to the other inputs,
  and not in a config file format of its own.
- The level names say what a consumer gets, so a bug report can say
  "assembled at `Lines`" and be reproduced.

### Negative Consequences

- The options are one more versioned input:
  the driver memorizes them, and a changed level re-links a project.
- The debug sections are ours to keep correct as the backend grows;
  `gimli` writes them but does not check our addresses.
- `Full` is a promise the browser cannot keep:
  a reader of the editor may expect to inspect variables,
  and only a native debugger will show them.

## Pros and Cons of the Options

### Emit DWARF always, at one level

- Good, because there is nothing to configure and nothing to get wrong.
- Bad, because debug information is size a release build must not carry
  ([ADR-0021][0021-translation-units.md] ships modules as they assemble).
- Bad, because the line tables are useful without the DIEs,
  and the levels are the honest way to say what is emitted.

### Emit line tables only, and stop there

- Good, because it is the whole of what the browser and `addr2line` read.
- Bad, because Wasmtime and its debuggers can read locals,
  and the DIEs to let them are not much more work than the line tables.

### Levels, with the lowest useful one first

- Good, because each level names its consumers and can be tested as a contract.
- Good, because the levels are configuration, and configuration is an input
  the driver already knows how to version ([ADR-0008][0008-compiler-driver.md]).
- Bad, because `Full` arrives later than `Lines`,
  and the levels therefore differ in code before they differ in a manual.

### Source maps instead of DWARF

- Good, because every browser reads them without an extension.
- Bad, because they carry lines only,
  which is what `Lines` already gives and nothing more.
- Bad, because they are a second format with a second emitter,
  and the requirement of [ADR-0020][0020-wasm-backend.md] is DWARF.

## Links

- Values as words: [0018-values-as-words.md]
- MIR and its passes: [0019-mir.md]
- WASM backend, whose DWARF plan this record implements: [0020-wasm-backend.md]
- Translation units: [0021-translation-units.md]
- WASM LIR: [0022-wasm-lir.md]
- The driver whose inputs the options join: [0008-compiler-driver.md]
- Pass contract: [0009-pass-contract.md]
- Snapshot testing: [0006-snapshot-testing.md]
- DWARF for WebAssembly: <https://yurydelendik.github.io/webassembly-dwarf/>
- Chrome DevTools and its DWARF extension:
  <https://developer.chrome.com/docs/devtools/wasm/>
- Wasmtime native debugging:
  <https://docs.wasmtime.dev/examples-debugging-native-debugger.html>
- Wasmtime debugging RFC:
  <https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-debugging.md>
- `wasm-tools`, whose `addr2line` pins the address convention:
  <https://github.com/bytecodealliance/wasm-tools>

[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0019-mir.md]: 0019-mir.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
[0021-translation-units.md]: 0021-translation-units.md
[0022-wasm-lir.md]: 0022-wasm-lir.md
