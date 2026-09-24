#!/usr/bin/env node
/**
 * Checks the editor in a browser.
 *
 * The wasm boundary is the one part of the editor no other kind of test can reach:
 * the module has to be fetched, instantiated and asked to compile a buffer
 * by a real JavaScript host, and the page has to show what it got back.
 * This drives the built site in a headless browser over CDP and reads the page afterwards,
 * so what it asserts is what a person would see.
 *
 *     node scripts/browser-check.mjs [--base URL] [--cdp URL] [--keep]
 *
 * The check serves `web/build`, so the site has to be built first — `pnpm check:browser`
 * does that. `vite preview` takes a free port, and `obscura serve` is started when no
 * browser answers; both are taken down on the way out unless `--keep` asks otherwise.
 * Pass `--base` to check a site that already runs — the dev server, say.
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
 * The page is asked one question at a time, and the browser runs it between questions.
 *
 * Nothing here waits: a question that awaited something would hold the page still
 * for as long as it waited, because the browser runs the page and the answer on one thread.
 */
const HELPERS = `
	const text = (element) => element?.textContent ?? '';
	const inspector = () => document.querySelector('[data-panel="inspector"]');
	const show = (which) => document.querySelector('[data-tab=' + which + ']').click();
	const diagnostics = () => [...inspector().querySelectorAll('li')];
`;

/** What is typed into the editor to make it say something: a module that is not one. */
const BROKEN = "fun main(): Unit =\n    let x = 1\n";

/**
 * What is typed to leave the parser with a node it has no room for: a name among declarations.
 *
 * The parser keeps what it cannot place as a node of the tree — `BogusDecl` here — which the
 * ast has to show the way it shows any other node.
 */
const STRAY = "fun main(): Unit =\n    x\nabc\n";

