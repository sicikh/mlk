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
	const panel = () => [...document.querySelectorAll('[data-panel]')]
		.filter((it) => it.getBoundingClientRect().width > 0)
		.map((it) => it.dataset.panel)
		.join(' ');
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

/**
 * What is typed to check the rest of the keywords the language reads out of names:
 * an import renamed at the place it is written, under a declaration the module is known by.
 * A word the grammar knows is painted the colour of every other keyword, so `pub`, `use`
 * and `as` are asked to be the colour of `fun` (see `src/lib/highlight.ts`).
 */
const VISIBLE =
    "use project::data::core as data\n\npub fun main(): Unit =\n    data.start()\n";

/**
 * A buffer long enough that its end is not on the screen.
 *
 * A pick of a row of a tree is a request to be taken to the place the row stands for, and
 * a buffer whose end is already in view has nowhere to be taken: this one has.
 */
const LONG =
    "fun main(): Unit =\n" +
    Array.from({ length: 80 }, (_, it) => `    let x${it} = ${it} in`).join(
        "\n",
    ) +
    "\n    x0\n";

/**
 * What is typed into a file of the standard library, which is not a file a person may write.
 *
 * The library is the compiler's: the editor shows a file of it as it shows a buffer, and a
 * state that is read-only takes no text from it.
 */
