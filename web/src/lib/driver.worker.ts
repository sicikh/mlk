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

import type { DriverRequest, DriverResponse } from "./driver";

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
        case "diagnostics":
            return driver.diagnostics(request.path);
        case "stats":
            return driver.stats();
    }
}
