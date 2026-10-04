# Choose one debug information format per module

- Status: accepted
- Date: 2026-10-04

## Context and Problem Statement

[ADR-0023][0023-debug-information.md] made DWARF the debug information of a module
and named the browser among its consumers,
and [ADR-0024][0024-browser-debug-information.md] added a source map beside it.
Both records are now superseded, and the reason is the browser.

**DWARF in a browser is not a path to a program a person can inspect.**
Chrome reads the DWARF of a module only through the C/C++ DevTools Support (DWARF) extension,
and the variables it shows are values read out of linear memory:
the extension's LLDB backend reads a location
(`DW_OP_WASM_location` of the [DWARF for WebAssembly][dwarf-wasm] convention)
and then reads bytes, which for a variable in memory is a load
and for a variable in a WASM local is one of the few forms it can follow.
Our values are not bytes in memory:
a word is an `i31` immediate or a GC reference ([ADR-0018][0018-values-as-words.md]),
and a structure is a GC struct type ([ADR-0020][0020-wasm-backend.md]),
so the extension can show neither the immediate inside a value
nor the fields behind a reference.
To let it show them, a debug build would have to keep a shadow copy of every value
in linear memory as the program runs ---
a debug-only representation that changes what a running program is,
allocates on every instruction, and exists for a debugger that still cannot be
the debugger of the language.
That price is not worth paying, and this record refuses it.

**What remains of DWARF in a browser is lines.**
The extension maps instructions to source lines,
which is exactly what the source map gives without an extension,
without an installed host, and with the text of the files carried by the module itself.
The map is what the browser reads, and it works.

**Two formats in one module do not coexist well.**
A module that carries DWARF and a map is a module whose symbols an engine prefers to DWARF:
DevTools ranks `ExternalDWARF`, `EmbeddedDWARF`, and only then `SourceMap`
([debugger-model]),
and the extension claims every module of the language `WebAssembly`
that carries `EmbeddedDWARF` or `ExternalDWARF` ([devtools-plugin-host]).
DevTools then translates locations through the plugin first ([debugger-workspace-binding]),
and the source files it creates are loaded from the host ([debugger-language-plugins]):
a page without a source endpoint --- which is every deployment of the editor ---
shows `Could not load content` instead of a source.
The map cannot win while the DWARF is there; the priority is the engine's.

## Decision Drivers

- **The browser is a consumer that asks for one thing.**
  A source map with the sources inside it is the whole of what a browser can read
  from our values, and it needs no extension and no host
  ([ADR-0024][0024-browser-debug-information.md]).
- **`lldb` and `gdb` are consumers whose format is DWARF.**
  Wasmtime translates the DWARF of a module to its JIT code,
  and a debugger attached to it steps through source and reads values
  ([ADR-0023][0023-debug-information.md]);
  `wasm-tools addr2line` and core dumps read the same tables.
  These consumers do not share a format with a browser.
- **A module is easier to reason about when it carries one format.**
  Two formats in one module forced a host to strip one of them,
  and left the rank between them to an engine
  (`WASM_SYMBOLS_PRIORITY`, the priority of DevTools) that no page controls.
- **Configuration is an input of the driver.**
  A host already pushes options and the driver versions them
  ([ADR-0008][0008-compiler-driver.md]);
  which format a module carries is one more value of the same kind.

## Considered Options

- **Keep DWARF and the map in one module, as [ADR-0023][0023-debug-information.md]
  and [ADR-0024][0024-browser-debug-information.md] describe** ---
  the model that failed: the engine prefers the DWARF, the extension claims the module,
  and the browser cannot show anything the DWARF adds.
- **DWARF in a browser, with a shadow representation of values in linear memory** ---
  the extension could then show values, at the price of a second representation of
  every value of a running program.
- **A source map everywhere, and DWARF nowhere** ---
  one format for all consumers, and `lldb`/`gdb` lose what only DWARF gives them.
- **Options with one format per module** ---
  nothing at all, the source map of a browser, DWARF lines, and DWARF full,
  with the option deciding which one a module carries.

## Decision Outcome

Chosen option: "Options with one format per module".

`DebugInfo` ([`mlkc-codegen-wasm`][codegen]) is the option,
and a module carries exactly what it names:

- **`None`.** The `name` section alone. This is what a build that carries no debug
  information asks for; a stack trace still names its functions.
- **`SourceMap`.** The `name` section and a source map:
  a JSON version 3 map in a `data:` URL in the `sourceMappingURL` custom section,
  with `sourcesContent` carrying the text of every file the bodies were read from.
  A segment stands on generated line zero,
  and its generated column is the byte offset of the instruction in the module
  --- the convention Emscripten writes and every engine reads
  ([ADR-0024][0024-browser-debug-information.md]).
  The original line and column count from zero, as the `LineIndex` does.
  This is the debug information of the editor and of a program a browser runs.
- **`DwarfLines`.** The `name` section, a compile unit per module, a subprogram per
  function with `DW_AT_name` and `DW_AT_low_pc`/`DW_AT_high_pc`, and a line program
  mapping code-section offsets to file, line, and column.
  This is the debug information of `wasmtime` with `gdb`/`lldb`, of `wasm-tools
addr2line`, and of a crash report.
- **`DwarfFull`.** `DwarfLines`, plus the DIEs of parameters and locals with
  `DW_AT_location` and the source types the checker gave them, once a pass emits
  locations. Until that pass exists, this option carries what `DwarfLines` carries.