/** The questions themselves, each answered by one round trip. */
const STEPS = {
    state: `return JSON.stringify({
    		panels: document.querySelectorAll('[data-panel]').length,
    		file: text(document.querySelector('[data-panel=editor] .tab.active .pick'))
    	})`,

    // Showing a view and reading it are two questions: the page renders between them.
    showCst: `show('cst'); return true`,

    cst: `return JSON.stringify({
    		nodes: inspector().querySelectorAll('[data-kind]').length,
    		root: inspector().querySelector('[data-kind=MODULE_ROOT]') !== null
    	})`,

    // What a pointer on a row of a tree does in the editor. A row says what it stands for in
    // its own words — a name of the module is a token, and a row that says so — and the
    // editor is asked to have marked exactly that.
    hoverTree: `const row = document.querySelector('[data-panel=inspector] [data-kind=IDENT] .row');
    	row.dispatchEvent(new MouseEvent('mouseenter'));
    	return JSON.stringify({ says: row.querySelector('.text').textContent })`,

    hovered: `return JSON.stringify({
    		count: document.querySelectorAll('.cm-content .cm-hovered').length,
    		marked: text(document.querySelector('.cm-content .cm-hovered'))
    	})`,

    showDiagnostics: `show('diagnostics'); return true`,

    showAst: `show('ast'); return true`,

    ast: `return JSON.stringify({
    		root: inspector().textContent.includes('ModuleRoot'),
    		decl: inspector().textContent.includes('FunDecl')
    	})`,

    // A node of the typed tree says nothing about where it is: it covers what the tokens under
    // it cover, which is what a pointer on its row is asked to mark.
    hoverAst: `const row = [...document.querySelectorAll('[data-panel=inspector] .row')]
    		.find((it) => text(it.querySelector('.kind')) === 'FunDecl');
    	row.dispatchEvent(new MouseEvent('mouseenter'));
    	return true`,

    astHovered: `const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
    	return JSON.stringify({
    		count: parts.length,
    		marked: parts.map((it) => it.textContent).join('')
    	})`,

    clean: `return JSON.stringify({ diagnostics: diagnostics().length })`,

    showProgram: `document.querySelector('[data-console=program]').click(); return true`,

    program: `return JSON.stringify({
    		empty: document.querySelector('[data-panel=console] .empty') !== null
    	})`,

    showCompiler: `document.querySelector('[data-console=compiler]').click(); return true`,

    // A handle answers the arrow keys, which is how a person sizes a panel without a pointer.
    widen: `document.querySelector('[aria-label="Size of the files panel"]')
    		.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    	return true`,

    width: `return JSON.stringify({
    		files: Math.round(document.querySelector('[data-panel=files]').getBoundingClientRect().width)
    	})`,

    // A buffer is made by typing a path, which is also what makes the directories in it.
    open: `document.querySelector('[data-panel=files] .add').click(); return true`,

    typePath: `const field = document.querySelector('[data-panel=files] input');
    	field.value = 'lib/sample.mlk';
    	field.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    submitPath: `document.querySelector('[data-panel=files] form').requestSubmit(); return true`,

    made: `return JSON.stringify({
    		file: document.querySelector('[data-file="/lib/sample.mlk"]') !== null,
    		open: document.querySelectorAll('[data-panel=editor] [data-open]').length
    	})`,

    // The editor is a CodeMirror view, and CodeMirror reads what a browser puts into it.
    // There are no keystrokes here, so the text goes in through the DOM instead,
    // which is where the editor watches for what a person typed.
    type: `const content = document.querySelector('.cm-content');
	content.focus();
	content.textContent = ${JSON.stringify(BROKEN)};
	content.dispatchEvent(new Event('input', { bubbles: true }));
	return true`,

    // The same, with a mistake the parser cannot place anywhere: it keeps what it found as a
    // node of the tree, which the ast has to show the way it shows any other node.
    typeStray: `const content = document.querySelector('.cm-content');
	content.focus();
	content.textContent = ${JSON.stringify(STRAY)};
	content.dispatchEvent(new Event('input', { bubbles: true }));
	return true`,

    bogus: `return JSON.stringify({
		object: inspector().textContent.includes('[object Object]'),
		shown: inspector().textContent.includes('Bogus')
	})`,

    // A node of the syntax tree inside the typed one covers what its children cover, which is
    // what a pointer on its row asks the editor to mark.
    hoverBogus: `const row = [...document.querySelectorAll('[data-panel=inspector] .row')]
		.find((it) => text(it.querySelector('.kind')) === 'BogusDecl');
	row.dispatchEvent(new MouseEvent('mouseenter'));
	return true`,

    bogusMark: `const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
	return JSON.stringify({ count: parts.length, marked: parts.map((it) => it.textContent).join('') })`,

    closeTab: `document.querySelector('[data-open="/lib/arith.mlk"] .close').click(); return true`,

    closed: `return JSON.stringify({
    		open: document.querySelectorAll('[data-panel=editor] [data-open]').length,
    		listed: document.querySelector('[data-file="/lib/arith.mlk"]') !== null
    	})`,

    askDrop: `document.querySelector('[data-file="/lib/sample.mlk"] .drop').click(); return true`,

    confirmDrop: `document.querySelector('[data-panel=files] .yes').click(); return true`,

    dropped: `return JSON.stringify({
    		file: document.querySelector('[data-file="/lib/sample.mlk"]') === null
    	})`,

    seen: `return JSON.stringify(diagnostics().map((element) => ({
    		classes: [...element.classList],
    		code: text(element.querySelector('.code')),
    		message: text(element.querySelector('.message')),
    		number: text(element.querySelector('.number')),
    		caret: text(element.querySelector('.caret')),
    		whole: text(element)
    	})))`,

    // What the editor itself paints and marks, which the panel does not say.
    //
    // The colours are read off the view rather than off the classes it happens to use,
    // because a colour is what a person sees: a keyword and a number against a name,
    // which the language leaves the colour of text (see `src/lib/highlight.ts`).
    painted: `const spans = [...document.querySelectorAll('.cm-content .cm-line span')];
    	const colour = (text) => {
    		const span = spans.find((it) => it.textContent === text);
    		return span ? getComputedStyle(span).color : '';
    	};
    	return JSON.stringify({
    		keyword: colour('fun'),
    		number: colour('42'),
    		type: colour('Unit'),
    		name: colour('main'),
    		marks: document.querySelectorAll('.cm-content [class*=cm-lintRange], .cm-content [class*=cm-lintPoint]').length
    	})`,
};

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
                { once: true },
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
    if (!options.base) base = `http://127.0.0.1:${await freePort(4173)}`;

    await ensureStaticServer();
    await ensureBrowser();

    const { connection, session } = await openPage();
    const problems = [];
    const warnings = [];
    const asked = [];

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

        /** What the page asked for: a page that does not run is usually a request. */
        if (message.method === "Network.responseReceived")
            asked.push(
                `${message.params.response.url} ${message.params.response.status}`,
            );
    });

    await connection.send("Runtime.enable", {}, session);
    await connection.send("Log.enable", {}, session);
    await connection.send("Page.enable", {}, session);
    await connection.send("Network.enable", {}, session);

    const loaded = deferred();
    connection.on((message) => {
        if (message.method === "Page.loadEventFired") loaded.accept();
    });

    await connection.send("Page.navigate", { url: `${base}/` }, session);
    await Promise.race([
        loaded.promise,
        timeout(20000, `the page at ${base} never finished loading`),
    ]);

    /** One question, and the answer the page gave. */
    const ask = async (body) => {
        const result = await connection.send(
            "Runtime.evaluate",
            {
                expression: `(() => { ${HELPERS} ${body} })()`,
                returnByValue: true,
            },
            session,
        );

        if (result.exceptionDetails)
            throw new Error(describe(result.exceptionDetails));

        return result.result.value;
    };

    // The page hydrates after it has loaded, which is also when the kernel is fetched.
    const hydrated = await waitFor(
        async () => {
            const state = JSON.parse(await ask(STEPS.state));

            return state.panels > 0;
        },
        { timeout: 30000 },
    );

    const state = JSON.parse(await ask(STEPS.state));

    await ask(STEPS.showCst);
    const cst = JSON.parse(await ask(STEPS.cst));

    // The editor paints the buffer in front, before anything is typed into it.
    const painted = JSON.parse(await ask(STEPS.painted));

    // And a pointer on a row of the tree marks the code that row stands for.
    const says = JSON.parse(await ask(STEPS.hoverTree));
    const hover = { ...says, ...JSON.parse(await ask(STEPS.hovered)) };

    await ask(STEPS.showDiagnostics);
    const clean = JSON.parse(await ask(STEPS.clean));

    await ask(STEPS.showProgram);
    const program = JSON.parse(await ask(STEPS.program));
    await ask(STEPS.showCompiler);

    await ask(STEPS.widen);
    const width = JSON.parse(await ask(STEPS.width));

    await ask(STEPS.showAst);
    const ast = JSON.parse(await ask(STEPS.ast));

    // A node of the typed tree covers the tokens under it, and a pointer on its row marks as much.
    await ask(STEPS.hoverAst);
    const astHover = JSON.parse(await ask(STEPS.astHovered));

    await ask(STEPS.open);
    await ask(STEPS.typePath);
    await ask(STEPS.submitPath);
    const made = JSON.parse(await ask(STEPS.made));

    // Type into the buffer that was just made, the way a person would.
    await ask(STEPS.type);
    await sleep(300);

    await ask(STEPS.showDiagnostics);
    const broken = JSON.parse(await ask(STEPS.seen));

    const marks = JSON.parse(await ask(STEPS.painted));

    // A mistake the parser cannot place is a node of the tree, and the ast shows it as one.
    await ask(STEPS.typeStray);
    await sleep(300);
    await ask(STEPS.showAst);
    const bogus = JSON.parse(await ask(STEPS.bogus));

    await ask(STEPS.hoverBogus);
    const bogusMark = JSON.parse(await ask(STEPS.bogusMark));

    await ask(STEPS.closeTab);
    const closed = JSON.parse(await ask(STEPS.closed));

    await ask(STEPS.askDrop);
    await ask(STEPS.confirmDrop);
    const dropped = JSON.parse(await ask(STEPS.dropped));

    return report(
        {
            status: state.file,
            hydrated,
            clean: { root: cst.root, diagnostics: clean.diagnostics },
            ast,
            program,
            width,
            made,
            closed,
            dropped,
            tree: cst.nodes,
            painted,
            hover,
            astHover,
            marks,
            broken,
            bogus,
            bogusMark,
        },
        problems,
        warnings,
        asked,
    );
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
function report(page, problems, warnings, asked) {
    // A page that never came up fails everything below it in the same way,
    // and saying so once is the whole of what can be said about it.
    if (!page.hydrated) {
        console.log(`the page at ${base} did not come up`);
        console.log(`what it answered: ${page.status || "(nothing)"}`);
        console.log(
            `what it asked for:\n  ${[...new Set(asked)].join("\n  ")}`,
        );
        console.log(`console: ${[...problems, ...warnings].join(" | ")}`);

        return false;
    }

    const diagnostic = page.broken?.[0] ?? {};

    const checks = [
        ["the page hydrated", page.hydrated],
        ["the cst of a clean buffer is a module", page.clean.root],
        ["the tree is more than its root", page.tree > 5],
        ["the ast names its root", page.ast.root],
        ["the ast names a declaration", page.ast.decl],
        ["the editor paints a keyword", page.painted.keyword !== ""],
        [
            "the editor paints code in more than one colour",
            page.painted.keyword !== page.painted.number &&
                page.painted.number !== page.painted.type &&
                page.painted.keyword !== page.painted.name,
        ],
        [
            "the editor leaves a name the colour of text",
            page.painted.name === "",
        ],
        ["a row of a tree marks code in the editor", page.hover.count === 1],
        [
            "the mark is the code the row says it stands for",
            JSON.parse(page.hover.says) === page.hover.marked,
        ],
        ["a row of the ast marks code in the editor", page.astHover.count > 0],
        [
            "a node of the ast marks what it holds",
            // What a node covers is written over as many lines as it takes, and a mark is a
            // piece of a line: the pieces together are what the node covers.
            page.astHover.marked.includes(page.hover.marked.trim()) &&
                page.astHover.marked.trim().length >
                    page.hover.marked.trim().length,
        ],
        ["the editor marks what it reported", page.marks.marks > 0],
        ["a buffer can be made at a path", page.made.file],
        ["a buffer opens as a tab", page.made.open === 3],
        [
            "a tab closes without the file",
            page.closed.open === 2 && page.closed.listed,
        ],
        ["a buffer can be dropped", page.dropped.file],
        ["the console has a program tab", page.program.empty],
        ["a panel can be sized", page.width.files > 220],
        ["a clean buffer reports nothing", page.clean.diagnostics === 0],
        [
            "a node the grammar has no room for is shown as a node",
            page.bogus.shown,
        ],
        [
            "nothing the ast shows reads as an object",
            !page.bogus.object,
        ],
        [
            "a node the grammar has no room for marks what it holds",
            page.bogusMark.count > 0 && page.bogusMark.marked.includes("abc"),
        ],
        [
            "a broken buffer reports a diagnostic",
            (page.broken?.length ?? 0) > 0,
        ],
        [
            "the diagnostic is an error",
            (diagnostic.classes ?? []).includes("error"),
        ],
        ["the diagnostic has a code", /^E\d{4}$/.test(diagnostic.code ?? "")],
        [
            "the diagnostic shows the line it is about",
            /^\d+$/.test(diagnostic.number ?? "") &&
                (diagnostic.caret ?? "").includes("^"),
        ],
        ["the page said nothing it should not have", problems.length === 0],
    ];

    const held = checks.every(([, it]) => it);

    console.log(`the page at ${base} shows: ${page.status || "(nothing)"}`);
    console.log(`the cst holds ${page.tree} elements`);
    console.log(
        `the editor paints a keyword ${page.painted.keyword}, a number ${page.painted.number}, a type ${page.painted.type}`,
    );
    console.log(
        `the row that says ${page.hover.says} marks ${page.hover.marked || "nothing"} in the editor`,
    );
    console.log(
        `a row of the ast marks ${JSON.stringify(page.astHover.marked.slice(0, 40))}`,
    );
    console.log(
        `a node the grammar has no room for reads as ${page.bogus.shown ? "a node" : "nothing"}`,
    );
    console.log(
        `its elements mark ${JSON.stringify(page.bogusMark.marked.slice(0, 24))}`,
    );
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
            `what it asked for:\n  ${[...new Set(asked)].join("\n  ")}`,
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

    if (!response.ok)
        throw new Error(
            `${base} serves another build: ${asset} answers ${response.status}`,
        );
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
        await sleep(interval);
    }

    return false;
}

function sleep(ms) {
    return new Promise((accept) => setTimeout(accept, ms));
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
