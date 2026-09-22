#!/usr/bin/env node
/**
 * Checks the editor in a browser.
 *
 * The wasm boundary is the one part of the editor no other kind of test can reach:
 * the module has to be fetched, instantiated and asked to compile a buffer
 * by a real JavaScript host, and the page has to show what it got back ([ADR-0008]).
 * This drives the built site in a headless browser over CDP and reads the page afterwards,
 * so what it asserts is what a person would see.
 *
 *     node scripts/browser-check.mjs [--base URL] [--cdp URL] [--keep]
 *
 * The check serves `web/build`, so the site has to be built first — `pnpm check:browser`
 * does that. `vite preview` takes a free port, and `obscura serve` is started when no
 * browser answers; both are taken down on the way out unless `--keep` asks otherwise.
 * Pass `--base` to check a site that already runs — the dev server, say.
 *
 * [ADR-0008]: https://github.com/sicikh/mlk/blob/main/docs/adr/0008-compiler-driver.md
 */
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { createServer } from "node:net";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const options = parseArguments(process.argv.slice(2));
const cdp = options.cdp ?? "http://127.0.0.1:9222";

/** Where the site is served: the one asked for, or one of our own. */
let base = options.base ?? "http://127.0.0.1:4173";

/** What the check brought up itself, and therefore has to take down again. */
const started = [];

/**
 * What the page is asked to tell about itself.
 *
 * It waits for the driver on its own and types into the source pane,
 * because both are things a person does and neither is visible from the outside.
 */
const PROBE = `(async () => {
	const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
	const pane = (title) => [...document.querySelectorAll('.pane')]
		.find((element) => element.querySelector('h2')?.textContent?.trim().startsWith(title));
	const text = (element) => element?.textContent ?? '';
	const status = () => text(document.querySelector('.status'));

	const settled = () => {
		const value = status();
		return value !== '' && !value.startsWith('loading');
	};

	const deadline = Date.now() + 20000;
	while (!settled() && Date.now() < deadline) await sleep(100);

	const source = pane('Source')?.querySelector('textarea');
	const diagnostics = () => [...(pane('Diagnostics')?.querySelectorAll('li') ?? [])];
	const report = {
		status: status(),
		hydrated: Boolean(source),
		clean: {
			cst: text(pane('CST')?.querySelector('pre')),
			ast: text(pane('AST')?.querySelector('pre')),
			diagnostics: diagnostics().length
		},
		broken: null,
		debug: {
			readyState: document.readyState,
			bodyBytes: document.body.innerHTML.length,
			panes: document.querySelectorAll('.pane').length,
			resources: performance.getEntriesByType('resource')
				.map((entry) => entry.name.split('/').pop() + ' ' + (entry.responseStatus ?? '?') + ' ' + Math.round(entry.duration) + 'ms')
				.slice(-10)
		}
	};

	if (source) {
		source.value = 'fun main(): Unit =\\n    let x = 1\\n';
		source.dispatchEvent(new Event('input', { bubbles: true }));
		await sleep(200);

		report.broken = diagnostics().map((element) => ({
			classes: [...element.classList],
			code: text(element.querySelector('.code')),
			label: text(element.querySelector('.label')),
			whole: text(element)
		}));
	}

	return report;
})()`;

/** A CDP connection: commands are answered by id, events go to whoever listens. */
class Connection {
    #socket;
    #next = 1;
    #pending = new Map();
    #listeners = new Set();

    static async open(url) {
        const socket = new WebSocket(url);

        await new Promise((accept, reject) => {
            socket.addEventListener("open", accept, { once: true });
            socket.addEventListener(
                "error",
                () => reject(new Error(`cannot connect to ${url}`)),
                {
                    once: true,
                },
            );
        });

        return new Connection(socket);
    }

    constructor(socket) {
        this.#socket = socket;
        socket.addEventListener("message", (event) =>
            this.#receive(String(event.data)),
        );
    }

    send(method, params = {}, sessionId) {
        const id = this.#next++;

        return new Promise((accept, reject) => {
            this.#pending.set(id, { accept, reject });
            this.#socket.send(
                JSON.stringify({ id, method, params, sessionId }),
            );
        });
    }

    on(listener) {
        this.#listeners.add(listener);
    }

    #receive(raw) {
        const message = JSON.parse(raw);

        if (message.id !== undefined) {
            const pending = this.#pending.get(message.id);
            this.#pending.delete(message.id);

            if (message.error)
                pending?.reject(new Error(`${message.error.message}`));
            else pending?.accept(message.result);

            return;
        }

        for (const listener of this.#listeners) listener(message);
    }
}