const WRITTEN = "fun main(): Unit =\n    x\n";

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
    // editor is asked to have marked exactly that. The row taken is a name inside a declaration
    // and not the first name of the module: a module may open with an import, and what an
    // import names is not written inside the declaration the ast is asked about below.
    hoverTree: `const row = document.querySelector('[data-panel=inspector] [data-kind=FUN_DECL] [data-kind=IDENT] .row');
    	row.dispatchEvent(new MouseEvent('mouseenter'));
    	return JSON.stringify({ says: row.querySelector('.text').textContent })`,

    hovered: `return JSON.stringify({
    		count: document.querySelectorAll('.cm-content .cm-hovered').length,
    		marked: text(document.querySelector('.cm-content .cm-hovered'))
    	})`,

    showDiagnostics: `show('diagnostics'); return true`,

    showAst: `show('ast'); return true`,

    hir: `const lines = [...inspector().querySelectorAll('[data-kind]')];
		return JSON.stringify({
			// A line holds what it says and the marks around it: what a person reads is the words.
			module: text(lines.find((it) => it.dataset.kind === 'module')).trim(),
			item: lines.some((it) => it.dataset.kind === 'item' && text(it).includes('fun main')),
			body: inspector().textContent.includes('BODY fun main in module #0'),
			pat: lines.some((it) => it.dataset.kind === 'pat' && text(it).includes('bind x')),
			path: lines.some((it) => it.dataset.kind === 'path' && text(it).includes('println-int'))
		})`,

    // A line of the hir says which expression it is, and a pointer on it asks the editor to
    // mark the code that expression was written as. An expression is not a token: what it
    // covers is what the lowering recorded for it.
    hoverHir: `const row = [...document.querySelectorAll('[data-panel=inspector] [data-kind=expr] .row')]
			.find((it) => text(it.querySelector('.text')).includes('literal 42'));
		row.dispatchEvent(new MouseEvent('mouseenter'));
		return JSON.stringify({ says: text(row.querySelector('.text')) })`,

    hirHovered: `const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
		return JSON.stringify({ count: parts.length, marked: parts.map((it) => it.textContent).join('') })`,

    // A path says what it names as well as what it is written as, and what it names is a place
    // in the same buffer: a pointer on the line asks the editor to mark both.
    hoverPath: `const row = [...document.querySelectorAll('[data-panel=inspector] [data-kind=path] .row')]
			.find((it) => text(it.querySelector('.text')).includes('-> fun println-int'));
		row.dispatchEvent(new MouseEvent('mouseenter'));
		return JSON.stringify({ says: text(row.querySelector('.text')).trim() })`,

    pathHovered: `const parts = (which) => [...document.querySelectorAll('.cm-content ' + which)];
		return JSON.stringify({
			at: parts('.cm-hovered').map((it) => it.textContent).join(''),
			names: parts('.cm-named').map((it) => it.textContent).join('')
		})`,

    // A type a signature writes is a place of the declaration as well: a pointer on the line
    // of a parameter asks the editor to mark the type it was annotated with.
    hoverType: `const row = [...document.querySelectorAll('[data-panel=inspector] [data-kind=field] .row')]
			.find((it) => text(it.querySelector('.text')).includes('param: Int ->'));
		row.dispatchEvent(new MouseEvent('mouseenter'));
		return JSON.stringify({ says: text(row.querySelector('.text')).trim() })`,

    typeHovered: `const parts = (which) => [...document.querySelectorAll('.cm-content ' + which)];
		return JSON.stringify({
			at: parts('.cm-hovered').map((it) => it.textContent).join(''),
			names: parts('.cm-named').map((it) => it.textContent).join('')
		})`,

    // The type a signature writes is a path, and it reads as one: the colour a part of a line
    // is painted is the colour the paths of the bodies are painted.
    typeColour: `const colour = (selector) => {
			const it = inspector().querySelector(selector);
			return it ? getComputedStyle(it).color : '';
		};
		return JSON.stringify({
			part: colour('[data-part=path]'),
			path: colour('[data-kind=path] .text')
		})`,

    showHir: `show('hir'); return true`,

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

    typeKeywords: `const content = document.querySelector('.cm-content');
		content.focus();
		content.textContent = ${JSON.stringify(VISIBLE)};
		content.dispatchEvent(new Event('input', { bubbles: true }));
		return true`,

    // Every keyword is painted the same, because a keyword is one tag in the grammar:
    // what the check has to catch is a word the grammar reads out of names going unpainted.
    keywordColours: `const spans = [...document.querySelectorAll('.cm-content .cm-line span')];
		const colour = (text) => {
			const span = spans.find((it) => it.textContent === text);
			return span ? getComputedStyle(span).color : '';
		};
		return JSON.stringify({
			keyword: colour('fun'),
			pub: colour('pub'),
			use: colour('use'),
			as: colour('as')
		})`,

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

    // The prelude of the library, opened as a tab: the step after this one closes it again.
    openPrelude: `document.querySelector('[data-file="/std/prelude.mlk"] .pick').click(); return true`,

    closeTab: `document.querySelector('[data-open="/std/prelude.mlk"] .close').click(); return true`,

    closed: `return JSON.stringify({
    		open: document.querySelectorAll('[data-panel=editor] [data-open]').length,
    		listed: document.querySelector('[data-file="/std/prelude.mlk"]') !== null
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
    	// What the theme calls green, read off the page the way every colour here is:
    	// a span wearing the variable, seen by a browser.
    	const probe = document.createElement('span');
    	probe.style.color = 'var(--ok)';
    	document.body.appendChild(probe);
    	const green = getComputedStyle(probe).color;
    	probe.remove();
    	return JSON.stringify({
    		keyword: colour('fun'),
    		number: colour('42'),
    		type: colour('Unit'),
    		name: colour('main'),
    		attribute: colour('#[extern]'),
    		green,
    		marks: document.querySelectorAll('.cm-content [class*=cm-lintRange], .cm-content [class*=cm-lintPoint]').length
    	})`,

    /**
     * A wide screen, where there is room for every panel at once and nothing to pick.
     *
     * The switcher is asked the same question a phone asks it: what it measures.
     */
    wide: `return JSON.stringify({
    		width: innerWidth,
    		switches: [...document.querySelectorAll('[data-pane]')]
    			.filter((it) => it.getBoundingClientRect().width > 0).length
    	})`,

    /**
     * A phone: the page at the width of one, where there is room for a single panel.
     *
     * What a panel is drawn as is what it measures: a panel a person did not pick takes
     * no room at all, so the boxes of the four of them say which one the screen shows.
     * The width is the narrowest a phone is made in, and nothing may overflow it.
     */
    phone: `return JSON.stringify({
    		width: innerWidth,
    		shown: panel(),
    		switches: document.querySelectorAll('[data-pane]').length,
    		handles: [...document.querySelectorAll('[role=separator]')]
    			.filter((it) => getComputedStyle(it).display !== 'none').length,
    		overflows: document.documentElement.scrollWidth > innerWidth
    	})`,

    pickFiles: `document.querySelector('[data-pane=files]').click(); return true`,

    shownFiles: `return JSON.stringify({
    		shown: panel(),
    		files: document.querySelectorAll('[data-panel=files] [data-file]').length,
    		overflows: document.documentElement.scrollWidth > innerWidth
    	})`,

    // Picking a buffer among the files is a request to write in it, not to read its name
    // again: the editor comes in front, with the buffer that was picked in it.
    pickBuffer: `document.querySelector('[data-file="/main.mlk"] .pick').click(); return true`,

    shownBuffer: `return JSON.stringify({
    		shown: panel(),
    		file: text(document.querySelector('[data-panel=editor] .tab.active .pick')),
    		overflows: document.documentElement.scrollWidth > innerWidth
    	})`,

    pickInspector: `document.querySelector('[data-pane=inspector]').click(); return true`,

    shownInspector: `return JSON.stringify({
    		shown: panel(),
    		tabs: document.querySelectorAll('[data-panel=inspector] [data-tab]').length,
    		overflows: document.documentElement.scrollWidth > innerWidth
    	})`,

    pickConsole: `document.querySelector('[data-pane=console]').click(); return true`,

    shownConsole: `return JSON.stringify({
    		shown: panel(),
    		lines: document.querySelector('[data-panel=console] .lines') !== null,
    		overflows: document.documentElement.scrollWidth > innerWidth
    	})`,

    // The standard library is in the files with everything else, and it is the compiler's:
    // the directory of the library says so once, and a row of it offers nothing to drop.
    library: `const row = document.querySelector('[data-file="/std/core.mlk"]');
    	const folder = document.querySelector('[data-folder=std]');
    	return JSON.stringify({
    		files: [...document.querySelectorAll('[data-file]')].map((it) => it.dataset.file),
    		drop: row === null ? null : row.querySelector('.drop') !== null,
    		file: text(row?.querySelector('.locked')),
    		locked: text(folder?.querySelector('.locked'))
    	})`,

    openLibrary: `document.querySelector('[data-file="/std/core.mlk"] .pick').click(); return true`,

    // What the editor holds of the file in front, and whether it offers to take text at all.
    readLibrary: `const content = document.querySelector('.cm-content');
    	return JSON.stringify({
    		editable: content.getAttribute('contenteditable'),
    		text: text(content),
    		tab: text(document.querySelector('[data-panel=editor] .tab.active .locked'))
    	})`,

    writeLibrary: `const content = document.querySelector('.cm-content');
    	content.focus();
    	content.textContent = ${JSON.stringify(WRITTEN)};
    	content.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    // A buffer cannot be made in the directory of the library, and the form says so.
    typeStdPath: `const field = document.querySelector('[data-panel=files] input');
    	field.value = 'std/thing.mlk';
    	field.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    submitStdPath: `document.querySelector('[data-panel=files] form').requestSubmit(); return true`,

    refusedStdPath: `return JSON.stringify({
    		problem: text(document.querySelector('[data-panel=files] .problem')),
    		made: document.querySelector('[data-file="/std/thing.mlk"]') !== null
    	})`,

    pickCode: `document.querySelector('[data-pane=code]').click(); return true`,

    typeLong: `const content = document.querySelector('.cm-content');
    	content.focus();
    	content.textContent = ${JSON.stringify(LONG)};
    	content.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    // The last token of the tree, which is at the far end of a buffer the screen does not hold.
    pickLastToken: `const rows = [...document.querySelectorAll('[data-panel=inspector] [data-kind=IDENT] .row')];
    	const row = rows[rows.length - 1];
    	row.dispatchEvent(new MouseEvent('mouseenter'));
    	row.click();
    	return JSON.stringify({ says: JSON.parse(text(row.querySelector('.text'))).trim() })`,

    pickedToken: `const scroller = document.querySelector('.cm-scroller');
    	return JSON.stringify({
    		shown: panel(),
    		marked: text(document.querySelector('.cm-content .cm-hovered')).trim(),
    		scrolls: scroller.scrollHeight > scroller.clientHeight,
    		scrolled: scroller.scrollTop > 0
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

    await ask(STEPS.showHir);
    const hir = JSON.parse(await ask(STEPS.hir));

    // A line of the hir stands for a node of the HIR, and the lowering is what says where that
    // node was written: a pointer on the line asks the editor to mark it.
    const hirSays = JSON.parse(await ask(STEPS.hoverHir));
    const hirHover = {
        ...hirSays,
        ...JSON.parse(await ask(STEPS.hirHovered)),
    };

    // And a path marks what it names besides itself: the declaration the name comes from.
    const pathSays = JSON.parse(await ask(STEPS.hoverPath));
    const pathHover = {
        ...pathSays,
        ...JSON.parse(await ask(STEPS.pathHovered)),
    };

    // A type of a signature is a place of its declaration: the annotation it was written as.
    const typeSays = JSON.parse(await ask(STEPS.hoverType));
    const typeHover = {
        ...typeSays,
        ...JSON.parse(await ask(STEPS.typeHovered)),
    };
    const typeColour = JSON.parse(await ask(STEPS.typeColour));

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

    // The words the language reads out of names are painted whatever they are written as,
    // and the buffer in front is asked about the ones a module is written with.
    await ask(STEPS.typeKeywords);
    await sleep(300);
    const words = JSON.parse(await ask(STEPS.keywordColours));

    // A mistake the parser cannot place is a node of the tree, and the ast shows it as one.
    await ask(STEPS.typeStray);
    await sleep(300);
    await ask(STEPS.showAst);
    const bogus = JSON.parse(await ask(STEPS.bogus));

    await ask(STEPS.hoverBogus);
    const bogusMark = JSON.parse(await ask(STEPS.bogusMark));

    // A file of the library opens as a tab like any other, and closing the tab is not dropping
    // the file: the library is read, not locked away.
    await ask(STEPS.openPrelude);

    await ask(STEPS.closeTab);
    const closed = JSON.parse(await ask(STEPS.closed));

    await ask(STEPS.askDrop);
    await ask(STEPS.confirmDrop);
    const dropped = JSON.parse(await ask(STEPS.dropped));

    // A wide screen draws every panel at once, and its switcher is asked about here: once the
    // page is narrowed below, the panels take turns being on the screen.
    const wide = JSON.parse(await ask(STEPS.wide));

    // A phone: the page at the width of one, where the panels are picked rather than laid out
    // side by side. The questions below are the ones a person asks with a thumb: what is on
    // the screen now, and what a tap puts there.
    await connection.send(
        "Emulation.setDeviceMetricsOverride",
        {
            width: 320,
            height: 568,
            deviceScaleFactor: 2,
            mobile: true,
            screenWidth: 320,
            screenHeight: 568,
        },
        session,
    );

    // The page hands the new size to the layout, which is not done before the next line runs.
    await sleep(300);

    const phone = JSON.parse(await ask(STEPS.phone));

    await ask(STEPS.pickFiles);
    const phoneFiles = JSON.parse(await ask(STEPS.shownFiles));

    await ask(STEPS.pickBuffer);
    const phoneBuffer = JSON.parse(await ask(STEPS.shownBuffer));

    await ask(STEPS.pickInspector);
    const phoneInspector = JSON.parse(await ask(STEPS.shownInspector));

    await ask(STEPS.pickConsole);
    const phoneConsole = JSON.parse(await ask(STEPS.shownConsole));

    // A row of a tree that holds nothing is a place in the source rather than a thing to fold:
    // picking one puts the editor in front, where the mark the row makes is read. A buffer is
    // longer than the screen it is read on, so what a pick asks for is the place itself.
    await ask(STEPS.pickCode);
    await ask(STEPS.typeLong);
    await sleep(300);
    await ask(STEPS.pickInspector);
    await ask(STEPS.showCst);
    const phoneSays = JSON.parse(await ask(STEPS.pickLastToken));
    const phoneToken = {
        ...phoneSays,
        ...JSON.parse(await ask(STEPS.pickedToken)),
    };

    // The standard library is in the files with everything else, and nothing of it is a
    // person's to write in or to drop.
    await ask(STEPS.pickFiles);
    const library = JSON.parse(await ask(STEPS.library));

    await ask(STEPS.openLibrary);
    const readLibrary = JSON.parse(await ask(STEPS.readLibrary));
    await ask(STEPS.writeLibrary);
    const writtenLibrary = JSON.parse(await ask(STEPS.readLibrary));

    // And a buffer cannot be made in the directory of the library: the form says why.
    await ask(STEPS.pickFiles);
    await ask(STEPS.open);
    await ask(STEPS.typeStdPath);
    await ask(STEPS.submitStdPath);
    const refusedStdPath = JSON.parse(await ask(STEPS.refusedStdPath));

    return report(
        {
            status: state.file,
            hydrated,
            clean: { root: cst.root, diagnostics: clean.diagnostics },
            ast,
            hir,
            hirHover,
            pathHover,
            typeHover,
            typeColour,
            program,
            width,
            made,
            closed,
            dropped,
            tree: cst.nodes,
            painted,
            words,
            hover,
            astHover,
            marks,
            broken,
            bogus,
            bogusMark,
            wide,
            phone,
            phoneFiles,
            phoneBuffer,
            phoneInspector,
            phoneConsole,
            phoneToken,
            library,
            readLibrary,
            writtenLibrary,
            refusedStdPath,
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
        [
            "the editor paints an attribute the green of the theme",
            page.painted.attribute !== "" &&
                page.painted.attribute === page.painted.green,
        ],
        [
            "the editor paints pub, use and as the way it paints fun",
            page.words.keyword !== "" &&
                [page.words.pub, page.words.use, page.words.as].every(
                    (it) => it === page.words.keyword,
                ),
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
        [
            "the hir is headed by the module and the path it is called by",
            page.hir.module === "MODULE #0 project::main",
        ],
        ["the hir names the items of the module", page.hir.item],
        ["the hir reads a body", page.hir.body],
        [
            "the hir reads the patterns and the paths of the body",
            page.hir.pat && page.hir.path,
        ],
        [
            "a line of the hir marks code in the editor",
            page.hirHover.count === 1,
        ],
        [
            "a line of the hir marks code in the editor",
            page.hirHover.count === 1,
        ],
        [
            "the mark is the code the line says it stands for",
            page.hirHover.says.includes("literal 42") &&
                page.hirHover.marked.trim() === "42",
        ],
        [
            "a path of the hir marks the code it is written as",
            page.pathHover.at === "println-int",
        ],
        [
            "and marks what the path resolved to",
            // The declaration of a function is written with the attributes it carries:
            // what a path leads to is the declaration, `#[extern]` and all.
            page.pathHover.names.endsWith("fun println-int(x: Int): Unit"),
        ],
        [
            "a type of a signature marks the type the declaration wrote",
            page.typeHover.at === "Int" && page.typeHover.names === "",
        ],
        [
            "a type of a signature is painted the way a path of a body is",
            page.typeColour.part !== "" &&
                page.typeColour.part === page.typeColour.path,
        ],
        ["the editor marks what it reported", page.marks.marks > 0],
        ["a buffer can be made at a path", page.made.file],
        ["a buffer opens as a tab", page.made.open === 2],
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
        ["nothing the ast shows reads as an object", !page.bogus.object],
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
        [
            "a wide screen draws the panels together, with no switcher to pick one",
            page.wide.width >= 860 && page.wide.switches === 0,
        ],
        [
            "a phone shows one panel at a time, and the switcher is how it is picked",
            page.phone.width < 860 &&
                page.phone.shown === "editor" &&
                page.phone.switches === 4 &&
                page.phone.handles === 0 &&
                !page.phone.overflows,
        ],
        [
            "picking Files shows the files",
            page.phoneFiles.shown === "files" &&
                page.phoneFiles.files === 3 &&
                !page.phoneFiles.overflows,
        ],
        [
            "picking a buffer shows the editor, with the buffer in front",
            page.phoneBuffer.shown === "editor" &&
                page.phoneBuffer.file === "main.mlk" &&
                !page.phoneBuffer.overflows,
        ],
        [
            "picking Inspect shows the inspector",
            page.phoneInspector.shown === "inspector" &&
                page.phoneInspector.tabs === 4 &&
                !page.phoneInspector.overflows,
        ],
        [
            "picking Console shows the console",
            page.phoneConsole.shown === "console" &&
                page.phoneConsole.lines &&
                !page.phoneConsole.overflows,
        ],
        [
            "picking a token of a tree shows the editor, with the token marked in it",
            page.phoneToken.shown === "editor" &&
                page.phoneToken.marked !== "" &&
                page.phoneToken.marked === page.phoneToken.says,
        ],
        [
            // A place is brought to a person, not merely marked: a buffer is read through a
            // window onto it, and a pick moves that window. A browser that lays the editor out
            // with nothing to scroll — the one this check drives is one — has no window to move,
            // and is asked for the mark alone.
            "picking a token brings the place it stands for into view",
            !page.phoneToken.scrolls || page.phoneToken.scrolled,
        ],
        [
            // The library is the compiler's: it is in the files with everything else, and the
            // directory of the library is what says so --- once, for everything under it.
            "the standard library is in the files, and offers nothing to drop",
            page.library.files.includes("/std/core.mlk") &&
                page.library.files.includes("/std/prelude.mlk") &&
                page.library.drop === false &&
                page.library.file === "" &&
                page.library.locked === "read-only",
        ],
        [
            // What the editor shows of a file of the library is what the compiler holds: a
            // state that is read-only takes no text, and the tab says what the file is.
            "a file of the library is read, and not written in",
            page.readLibrary.editable === "false" &&
                page.readLibrary.tab === "r/o" &&
                page.readLibrary.text.includes("module project::core") &&
                page.writtenLibrary.text === page.readLibrary.text,
        ],
        [
            "a buffer cannot be made in the directory of the library",
            page.refusedStdPath.problem ===
                "the standard library is read-only" &&
                !page.refusedStdPath.made,
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
        `it paints pub ${page.words.pub}, use ${page.words.use} and as ${page.words.as}, against fun ${page.words.keyword}`,
    );
    console.log(
        `it paints an attribute ${page.painted.attribute}, which is the green of the theme ${page.painted.green}`,
    );
    console.log(
        `the row that says ${page.hover.says} marks ${page.hover.marked || "nothing"} in the editor`,
    );
    console.log(
        `a row of the ast marks ${JSON.stringify(page.astHover.marked.slice(0, 40))}`,
    );
    console.log(
        `the hir reads ${page.hir.module}, and ${page.hirHover.says} marks ${JSON.stringify(page.hirHover.marked)}`,
    );
    console.log(
        `the path ${page.pathHover.says} marks ${JSON.stringify(page.pathHover.at)} and ${JSON.stringify(page.pathHover.names)}`,
    );
    console.log(
        `the type line ${page.typeHover.says} marks ${JSON.stringify(page.typeHover.at)} and is painted ${page.typeColour.part}`,
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
    console.log(
        `a phone at ${page.phone.width}px shows ${page.phone.shown}, and the switcher has ${page.phone.switches} panels to pick`,
    );
    console.log(
        `the files hold ${page.library.files.length} buffers, and the editor ${page.writtenLibrary.text === page.readLibrary.text ? "did not take" : "TOOK"} what was typed into a file of the library`,
    );
    console.log(
        `a pick of ${JSON.stringify(page.phoneToken.says)} in a tree marks ${JSON.stringify(page.phoneToken.marked)}, which the editor ${page.phoneToken.scrolls ? (page.phoneToken.scrolled ? "is taken to" : "stays away from") : "has nothing to scroll to"}`,
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
