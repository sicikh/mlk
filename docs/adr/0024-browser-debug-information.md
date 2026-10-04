# Carry a source map, so a browser reads a module's debug information

- Status: superseded by [ADR-0025](0025-debug-information-formats.md)
- Date: 2026-10-04

## Context and Problem Statement

[ADR-0023][0023-debug-information.md] decides what DWARF a module carries,
and names the browser among the consumers.
A browser reads debug information of a script in one of two ways:
through the C/C++ DevTools Support (DWARF) extension,
which parses DWARF and drives the runtime
([ADR-0023][0023-debug-information.md]), or through a source map,
which every browser reads without an extension.
The extension alone cannot be the answer:
it shows lines but not locals,
and it does not decide where DevTools takes the text of the source from.
Sources in DWARF exist --- `DW_LNCT_LLVM_source` is a vendor extension of LLVM ---
but the extension has no channel to hand them to DevTools;
DevTools reads the sources of a script itself,
from what the map says or from the host.

The editor is the host that makes the question concrete.
It compiles a program of buffers and runs it from a static site
(a deployment on GitHub Pages is a target),
where the path of a file is not a URL of anything:
a debug session must open the source from the module itself,
not from a host that serves `/main.mlk`.

## Decision Drivers

- **V8 reads a source map of a module, not DWARF.**
  The engine parses the `sourceMappingURL` custom section of a WASM module
  and passes the URL to the debugger as `Debugger.scriptParsed.sourceMapURL`
  (<https://chromedevtools.github.io/devtools-protocol/tot/Debugger/#event-scriptParsed>);
  DWARF of a browser is the work of the extension, and the map is the work of
  the engine ([wasm-constants], [wasm-debugger-script]).
- **A map can carry its sources.**
  When `sourcesContent` holds the text of a file,
  DevTools builds a content provider from it and fetches nothing
  (<https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/CompilerScriptMapping.ts>).
  A module can therefore travel alone: no host, no path scheme, no request.
- **The convention is Emscripten's.**
  A WASM source map is what its `wasm-sourcemap.py` writes:
  every segment stands on generated line zero,
  and the generated column is the byte offset of the instruction in the module
  (<https://github.com/emscripten-core/emscripten/blob/main/tools/wasm-sourcemap.py>).
  The engine reads that, and a reader of the editor sees the same table.
- **The addresses are already known.**
  The line program is built from the `Origin`s of the bodies
  ([ADR-0023][0023-debug-information.md]):
  a segment of the map is the same origin,
  counted from the first byte of the module rather than from the code section.
- **The map counts lines and columns from zero on the wire.**
  DevTools adds one for the editor
  (<https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/panels/sources/DebuggerPlugin.ts>),
  and the `LineIndex` of a file counts from zero as well,
  so the map is written from the index without the +1 the line program needs.
- **DWARF stays.**
  Wasmtime, `gdb`/`lldb`, and `wasm-tools addr2line` read it
  ([ADR-0023][0023-debug-information.md]);
  the map is one custom section beside the tables, not a replacement.

## Considered Options

- **DWARF only, and leave the browser to the extension** --- lines work
  after installing an extension, and nothing is carried for the engine.
- **Embed the sources in DWARF** (`DW_LNCT_LLVM_source`)
  **and leave the browser to the extension** --- the module carries its text,
  but no browser channel hands a DWARF source to DevTools.
- **A source map as a file beside the module** --- what Emscripten ships,
  and the map is fetched by the URL of the section.
- **A source map inside the module, with `sourcesContent`** ---
  the engine reads it, the host serves nothing, and the module is self-contained.

## Decision Outcome

Chosen option: "A source map inside the module, with `sourcesContent`".

### The section

- The name is `sourceMappingURL`, and the data is the URL of the map as a
  length-prefixed string (LEB128), which is what the engine reads:
  the decoder of V8 reads a length and then a string of that length
  ([module-decoder-impl]).
- The URL is a `data:application/json;charset=utf-8;base64,` URL.
  The map travels with the module, and the section does not name a file
  anyone has to serve --- which is what a deployment on a static site needs.

### The map

- The format is source map version 3
  (<https://tc39.es/source-map/>):
  `version`, `sources`, `sourcesContent`, `names`, and `mappings`.
- One segment per `Origin`, in the order the line program writes its rows,
  and at the same instruction:
  a segment of a function stands at `code_payload + at.content + origin.offset`,
  where `code_payload` is the offset of the contents of the code section
  in the module --- the count of the bodies, after the `id` of the section and
  its size.
- The generated line of every segment is zero,
  and the generated column is that module offset, which is the Emscripten
  convention every engine and the `source-map` package read.
- The original line and column are those of the `LineIndex`, zero-based:
  the map counts as the index does, and a reader adds one.
- The `sources` are the paths the driver reads files under,
  the same names the DWARF line program carries --- `/main.mlk`.
- `names` is empty: the map points at code, and the name of a function
  is what the `name` section says.

### When it is written

The map follows the debug level of [ADR-0023][0023-debug-information.md]:
at `DebugLevel::Lines` and `Full` the module carries it beside the DWARF,
and at `None` it carries neither.
The map is built from the same `Origin`s as the line program,
so a module whose bodies have no line has no map:
a map of nothing points nowhere.

### What each consumer reads

| Consumer                            | `None` | `Lines` / `Full`                        |
| ----------------------------------- | ------ | --------------------------------------- |
| DevTools, without an extension      | names  | source stepping, breakpoints, sources   |
| DevTools, with the DWARF extension  | names  | lines, columns; locals are out of reach |
| Firefox DevTools                    | names  | source stepping, breakpoints, sources   |
| Wasmtime, `gdb`/`lldb`, `addr2line` | names  | DWARF: file, line, column               |

### Positive Consequences

- A program compiled at `Lines` can be debugged in a browser as it is,
  with no extension and nothing served:
  the sources are in the map, and the map is in the module.
- The map and the line program cannot disagree:
  both are a view of the same `Origin`s, written in one pass of the assembler,
  and a test checks that a segment stands where its row does.
- The host stays free of a path scheme:
  the editor, the CLI, and a page on a static site all run the same module.
- The format is read by every engine, so the decision is not about Chrome:
  the map is the intersection of what browsers read.

### Negative Consequences

- The module grows by the text of its sources and the map,
  and a release assembles without either:
  this is another reason the debug information is a level
  ([ADR-0023][0023-debug-information.md]), not a constant.
- A second format has to be kept correct as the backend grows;
  the emitters share the `Origin`s but not the writer, and neither `gimli` nor
  `wasm-encoder` checks our offsets.
- DevTools shows the source only where an instruction stands:
  a line of a body that emitted no instruction has no segment to be seen at,
  and the map cannot say more than the line program does.

## Pros and Cons of the Options

### DWARF only, and leave the browser to the extension

- Good, because it is the whole of what is emitted today, and no second format
  is maintained.
- Bad, because a person must install the extension and work in Chrome.
- Bad, because the extension reads lines but not locals
  ([ADR-0023][0023-debug-information.md]),
  so the sole browser path is the one that shows the least.
- Bad, because DevTools still fetches the sources from the host,
  which a deployment without a source endpoint does not have.

### Embed the sources in DWARF and leave the browser to the extension

- Good, because the module carries its text, and a native debugger reads it too
  (`llvm-dwarfdump` shows `DW_LNCT_LLVM_source`).
- Bad, because the extension has no channel to hand a DWARF source to
  DevTools: DevTools builds the sources of a script from the map or fetches
  them, and the extension cannot register a provider
  ([CompilerScriptMapping.ts][compiler-script-mapping]).
- Bad, because no browser guarantees the vendor extension is read at all.

### A source map as a file beside the module

- Good, because the module stays small, and the map is fetched only by a
  debugger.
- Bad, because the host must serve the map at a URL the section names,
  and the section names it relative to the script:
  a module instantiated from bytes by a worker has no URL to be relative to,
  and a static site has no `/main.mlk` to serve.
- Bad, because the editor runs modules from memory and would need a registry
  of URLs to fake what a file server does.

### A source map inside the module, with `sourcesContent`

- Good, because the module is self-contained: the engine reads the map,
  and DevTools opens the source with no request.
- Good, because a `data:` URL needs no host path, so GitHub Pages and a worker
  that runs bytes are both served.
- Bad, because a module assembled with debug information grows by its sources,
  and the base64 of a `data:` URL costs a third more than the JSON it carries.

## Links

- Values as words: [0018-values-as-words.md]
- WASM backend, which requires debug information of a module: [0020-wasm-backend.md]
- Translation units, whose modules travel alone: [0021-translation-units.md]
- WASM LIR, whose origins the map is written from: [0022-wasm-lir.md]
- Emit debug information, and let a host configure it: [0023-debug-information.md]
- The driver whose options the level joins: [0008-compiler-driver.md]
- Pass contract: [0009-pass-contract.md]
- Source map format: <https://tc39.es/source-map/>
- Emscripten, whose `wasm-sourcemap.py` pins the convention:
  <https://github.com/emscripten-core/emscripten/blob/main/tools/wasm-sourcemap.py>
- CDP `Debugger.scriptParsed`, which carries `sourceMapURL`:
  <https://chromedevtools.github.io/devtools-protocol/tot/Debugger/#event-scriptParsed>
- DevTools, which reads `sourcesContent` before it fetches anything:
  <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/CompilerScriptMapping.ts>
- DevTools, which turns a source map line into a line of the editor:
  <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/panels/sources/DebuggerPlugin.ts>
- Chrome DevTools and its DWARF extension:
  <https://developer.chrome.com/docs/devtools/wasm/>

[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
[0021-translation-units.md]: 0021-translation-units.md
[0022-wasm-lir.md]: 0022-wasm-lir.md
[0023-debug-information.md]: 0023-debug-information.md
[wasm-constants]: https://github.com/v8/v8/blob/main/src/wasm/wasm-constants.h
[wasm-debugger-script]: https://github.com/v8/v8/blob/main/src/inspector/v8-debugger-script.cc
[module-decoder-impl]: https://github.com/v8/v8/blob/main/src/wasm/module-decoder-impl.h
[compiler-script-mapping]: https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/bindings/CompilerScriptMapping.ts
