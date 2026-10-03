/**
 * The driver, on a thread of its own.
 *
 * The compiler is a caller, not a callee: it runs when a host pulls a value out of it, and a
 * pull of a tree is real work. Here that work happens off the thread the page draws on, so
 * typing stays typing while the pipeline runs behind it, and a page that opens a big buffer
 * does not wait for it to be lowered before it can be seen.
 *
 * What crosses the boundary is `driver.ts`'s protocol: the text of buffers in, one question
 * per value out, each answered under the id it was asked with. The worker answers in the order
 * it is spoken to, which is what lets a push be a notification --- the pulls after it see the
 * text without anything having to be acknowledged.
 *
 * Nothing here is awaited at the top of the module: the worker the build hands over is one
 * script, which a host without module syntax can run as a classic one, and a top-level await is
 * not something such a script has. The wasm module is instantiated in `load`, and questions are
 * answered from the moment it is: `driver.ts` sends nothing before this worker says it is up,
 * so nothing is asked of a driver that does not exist yet.
 */

import init, { WasmDriver } from "@mlk/wasm";

import type {
    DriverRequest,
    DriverResponse,
    Program,
    Run,
    RunEntry,
} from "./driver";

/**
 * The scope of this worker.
 *
 * The page's types describe a `window`, where `postMessage` takes a target origin; a worker
 * has no origin to name, and the DOM types say nothing about a scope that is a worker's own.
 */
const scope = self as unknown as {
    onmessage: ((event: MessageEvent<DriverRequest>) => void) | null;
    postMessage(message: DriverResponse): void;
};

load();

/**
 * Instantiates the wasm module, and starts answering with the driver over it.
 *
 * A module that does not load is not a worker that says nothing forever: what went wrong goes
 * to the page, which is waiting to hear that this one is up.
 */
function load() {
    init().then(
        () => hold(new WasmDriver()),
        (error) =>
            scope.postMessage({
                kind: "fatal",
                message: `the wasm module did not load: ${String(error)}`,
            }),
    );
}

/** Answers what the page asks, from the moment there is a driver to answer with. */
function hold(driver: WasmDriver) {
    scope.onmessage = (event) => receive(driver, event.data);
    scope.postMessage({ kind: "ready" });
}

/** One message from the page: a notification to keep, or a question to answer. */
function receive(driver: WasmDriver, request: DriverRequest) {
    // A push is a notification: the driver keeps the text, and every pull after it reads what
    // the driver holds, because a worker takes its messages in the order they were sent.
    if (request.kind === "setText") {
        driver.setText(request.path, request.text);

        return;
    }

    try {
        scope.postMessage({
            kind: "answer",
            id: request.id,
            value: answer(driver, request),
        });
    } catch (error) {
        // A pull the driver refused is an answer too, and a refused one: the question is
        // closed, and the page reads why rather than waiting for a value that is not coming.
        scope.postMessage({
            kind: "error",
            id: request.id,
            message: String(error),
        });
    }
}

/** The value one question asks the driver for. */
function answer(
    driver: WasmDriver,
    request: Exclude<DriverRequest, { kind: "setText" }>,
): unknown {
    switch (request.kind) {
        case "setOptions":
            return driver.setOptions(request.debug, request.opt);
        case "useStd":
            return driver.useStd();
        case "cst":
            return driver.cst(request.path);
        case "ast":
            return driver.ast(request.path);
        case "hir":
            return driver.hir(request.path);
        case "types":
            return driver.types(request.path);
        case "mir":
            return driver.mir(request.path);
        case "mirSsa":
            return driver.mirSsa(request.path);
        case "lir":
            return driver.lir(request.path);
        case "wat":
            return driver.wat(request.path);
        case "run":
            return run(driver.run());
        case "diagnostics":
            return driver.diagnostics(request.path);
        case "stats":
            return driver.stats();
    }
}

/**
 * Runs the program the driver holds, in the WebAssembly of the browser ([ADR-0021]).
 *
 * The manifest is the driver's: this is the host [ADR-0021] describes, and it does what a host
 * does. The module of the host functions is instantiated first over the numbers a browser has;
 * every module of the program is instantiated in the order it is given, with the exports of the
 * modules before it handed to its imports; and the entry point is called. What the program
 * printed is what the host functions were handed, which is why they are the only place a word
 * of the language crosses this boundary.
 *
 * [adr-0021]: ../../docs/adr/0021-translation-units.md
 */
function run(program: Program): Run {
    const printed: string[] = [];
    const result: Run = {
        entry: program.entry,
        printed,
        error: null,
        diagnostics: program.diagnostics,
    };

    // What the compiler cannot do for the program is a run that does not begin, and the reason
    // is the compiler's: a host function the host does not have, or an entry it does not
    // declare.
    if (program.problems.length > 0) {
        result.error = program.problems.join("; ");

        return result;
    }

    if (!program.entry) {
        result.error = "the program declares no `#[entry]`";

        return result;
    }

    try {
        const host = instantiate(program.host, {
            host: {
                "print-int": (value: number) => {
                    printed.push(String(value));

                    return 0;
                },
                "print-bool": (value: number) => {
                    printed.push(value !== 0 ? "true" : "false");

                    return 0;
                },
            },
        });
        const instances = new Map<string, WebAssembly.Instance>();

        for (const module of program.modules) {
            const imports: WebAssembly.Imports = {};

            for (const it of module.imports) {
                const provider = it.external
                    ? host.exports
                    : instances.get(it.module)?.exports;
                const fn = provider?.[it.name];

                if (typeof fn !== "function") {
                    throw new Error(
                        `the program imports \`${it.module}::${it.name}\`, and nothing provides it`,
                    );
                }

                (imports[it.module] ??= {})[it.name] = fn;
            }

            instances.set(module.name, instantiate(module.bytes, imports));
        }

        const main = instance(instances, program.entry);

        // The entry is `() -> Unit`: it takes nothing, and the word it gives back is the unit
        // of the language, which a host reads as nothing at all.
        main();
    } catch (error) {
        result.error = String(error);
    }

    return result;
}

/** The instance of a module of the program, by the canonical name of the module. */
function instance(
    instances: Map<string, WebAssembly.Instance>,
    entry: RunEntry,
): CallableFunction {
    const main = instances.get(entry.module)?.exports[entry.name];

    if (typeof main !== "function") {
        throw new Error(
            `the program begins in \`${entry.module}::${entry.name}\`, and no module exports it`,
        );
    }

    return main as CallableFunction;
}

/**
 * Instantiates a module over an import object.
 *
 * The bytes cross the boundary as an array of numbers — the same bytes a browser compiles —
 * and `Uint8Array` is what `WebAssembly.Module` reads.
 */
function instantiate(
    bytes: Uint8Array | number[],
    imports: WebAssembly.Imports,
): WebAssembly.Instance {
    return new WebAssembly.Instance(
        new WebAssembly.Module(new Uint8Array(bytes)),
        imports,
    );
}