The `name` section is written at every option, because a stack trace without names
is not readable; `None` is that section alone. The default is `SourceMap`:
a host that does not think about debug information is a browser.

No option carries both formats. A module of a browser has no DWARF for an engine to
prefer, and a module of a debugger has no map to be preferred to it;
the host [ADR-0021][0021-translation-units.md] runs the bytes it is handed,
and nothing has to be stripped on the way.

### What each consumer reads

| Consumer                                    | `None`                  | `SourceMap`                                          | `DwarfLines` / `DwarfFull`           |
| ------------------------------------------- | ----------------------- | ---------------------------------------------------- | ------------------------------------ |
| Browser DevTools                            | names in stack traces   | source stepping, breakpoints, the sources themselves | nothing (the extension needs a host) |
| `wasmtime` + `gdb`/`lldb`                   | names, line-less frames | names, line-less frames                              | symbolication, stepping, locals      |
| `wasm-tools addr2line`, crash symbolication | function names          | function names                                       | file, line, column of an instruction |

A shadow representation of values, so that a browser could show them,
is neither built nor planned here:
this record refuses it rather than defers it.

### Positive Consequences

- The debug information of a browser is what a browser actually reads,
  and it works without an extension and without a host.
- A module carries one format, so no consumer can prefer another one,
  and a host that runs a module does not have to take one apart.
- DWARF keeps every consumer it had, and the tests of the tables keep watching it.
- The option is an input the driver already versions ([ADR-0008][0008-compiler-driver.md]),
  so which format a module carries is visible in the bytes and in the editor.

### Negative Consequences

- A program built for a browser cannot be stepped through the C/C++ extension,
  by design; the browser path is the map, and nothing else is promised.
- A build that carries nothing still carries the `name` section:
  there is no option that strips names too, because a stack trace without names
  is not readable.
- `DwarfFull` is a promise that arrives later: it carries what `DwarfLines` carries
  until a pass emits a location for a value.
- The map is redundant with a build that only a native debugger reads,
  and DWARF is dead weight in a build that only a browser reads;
  the option is what a host picks between them.

## Pros and Cons of the Options

### Keep DWARF and the map in one module

- Good, because the module serves both consumers at once.
- Bad, because the browser cannot show what the DWARF adds:
  the extension reads memory, and our values are not in memory.
- Bad, because the engine prefers the DWARF to the map, the extension claims the module,
  and the sources it names are fetched from a host that is not there.

### DWARF in a browser, with a shadow representation of values

- Good, because the extension could show values of the language.
- Bad, because it changes what a running program is:
  every value is copied into a debug-only region of linear memory as the program runs.
- Bad, because it exists for a consumer that still knows nothing of the types of the language
  without more metadata than DWARF can carry.

### A source map everywhere, and DWARF nowhere

- Good, because there is one format to write and to keep correct.
- Bad, because `gdb` and `lldb` lose values, types, and stack frames as a person reads them,
  and `wasmtime` debugging is what the DWARF of a module is for.

### Options with one format per module

- Good, because each consumer gets the format it reads, and a host says which one it is;
  a host that wants none says none.
- Good, because the priority between formats stops being a question:
  a module carries one, and no engine has to rank them.
- Bad, because the same program assembled for two consumers is two modules,
  and the option has to travel with the bytes that were built under it.

## Links

- Values as words: [0018-values-as-words.md]
- WASM backend, whose tables this record keeps for debuggers: [0020-wasm-backend.md]
- Translation units, whose modules a run is: [0021-translation-units.md]
- WASM LIR, whose origins both formats are written from: [0022-wasm-lir.md]
- The record this one replaces, with what each consumer of DWARF reads: [0023-debug-information.md]
- The record this one replaces, with the convention of the map: [0024-browser-debug-information.md]
- The driver whose options the format joins: [0008-compiler-driver.md]
- Pass contract: [0009-pass-contract.md]
- DWARF for WebAssembly: <https://yurydelendik.github.io/webassembly-dwarf/>
- DevTools, which ranks DWARF above a source map:
  <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/core/sdk/DebuggerModel.ts>
- The C/C++ extension, which claims a module that carries DWARF:
  <https://github.com/ChromeDevTools/devtools-frontend/blob/main/extensions/cxx_debugging/src/DevToolsPluginHost.ts>
- DevTools, which asks the plugin before the map:
  <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/DebuggerWorkspaceBinding.ts>
- DevTools, which loads the sources of a plugin from the host:
  <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/DebuggerLanguagePlugins.ts>
- Wasmtime native debugging:
  <https://docs.wasmtime.dev/examples-debugging-native-debugger.html>
- Emscripten, whose `wasm-sourcemap.py` pins the convention of the map:
  <https://github.com/emscripten-core/emscripten/blob/main/tools/wasm-sourcemap.py>

[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
[0021-translation-units.md]: 0021-translation-units.md
[0022-wasm-lir.md]: 0022-wasm-lir.md
[0023-debug-information.md]: 0023-debug-information.md
[0024-browser-debug-information.md]: 0024-browser-debug-information.md
[codegen]: ../../crates/mlkc-codegen-wasm
[dwarf-wasm]: https://yurydelendik.github.io/webassembly-dwarf/
[debugger-model]: https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/core/sdk/DebuggerModel.ts
[devtools-plugin-host]: https://github.com/ChromeDevTools/devtools-frontend/blob/main/extensions/cxx_debugging/src/DevToolsPluginHost.ts
[debugger-workspace-binding]: https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/DebuggerWorkspaceBinding.ts
[debugger-language-plugins]: https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/DebuggerLanguagePlugins.ts