async function main() {
    // A probe that does not parse is a bug in the checker, and it would look like a broken page.
    new Function(PROBE);

    if (!options.base) base = `http://127.0.0.1:${await freePort(4173)}`;

    await ensureStaticServer();
    await ensureBrowser();

    const { connection, session } = await openPage();
    const problems = [];
    const warnings = [];

    /** Everything the page said that it should not have. */
    connection.on((message) => {
        if (message.method === "Runtime.exceptionThrown") {
            problems.push(describe(message.params.exceptionDetails));
        }

        if (message.method === "Runtime.consoleAPICalled") {
            const said = (message.params.args ?? [])
                .map((argument) => argument.value ?? argument.description ?? "")
                .join(" ");

            if (message.params.type === "error")
                problems.push(`console.error: ${said}`);
            else if (message.params.type === "warning") warnings.push(said);
        }

        if (message.method === "Log.entryAdded") {
            const entry = message.params.entry;

            if (entry.level === "error")
                problems.push(`${entry.source}: ${entry.text}`);
            else if (entry.level === "warning") warnings.push(entry.text);
        }
    });

    await connection.send("Runtime.enable", {}, session);
    await connection.send("Log.enable", {}, session);
    await connection.send("Page.enable", {}, session);

    const loaded = deferred();
    connection.on((message) => {
        if (message.method === "Page.loadEventFired") loaded.accept();
    });

    await connection.send("Page.navigate", { url: `${base}/` }, session);
    await Promise.race([
        loaded.promise,
        timeout(20000, `the page at ${base} never finished loading`),
    ]);

    const result = await connection.send(
        "Runtime.evaluate",
        { expression: PROBE, awaitPromise: true, returnByValue: true },
        session,
    );

    if (result.exceptionDetails)
        throw new Error(describe(result.exceptionDetails));

    return report(result.result.value, problems, warnings);
}

/**
 * The page to drive: the browser names the session of a new page on the connection,
 * and everything said about that page carries the session it names.
 */
async function openPage() {
    const version = await json(`${cdp}/json/version`);
    const connection = await Connection.open(version.webSocketDebuggerUrl);

    const attached = deferred();
    connection.on((message) => {
        if (message.method === "Target.attachedToTarget")
            attached.accept(message.params.sessionId);
    });

    await connection.send("Target.createTarget", { url: "about:blank" });
    const session = await Promise.race([
        attached.promise,
        timeout(10000, "the browser attached no page"),
    ]);

    return { connection, session };
}

/** What the run found, said the way a person reads it. */
function report(page, problems, warnings) {
    const diagnostic = page.broken?.[0] ?? {};

    const checks = [
        ["the page hydrated", page.hydrated],
        ["the driver loaded", page.status === "the driver is loaded"],
        [
            "the cst of a clean buffer is a module",
            page.clean.cst.includes("MODULE_ROOT@"),
        ],
        [
            "the ast of a clean buffer is a module root",
            page.clean.ast.includes("ModuleRoot"),
        ],
        ["a clean buffer reports nothing", page.clean.diagnostics === 0],
        [
            "a broken buffer reports a diagnostic",
            (page.broken?.length ?? 0) > 0,
        ],
        [
            "the diagnostic is an error",
            (diagnostic.classes ?? []).includes("error"),
        ],
        [
            "the diagnostic says where it is",
            /\d+:\d+/.test(diagnostic.label ?? ""),
        ],
        ["the page said nothing it should not have", problems.length === 0],
    ];

    const held = checks.every(([, it]) => it);

    console.log(`the page at ${base} says: ${page.status || "(nothing)"}`);
    console.log(
        `a broken buffer gives ${page.broken?.length ?? 0} diagnostic(s)`,
    );

    if (diagnostic.whole)
        console.log(`  ${diagnostic.whole.trim().replace(/\s+/g, " ")}`);

    for (const [what, it] of checks)
        console.log(`${it ? "ok  " : "FAIL"} ${what}`);

    if (warnings.length > 0)
        console.log(`warnings:\n  ${warnings.join("\n  ")}`);

    if (problems.length > 0)
        console.log(`problems:\n  ${problems.join("\n  ")}`);

    if (!held)
        console.log(
            `what the page was: ${JSON.stringify(page.debug, null, 2)}`,
        );

    return held;
}

/** Starts a process of its own group, so that taking it down takes down what it spawned. */
function start(name, command, args) {
    const child = spawn(command, args, { cwd: root, detached: true });
    const said = [];

    child.stdout.on("data", (chunk) => said.push(String(chunk)));
    child.stderr.on("data", (chunk) => said.push(String(chunk)));

    started.push({ name, child, said });

    return child;
}

async function ensureStaticServer() {
    if (options.base) {
        if (!(await reachable(`${base}/`)))
            throw new Error(`nothing answers at ${base}`);

        return;
    }

    if (await reachable(`${base}/`)) {
        // Something else already holds the port, and a preview server started before the
        // last build hands out a site whose scripts are gone. Refuse to check against it.
        throw new Error(
            `${base} is taken: pass --base if that server serves the site you mean`,
        );
    }

    const port = new URL(base).port || "4173";
    start("vite preview", "pnpm", [
        "exec",
        "vite",
        "preview",
        "--port",
        port,
        "--strictPort",
        "--host",
        "127.0.0.1",
    ]);

    if (!(await waitFor(() => reachable(`${base}/`)))) {
        throw new Error(`no server at ${base}: was the site built?`);
    }

    await ensureServedSite();
}

/**
 * Whether the server hands out the site the build just wrote.
 *
 * A preview server started before a rebuild serves the shell of one build with the assets
 * of another, which looks like a page that loads and does nothing.
 */
async function ensureServedSite() {
    const index = await readFile(
        join(root, "build", "index.html"),
        "utf8",
    ).catch(() => {
        throw new Error("no built site in web/build: run `pnpm build` first");
    });

    const asset = index.match(/_app\/immutable\/[^"']+\.js/)?.[0];

    if (!asset) throw new Error("the built site names no script to load");

    const response = await fetch(`${base}/${asset}`);

    if (!response.ok) {
        throw new Error(
            `${base} serves another build: ${asset} answers ${response.status}`,
        );
    }
}

async function ensureBrowser() {
    if (await reachable(`${cdp}/json/version`)) return;

    const port = new URL(cdp).port || "9222";
    start("obscura serve", process.env.OBSCURA ?? "obscura", [
        "serve",
        "-p",
        port,
        "--allow-private-network",
    ]);

    if (!(await waitFor(() => reachable(`${cdp}/json/version`)))) {
        throw new Error(`no browser at ${cdp}`);
    }
}

/** Whether something answers at a URL, which is all this needs to know about it. */
async function reachable(url) {
    try {
        const response = await fetch(url, {
            signal: AbortSignal.timeout(1000),
        });
        return response.ok;
    } catch {
        return false;
    }
}

/**
 * A port nothing is listening on, from the one the check prefers upwards.
 *
 * A server left over from an older run may hold the usual one,
 * and a preview started before the last build serves a site whose scripts are gone.
 */
async function freePort(preferred) {
    for (let port = preferred; port < preferred + 20; port++) {
        if (await available(port)) return port;
    }

    throw new Error(`every port from ${preferred} upwards is taken`);
}

function available(port) {
    return new Promise((accept) => {
        const probe = createServer();

        probe.once("error", () => accept(false));
        probe.once("listening", () => probe.close(() => accept(true)));
        probe.listen(port, "127.0.0.1");
    });
}

async function waitFor(check, { timeout: limit = 30000, interval = 250 } = {}) {
    const deadline = Date.now() + limit;

    while (Date.now() < deadline) {
        if (await check()) return true;
        await new Promise((accept) => setTimeout(accept, interval));
    }

    return false;
}

async function json(url) {
    return await (await fetch(url)).json();
}

function timeout(after, message) {
    return new Promise((_, reject) =>
        setTimeout(() => reject(new Error(message)), after),
    );
}

function deferred() {
    let accept;
    const promise = new Promise((resolve) => {
        accept = resolve;
    });

    return { promise, accept };
}

function describe(details) {
    return (
        details?.exception?.description ??
        details?.text ??
        JSON.stringify(details)
    );
}

function parseArguments(argv) {
    const options = {};

    for (let index = 0; index < argv.length; index++) {
        const argument = argv[index];
        const [name, value] = argument.split("=");

        if (name === "--help" || name === "-h") {
            console.log(
                "usage: node scripts/browser-check.mjs [--base URL] [--cdp URL] [--keep]",
            );
            process.exit(0);
        }

        if (name === "--keep") options.keep = true;
        else if (name === "--base") options.base = value ?? argv[++index];
        else if (name === "--cdp") options.cdp = value ?? argv[++index];
        else throw new Error(`unknown argument ${argument}`);
    }

    return options;
}

let ok = false;

try {
    ok = await main();
} catch (error) {
    console.error(`the check could not run: ${error.message}`);
    ok = false;
} finally {
    if (!options.keep) {
        for (const { name, child, said } of started) {
            // Only the tail of a process's output is worth reading when something failed.
            const lines = said.join("").trim().split("\n").slice(-10);

            if (!ok && said.length > 0) {
                console.error(`${name} said:\n${lines.join("\n")}`);
            }

            // The group, not the process: what a preview server spawned has to go too.
            try {
                process.kill(-child.pid, "SIGTERM");
            } catch {
                child.kill();
            }
        }
    }
}

console.log(ok ? "the browser check passed" : "the browser check failed");
process.exit(ok ? 0 : 1);
