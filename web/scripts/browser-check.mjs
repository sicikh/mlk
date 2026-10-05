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
 *     node scripts/browser-check.mjs [--base URL] [--cdp URL] [--keep] [--list] [--only TEXT]
 *
 * The check serves `web/build`, so the site has to be built first — `pnpm check:browser`
 * does that. `vite preview` takes a free port, and `obscura serve` is started when no
 * browser answers; both are taken down on the way out unless `--keep` asks otherwise.
 * Pass `--base` to check a site that already runs — the dev server, say. What the check
 * drives by default is the built site, which hands the driver's worker over as one script
 * every host runs; a dev server hands it over as a module, and obscura runs a worker as a
 * classic script whatever its type says, so a dev server is one this browser cannot drive.
 *
 * `--list` prints what the check asks and exits, and `--only TEXT` asks only the checks
 * whose wording contains TEXT; the flag may be given more than once. A check that fails does
 * not stop the rest: every check is read, the failures say what was expected against what the
 * run saw, and the last lines are the tally.
 *
 * What is asked of the page is the table `RUN`, and what is asked of what the run read is
 * the table `CHECKS`; both are written to be read as tables rather than as code.
 */
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { createServer } from "node:net";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Node reads the types of a `.ts` module itself, so the reader of an archive is the one the
// editor writes the archive with rather than a copy of it written here.
import { readArchive } from "../src/lib/archive.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** @typedef {object} Options
 * @property {string} [base]
 * @property {string} [cdp]
 * @property {boolean} [keep]
 * @property {boolean} [list]
 * @property {string[]} [only]
 *
 * What was given on the command line: where the site and the browser are, and which of the
 * checks to ask. Everything else the check needs it arranges for itself. */

/** @type {Options} */
const options = parseArguments(process.argv.slice(2));
const cdp = options.cdp ?? "http://127.0.0.1:9222";

/** Where the site is served: the one asked for, or one of our own. */
let base = options.base ?? "http://127.0.0.1:4173";

/** What the check brought up itself, and therefore has to take down again.
 *
 * @type {{name: string, child: import("node:child_process").ChildProcess, said: string[]}[]} */
const started = [];

/**
 * The page is asked one question at a time, and the browser runs it between questions.
 *
 * Nothing here waits: a question is one expression the page answers at once, and waiting inside
 * the page would be waiting on the thread the answer has to be read on. What the page does wait
 * for --- a pull of the compiler, which the worker answers a message later --- is waited for
 * from here, by asking the same question again (see `until`).
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
 * What is typed into a buffer to watch what a read costs: the same module twice, with the body
 * of it edited between the two.
 *
 * The first is what the second is measured by: the buffer was another module before it, so that
 * read is paid for whole, and the second changes one body and nothing a reader of the module
 * sees --- the signature of `main` is the same in both, which is what the surface of the module
 * is a function of. The numbers are ones the buffer that was there before does not write.
 */
const EDITED = "fun main(): Unit =\n    let x = 80 + 80 in\n    x\n";
const EDITED_AGAIN = "fun main(): Unit =\n    let x = 90 + 90 in\n    x\n";

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

/**
 * What is typed to watch the MIR of a choice: a body with an `if`, so that the CFG form reads
 * a block that branches and the arms meeting in a block of their own, and the SSA form reads the
 * value the arms agree on as a parameter of the block they meet in.
 */
const BRANCH =
    "fun pick(flag: Bool): Int =\n    if flag && true then 1 else 2\n";

/**
 * What is typed to watch the MIR of a choice that selects nothing: an `if` without an `else`,
 * whose arms are `Unit`. The block control falls into when no condition holds writes the unit
 * the choice is, and the SSA form passes it to the join like any other value.
 */
const UNIT =
    "fun log(flag: Bool): Unit =\n    if flag then\n        log(flag)\n";

/**
 * What is typed to watch the bodies of lambdas: a lambda written in a body, and a lambda written
 * in that lambda. The MIR and LIR tabs read each of them as a body of its own, flat, named under
 * the body that wrote it ([ADR-0019](../../docs/adr/0019-mir.md)).
 */
const LAMBDA =
    "fun main(): Int =\n    let add = fn(x: Int) -> fn(y: Int) -> x + y in\n    add(1)(2)\n";

/**
 * What a tab of the MIR reads: which form it shows, the bodies, and the lines of their blocks.
 *
 * The lines are read as a person reads them: the text of every statement and terminator, which
 * is what says whether the tab shows the CFG form (slots) or the SSA form (values of their own).
 */
const MIR = `const lines = (kind) => [...inspector().querySelectorAll('[data-line=' + kind + ']')].map((it) => text(it));
	return JSON.stringify({
		form: inspector().querySelector('[data-form]')?.dataset.form,
		owners: lines('owner'),
		lambdas: lines('owner').filter((it) => it.includes('<mlkc@lambda-')),
		blocks: [...inspector().querySelectorAll('[data-block]')].map((it) => it.dataset.block),
		entry: inspector().querySelectorAll('[data-entry=true]').length,
		stmts: lines('stmt'),
		term: lines('term'),
		locals: lines('local'),
		blockParams: [...inspector().querySelectorAll('[data-block] [data-line=value]')].map((it) => text(it)),
		branches: [...inspector().querySelectorAll('[data-line=term][data-kind=branch]')].length
	})`;

/**
 * What the tab of the LIR reads: the bodies of the target's instructions, what every instruction
 * is, and where the values that need storage live.
 *
 * The lines are read as a person reads them: the text of every instruction and terminator, and
 * the name of the instruction each line is, which is what says the tab shows the lowering of
 * the MIR into the instructions of the target.
 */
const LIR = `const lines = (kind) => [...inspector().querySelectorAll('[data-line=' + kind + ']')].map((it) => text(it));
	return JSON.stringify({
		owners: lines('owner'),
		lambdas: lines('owner').filter((it) => it.includes('<mlkc@lambda-')),
		blocks: [...inspector().querySelectorAll('[data-block]')].map((it) => it.dataset.block),
		entry: inspector().querySelectorAll('[data-entry=true]').length,
		insts: lines('inst'),
		kinds: [...inspector().querySelectorAll('[data-line=inst]')].map((it) => it.dataset.kind),
		term: lines('term'),
		termKinds: [...inspector().querySelectorAll('[data-line=term]')].map((it) => it.dataset.kind),
		params: lines('params'),
		locals: lines('locals'),
		blockParams: [...inspector().querySelectorAll('[data-block] [data-line=value]')].map((it) => text(it)),
		structure: lines('structure'),
		structureKinds: [...inspector().querySelectorAll('[data-line=structure]')].map((it) => it.dataset.kind),
		structureDepths: [...inspector().querySelectorAll('[data-line=structure]')].map((it) => Number(it.dataset.depth))
	})`;

/** The questions themselves, each answered by one round trip. @type {Record<string, string>} */
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
			pat: lines.some((it) => it.dataset.kind === 'pat' && text(it).includes('bind n')),
			path: lines.some((it) => it.dataset.kind === 'path' && text(it).includes('print-int'))
		})`,

    // A line of the hir says which expression it is, and a pointer on it asks the editor to
    // mark the code that expression was written as. An expression is not a token: what it
    // covers is what the lowering recorded for it.
    hoverHir: `const row = [...document.querySelectorAll('[data-panel=inspector] [data-kind=expr] .row')]
			.find((it) => text(it.querySelector('.text')).includes('literal 5'));
		row.dispatchEvent(new MouseEvent('mouseenter'));
		return JSON.stringify({ says: text(row.querySelector('.text')) })`,

    hirHovered: `const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
		return JSON.stringify({ count: parts.length, marked: parts.map((it) => it.textContent).join('') })`,

    // A path says what it names as well as what it is written as, and what it names is a place
    // in the same buffer: a pointer on the line asks the editor to mark both. The path taken is
    // the call of `fib`, which is a function of this buffer; a name of another module is written
    // in a file this one is not marked against.
    hoverPath: `const row = [...document.querySelectorAll('[data-panel=inspector] [data-kind=path] .row')]
			.find((it) => text(it.querySelector('.text')).trim().endsWith('-> fun fib'));
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

    showStats: `show('stats'); return true`,

    // What the driver did for the look that is in front, as the table of the tab reads it: the
    // rows are per pass and unit, and the time of a row is the time the pass spent running.
    stats: `const value = (cell) => cell.dataset.took !== undefined ? Number(cell.dataset.took) : Number(text(cell));
	const row = (element) => ({ pass: element.dataset.stats, unit: element.dataset.unit, ...Object.fromEntries([...element.querySelectorAll('[data-count]')]
		.map((cell) => [cell.dataset.count, value(cell)])) });
	return JSON.stringify({
		took: Number(inspector().querySelector('[data-cost=took]').dataset.took),
		rows: [...inspector().querySelectorAll('[data-stats]')].map(row)
	})`,

    showTc: `show('tc'); return true`,

    // What the tab reads of the buffer: the surface of the module, and a row per node of every
    // body, each of them a piece of the code and the type the checker gave it.
    tc: `return JSON.stringify({
    		surface: [...inspector().querySelectorAll('[data-tc=entity]')].map((it) => ({
    			name: text(it.querySelector('.saying')),
    			ty: text(it.querySelector('.ty'))
    		})),
    		bodies: inspector().querySelectorAll('[data-tc=body]').length,
    		nodes: [...inspector().querySelectorAll('[data-tc=node]')].map((it) => ({
    			kind: it.dataset.kind,
    			code: text(it.querySelector('.saying')),
    			ty: text(it.querySelector('.ty'))
    		})),
    		errors: inspector().querySelectorAll('[data-tc=node][data-error=true]').length
    	})`,

    // A row of the types stands for a node of a body the checker read, and a range of the
    // compiler is a range of the editor: a pointer on the row marks the code it is a type of.
    hoverTc: `const row = [...inspector().querySelectorAll('[data-tc=node]')]
    		.find((it) => text(it.querySelector('.ty')) === 'Int');
    	row.dispatchEvent(new MouseEvent('mouseenter'));
    	return true`,

    tcHovered: `const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
    	return JSON.stringify({
    		count: parts.length,
    		marked: parts.map((it) => it.textContent).join('')
    	})`,

    showMir: `show('mir'); return true`,

    mir: MIR,

    // A line of a body stands for the expression it was lowered from: a pointer on it asks the
    // editor to mark that expression in the buffer.
    hoverMir: `const row = [...inspector().querySelectorAll('[data-line=stmt]')]
    		.find((it) => text(it).includes('const 5'));
    	row.dispatchEvent(new MouseEvent('mouseenter'));
    	return JSON.stringify({ says: text(row) })`,

    mirHovered: `const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
    	return JSON.stringify({
    		count: parts.length,
    		marked: parts.map((it) => it.textContent).join('')
    	})`,

    showSsa: `show('mir-ssa'); return true`,

    ssa: MIR,

    showLir: `show('lir'); return true`,

    lir: LIR,

    // The WASM of the buffer: the module the link stage hands a host, printed as text and read
    // in a view of its own. What is read is what the view shows: the code on the screen, what
    // the head says the module holds, and the markers of the forms that fold.
    showWat: `show('wat'); return true`,

    wat: `const view = inspector().querySelector('[data-wat]');
		return JSON.stringify({
			shown: view !== null,
			text: text(view?.querySelector('.cm-content')),
			lines: view?.querySelectorAll('.cm-line').length ?? 0,
			marks: view?.querySelectorAll('.cm-foldPlaceholder').length ?? 0,
			head: text(view?.querySelector('.lines')),
			gutters: view?.querySelectorAll('.cm-foldGutter .cm-gutterElement').length ?? 0
		})`,

    // What the format is painted with: a word of it and a type of it against the colours of
    // the theme, which is what says the tab is painted rather than plain text.
    watPaint: `const view = inspector().querySelector('[data-wat]');
		const spans = [...view.querySelectorAll('.cm-content span')];
		const colour = (text) => {
			const span = spans.find((it) => it.textContent === text);
			return span ? getComputedStyle(span).color : '';
		};
		const probe = document.createElement('span');
		document.body.appendChild(probe);
		const theme = (name) => {
			probe.style.color = 'var(' + name + ')';
			return getComputedStyle(probe).color;
		};
		const accent = theme('--accent');
		const type = theme('--type');
		const ok = theme('--ok');
		probe.remove();
		return JSON.stringify({
			keyword: colour('i32.const'),
			type: colour('i31'),
			string: colour('"std::runtime"'),
			accent,
			typeColour: type,
			ok
		})`,

    // A marker belongs to the line the form opens on: the module line and each `func` head
    // fold, and a line of a body --- `local.get $n` above a `(ref i31)` --- opens nothing.
    // The gutter draws an element for a marked line and none for the rest, so what says which
    // line a marker is on is where the marker stands rather than where it is in the DOM. The
    // marker is as tall as a line and stands a hair above it, so what says which line it is on
    // is the middle of it: the boxes of two lines share an edge, and the top of a marker is on
    // the edge of the line above it as often as it is on its own.
    watMarks: `const view = inspector().querySelector('[data-wat]');
		const shown = (selector) => [...view.querySelectorAll(selector)]
			.filter((it) => getComputedStyle(it).visibility !== 'hidden');
		const lines = [...view.querySelectorAll('.cm-line')];
		const marks = shown('.cm-foldGutter .cm-gutterElement');
		const at = (mark) => {
			const box = mark.getBoundingClientRect();
			const middle = box.top + box.height / 2;
			return lines.findIndex((line) => {
				const it = line.getBoundingClientRect();
				return middle >= it.top && middle < it.bottom;
			});
		};
		const marked = (needle) => {
			const row = lines.findIndex((it) => text(it).includes(needle));
			return row >= 0 && marks.some((mark) => at(mark) === row);
		};
		return JSON.stringify({
			rows: lines.length,
			marks: marks.length,
			list: marks.map((mark) => {
				const row = at(mark);
				return row >= 0 ? text(lines[row]).trim().slice(0, 32) : '?';
			}),
			bodyMarked: marked('local.get $n'),
			headMarked: marked('(func $fib '),
			moduleMarked: marked('(module $app::main')
		})`,

    // A form of the module folds from the line it opens on, and unfolds again.
    foldWat: `inspector().querySelector('[data-wat-fold-all]').click(); return true`,

    unfoldWat: `inspector().querySelector('[data-wat-unfold-all]').click(); return true`,

    // A frame of the structure folds what it holds away, and unfolding brings it back. The
    // frame taken is the first one, and it stays on the screen while folded.
    foldLir: `inspector().querySelector('[data-lir] [data-fold]').click(); return true`,

    unfoldLir: `inspector().querySelector('[data-lir] [data-fold]').click(); return true`,

    // The name section stands at the end of the module, which a person reads by scrolling to it.
    scrollWatEnd: `const scroller = inspector().querySelector('[data-wat] .cm-scroller');
		scroller.scrollTop = scroller.scrollHeight;
		return true`,

    // The configuration of the compiler is a tab of the inspector rather than tools of the
    // header: it grows with the pipeline --- the passes, the target of the back end --- and a
    // person reads it where a person sets it.
    showConfig: `show('config'); return true`,

    config: `const debug = document.querySelector('[data-debug]');
		const opt = document.querySelector('[data-opt]');
		return JSON.stringify({
			debug: debug?.value ?? '',
			opt: opt?.value ?? '',
			options: debug ? [...debug.options].map((it) => it.value).join(' ') : ''
		})`,

    // The debug option decides what a module carries: the source map of a browser, the tables
    // of DWARF, or nothing. The tables are not in the text of the module --- they are binary
    // --- so what the editor shows of them is the list the head of the tab reads.
    debugMap: `const select = document.querySelector('[data-debug]');
		select.value = 'source-map';
		select.dispatchEvent(new Event('change', { bubbles: true }));
		return true`,

    debugDwarf: `const select = document.querySelector('[data-debug]');
		select.value = 'dwarf-lines';
		select.dispatchEvent(new Event('change', { bubbles: true }));
		return true`,

    debugNone: `const select = document.querySelector('[data-debug]');
		select.value = 'none';
		select.dispatchEvent(new Event('change', { bubbles: true }));
		return true`,

    watSections: `const head = inspector().querySelector('[data-wat-sections]');
		return JSON.stringify({
			sections: head ? [...head.querySelectorAll('[data-section]')]
				.map((it) => it.dataset.section) : [],
			head: text(head)
		})`,

    // The status line of the diagnostics, which is in front whatever stage is shown: what the
    // compiler reported, and the dot that says how bad it is at worst.
    status: `const status = document.querySelector('[data-tab=diagnostics]');
		return JSON.stringify({
			shown: status !== null && status.getBoundingClientRect().width > 0,
			text: text(status).trim(),
			active: status?.classList.contains('active') ?? false
		})`,

    // The fades at the ends of the row of stages: the row is one line that scrolls, and a
    // fade is drawn only on a side that hides something.
    fades: `const row = document.querySelector('[aria-label="The stages of the pipeline"]');
		return JSON.stringify({
			overflows: row ? row.scrollWidth > row.clientWidth : false,
			start: document.querySelector('.fade.start') !== null,
			end: document.querySelector('.fade.end') !== null
		})`,

    // The program: every buffer is compiled and linked, the modules are instantiated, and the
    // entry point is called. `fib(5)` prints 5, which is what the program console holds.
    run: `document.querySelector('[data-run]').click(); return true`,

    ran: `const lines = [...document.querySelectorAll('[data-panel=console] .line')];
		return JSON.stringify({
			tab: document.querySelector('[data-console=program].active') !== null,
			printed: lines.map((it) => text(it).trim())
		})`,

    // The header is what a person reaches for from anywhere: the tools of the project, marked
    // and named, and not the name of the buffer, which is on its tab in the editor.
    header: `const top = document.querySelector('header.top');
		const tools = [...top.querySelectorAll('button')];
		return JSON.stringify({
			brand: text(top.querySelector('.brand')).trim(),
			name: top.querySelector('.tool-name') !== null,
			tools: tools.map((it) => ({
				label: text(it.querySelector('.label')).trim(),
				aria: it.getAttribute('aria-label') ?? '',
				title: it.title,
				mark: it.querySelector('svg') !== null,
				primary: it.classList.contains('primary')
			}))
		})`,

    // Compiling the project is the pipeline end to end with nothing run: every module of the
    // project and of the library is compiled and linked into one archive, which is what a page
    // hands a person --- one download holding every path of the build. A page writes a file by
    // making a blob and starting the download of a link: this stands in for the blob and for
    // the link, because a headless browser keeps no files, and what a person would get is the
    // name and the bytes, which are read on the way.
    compile: `window.__build = [];
		window.__saved = [];
		URL.createObjectURL = (blob) => {
			window.__saved.push({ size: blob.size, type: blob.type });
			blob.arrayBuffer().then((buffer) => {
				const bytes = new Uint8Array(buffer);
				const chunks = [];
				for (let at = 0; at < bytes.length; at += 0x8000) {
					chunks.push(String.fromCharCode(...bytes.subarray(at, at + 0x8000)));
				}
				window.__zip = btoa(chunks.join(''));
			});
			return 'blob:build-' + window.__saved.length;
		};
		const create = document.createElement.bind(document);
		document.createElement = (name) => {
			const element = create(name);
			if (String(name).toLowerCase() === 'a') {
				element.click = function () {
					window.__build.push({
						name: this.download,
						blob: String(this.href).startsWith('blob:')
					});
				};
			}
			return element;
		};
		document.querySelector('[data-console=compiler]').click();
		document.querySelector('[data-compile]').click();
		return true`,

    built: `return JSON.stringify({
			files: window.__build ?? [],
			saved: window.__saved ?? [],
			lines: [...document.querySelectorAll('[data-panel=console] .line')]
				.map((it) => text(it).trim())
		})`,

    // The archive itself, as the text a check reads bytes in: what it holds is its paths, and
    // the paths are read off the directory at its end (see `readArchive` in `src/lib/archive`).
    packed: `return window.__zip ?? ''`,

    // The sources beside the build: every buffer of the project and of the library, under the
    // paths the debug information names them by. The same stub as the build reads the file,
    // and the archive of the build is already in hand, so this one replaces it.
    sources: `window.__zip = '';
		document.querySelector('[data-sources]').click();
		return true`,

    sourcesRead: `return JSON.stringify({
			files: window.__build ?? [],
			saved: window.__saved ?? [],
			lines: [...document.querySelectorAll('[data-panel=console] .line')]
				.map((it) => text(it).trim())
		})`,

    // A project with a mistake in it has no build a person can ask for: the tool is read rather
    // than pressed, and what it says is the reason.
    compileGuard: `const button = document.querySelector('[data-compile]');
		return JSON.stringify({
			disabled: button.disabled,
			title: button.title
		})`,

    // A body with a choice in it: the tab reads it as the blocks it branches into.
    typeBranch: `const content = document.querySelector('.cm-content');
    	content.focus();
    	content.textContent = ${JSON.stringify(BRANCH)};
    	content.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    // The same with a choice that selects no value.
    typeUnit: `const content = document.querySelector('.cm-content');
    	content.focus();
    	content.textContent = ${JSON.stringify(UNIT)};
    	content.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    // A lambda written in a body, and a lambda written in that one: the tabs read each of them
    // as a body of its own, nested where it is written.
    typeLambda: `const content = document.querySelector('.cm-content');
    	content.focus();
    	content.textContent = ${JSON.stringify(LAMBDA)};
    	content.dispatchEvent(new Event('input', { bubbles: true }));
    	return true`,

    // The keywords of a choice are the words of the language, and a truth value is a literal:
    // the editor paints the first as it paints every keyword, and the second as it paints a
    // number.
    branchWords: `const spans = [...document.querySelectorAll('.cm-content .cm-line span')];
    	const colour = (text) => {
    		const span = spans.find((it) => it.textContent === text);
    		return span ? getComputedStyle(span).color : '';
    	};
    	return JSON.stringify({
    		keyword: colour('fun'),
    		if: colour('if'),
    		then: colour('then'),
    		else: colour('else'),
    		number: colour('1'),
    		truth: colour('true')
    	})`,

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
    		empty: document.querySelector('[data-panel=console] .empty') !== null,
    		tab: document.querySelector('[data-console=program]') !== null
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

    typeEdited: `const content = document.querySelector('.cm-content');
		content.focus();
		content.textContent = ${JSON.stringify(EDITED)};
		content.dispatchEvent(new Event('input', { bubbles: true }));
		return true`,

    typeEditedAgain: `const content = document.querySelector('.cm-content');
		content.focus();
		content.textContent = ${JSON.stringify(EDITED_AGAIN)};
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
	    	number: colour('5'),
    		type: colour('Unit'),
    		name: colour('main'),
    		attribute: colour('#[entry]'),
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
    	// A mark is a range of the source, and the highlighter splits it where the code reads
    	// differently: the pieces together are what the row stands for.
    	const parts = [...document.querySelectorAll('.cm-content .cm-hovered')];
    	return JSON.stringify({
    		shown: panel(),
    		marked: parts.map((it) => it.textContent).join('').trim(),
    		scrolls: scroller.scrollHeight > scroller.clientHeight,
    		scrolled: scroller.scrollTop > 0
    	})`,
};

/** @typedef {object} Wait
 * @property {string} what what a person is told the step waited for
 * @property {string} test the question the page is asked until it answers yes
 * @property {number} [timeout] how long to keep asking, in ms
 *
 * A wait is the thing a step reads, asked as a question until it answers yes: the page paints
 * the answer of a pull a message later, and what a step reads is only there once it has. */

/**
 * What a step waits to see before it reads what it is about.
 *
 * A pull of the compiler is a question the page asks the worker, and the inspector paints the
 * answer a message later rather than in the click that asked for it: what a step is about to
 * read is only there once the tab holds it.
 *
 * @type {Record<string, Wait>}
 */
const WAITS = {
    hydrated: {
        what: "the page to come up",
        test: `return document.querySelectorAll('[data-panel]').length > 0`,
        timeout: 30000,
    },
    cst: {
        what: "the cst of the buffer",
        test: `return inspector().querySelector('[data-kind=MODULE_ROOT]') !== null`,
    },
    ast: {
        what: "the ast of the buffer",
        test: `return inspector().textContent.includes('ModuleRoot')`,
    },
    hir: {
        what: "the hir of the buffer",
        test: `return inspector().textContent.includes('BODY fun main')`,
    },
    tc: {
        what: "the types of the buffer",
        test: `return inspector().querySelector('[data-tc=entity]') !== null`,
    },
    mir: {
        what: "the mir of the buffer",
        test: `return inspector().querySelector('[data-form=cfg] [data-block]') !== null`,
    },
    ssa: {
        what: "the ssa form of the buffer",
        test: `return inspector().querySelector('[data-form=ssa] [data-line=term]') !== null`,
    },
    lir: {
        what: "the lir of the buffer",
        test: `return inspector().querySelector('[data-lir] [data-block]') !== null && inspector().querySelector('[data-lir] [data-structure] [data-line=structure]') !== null`,
    },
    branch: {
        what: "the branches of the mir",
        test: `return inspector().querySelectorAll('[data-form=cfg] [data-block]').length > 1`,
    },
    branchSsa: {
        what: "the branches of the ssa form",
        test: `return inspector().querySelector('[data-form=ssa] [data-kind=branch]') !== null`,
    },
    unit: {
        what: "the unit of a choice that selects nothing",
        test: `return inspector().textContent.includes('const unit')`,
    },
    unitSsa: {
        what: "the unit of the ssa form",
        test: `return inspector().textContent.includes('call fun log')`,
    },
    lambda: {
        what: "the lambda of the mir",
        test: `return [...inspector().querySelectorAll('[data-form=cfg] [data-line=owner]')].some((it) => text(it).includes('<mlkc@lambda-'))`,
    },
    lambdaLir: {
        what: "the lambda of the lir",
        test: `return [...inspector().querySelectorAll('[data-lir] [data-line=owner]')].some((it) => text(it).includes('<mlkc@lambda-'))`,
    },
    wat: {
        what: "the wasm of the buffer",
        test: `const view = inspector().querySelector('[data-wat]');
		return view !== null && view.querySelector('.cm-content') !== null`,
    },
    // The buffer was replaced by another module, so waiting for a word of it is waiting for
    // the editor to have taken what was typed, which is also when it is painted.
    branchWords: {
        what: "the words of the choice",
        test: `return text(document.querySelector('.cm-content')).includes('if flag && true')`,
    },
    keywords: {
        what: "the words of the module",
        test: `return text(document.querySelector('.cm-content')).includes('as data')`,
    },
    // The scroll is applied where the module has somewhere to go: a browser that lays the
    // view out with nothing to scroll has the whole of it in the DOM already.
    watEnd: {
        what: "the end of the module",
        test: `const scroller = inspector().querySelector('[data-wat] .cm-scroller');
		return scroller !== null && (scroller.scrollTop > 0 || scroller.scrollHeight <= scroller.clientHeight)`,
    },
    debugTables: {
        what: "the debug tables of the module",
        test: `const head = inspector().querySelector('[data-wat-sections]');
		return head !== null && head.textContent.includes('.debug_line')`,
    },
    debugMap: {
        what: "the source map of the module",
        test: `const head = inspector().querySelector('[data-wat-sections]');
		return head !== null && head.textContent.includes('sourceMappingURL')`,
    },
    debugNone: {
        what: "the module without debug information",
        test: `const head = inspector().querySelector('[data-wat-sections]');
		return head !== null && !head.textContent.includes('sourceMappingURL') &&
			!head.textContent.includes('.debug')`,
    },
    config: {
        what: "the configuration of the compiler",
        test: `return document.querySelector('[data-debug]') !== null &&
		document.querySelector('[data-opt]') !== null`,
    },
    ran: {
        what: "the program to run",
        test: `const lines = [...document.querySelectorAll('[data-panel=console] .line')];
		return document.querySelector('[data-console=program].active') !== null &&
			lines.some((it) => text(it).trim() === '5')`,
    },
    built: {
        what: "the build of the project",
        test: `return [...document.querySelectorAll('[data-panel=console] .line')]
			.some((it) => text(it).includes('the build is'))`,
    },
    packed: {
        what: "the archive of the build",
        test: `return (window.__zip ?? '').length > 0`,
    },
    sources: {
        what: "the archive of the sources",
        test: `return (window.__zip ?? '').length > 0`,
    },
    broken: {
        what: "the diagnostics of the broken buffer",
        test: `return diagnostics().length > 0`,
    },
    bogus: {
        what: "the node the grammar has no room for",
        test: `return inspector().textContent.includes('Bogus')`,
    },
    long: {
        what: "the tree of the long buffer",
        test: `return inspector().textContent.includes('x79')`,
    },
    edited: {
        what: "the tree of the edited buffer",
        test: `return inspector().textContent.includes('80')`,
    },
    editedAgain: {
        what: "the tree of the buffer edited again",
        test: `return inspector().textContent.includes('90')`,
    },
    // The page hands the new size to the layout, which is what puts the switcher in front.
    phone: {
        what: "the page at the width of a phone",
        test: `return innerWidth === 320 && document.querySelector('[data-pane=files]') !== null`,
    },
};

/** @typedef {object} RowSettings
 * @property {boolean} [merge] spread the answer over what the capture already holds
 * @property {boolean} [raw] keep the answer as the string the page gave
 * @property {object} [device] device metrics to set before the wait
 * @property {(value: any, page: Page) => any} [transform] what to keep of the answer
 *
 * What a row makes of the answer it read: the answer of a step is the JSON the page made,
 * and these are the few rows where that is not the whole story. */

/** @typedef {[string | null, string | null, (Wait | null)?, RowSettings?]} Row */

/**
 * What the run asks of the page, in order.
 *
 * A row is [capture, step, wait, settings]. The wait is asked until the page is ready for the
 * step, the step is asked, and the answer is kept under the capture — as the JSON it is, as
 * the raw string where `raw` says so, as what `transform` makes of it, or spread over what
 * the capture already holds where `merge` says so. A row with no capture is asked for its
 * effect alone, and a row with no step only waits and sets the device metrics.
 *
 * @type {Row[]}
 */
const RUN = [
    // The page hydrates after it has loaded, which is also when the kernel is fetched.
    ["state", STEPS.state, WAITS.hydrated],

    // What the header holds, which is what a person reaches for from anywhere in the page.
    ["header", STEPS.header],

    [null, STEPS.showCst],
    ["cst", STEPS.cst, WAITS.cst],

    // The editor paints the buffer in front, before anything is typed into it.
    ["painted", STEPS.painted],

    // A pointer on a row of the tree marks the code that row stands for.
    ["hover", STEPS.hoverTree],
    ["hover", STEPS.hovered, null, { merge: true }],

    [null, STEPS.showDiagnostics],
    ["clean", STEPS.clean],

    [null, STEPS.showProgram],
    ["program", STEPS.program],
    [null, STEPS.showCompiler],

    [null, STEPS.widen],
    ["width", STEPS.width],

    [null, STEPS.showAst],
    ["ast", STEPS.ast, WAITS.ast],

    // A node of the typed tree covers the tokens under it, and a pointer on its row marks as much.
    [null, STEPS.hoverAst],
    ["astHover", STEPS.astHovered],

    [null, STEPS.showHir],
    ["hir", STEPS.hir, WAITS.hir],

    // A line of the hir stands for a node of the HIR, and the lowering is what says where that
    // node was written: a pointer on the line asks the editor to mark it.
    ["hirHover", STEPS.hoverHir],
    ["hirHover", STEPS.hirHovered, null, { merge: true }],

    // And a path marks what it names besides itself: the declaration the name comes from.
    ["pathHover", STEPS.hoverPath],
    ["pathHover", STEPS.pathHovered, null, { merge: true }],

    // A type of a signature is a place of its declaration: the annotation it was written as.
    ["typeHover", STEPS.hoverType],
    ["typeHover", STEPS.typeHovered, null, { merge: true }],
    ["typeColour", STEPS.typeColour],

    // The types of the buffer: what the checker resolved the surface to, and what it checked
    // every node of every body to. A row of the tab stands for a node of the HIR, so a pointer
    // on one marks the code the node was read from.
    [null, STEPS.showTc],
    ["tc", STEPS.tc, WAITS.tc],
    [null, STEPS.hoverTc],
    ["tcHover", STEPS.tcHovered],

    // The MIR of the buffer: the CFG form the checked body is lowered into, and the SSA form
    // built from it. A line of a body stands for the expression it was read from, so a pointer
    // on one asks the editor to mark that code.
    [null, STEPS.showMir],
    ["mir", STEPS.mir, WAITS.mir],
    ["mirHover", STEPS.hoverMir],
    ["mirHover", STEPS.mirHovered, null, { merge: true }],

    [null, STEPS.showSsa],
    ["ssa", STEPS.ssa, WAITS.ssa],

    // The LIR of the buffer: what the WASM back end lowers the SSA form into, the target's own
    // instructions, where the values that need storage live, and the frames the encoder writes
    // them as.
    [null, STEPS.showLir],
    ["lir", STEPS.lir, WAITS.lir],

    // A frame of the structure folds what it holds away, and unfolding brings the lines back.
    [null, STEPS.foldLir],
    ["lirFolded", STEPS.lir],
    [null, STEPS.unfoldLir],
    ["lirAgain", STEPS.lir],

    // The module the link stage hands a host: the same bytes a run instantiates, read in a view
    // of its own, where the forms of it fold.
    [null, STEPS.showWat],
    ["wat", STEPS.wat, WAITS.wat],
    ["watPaint", STEPS.watPaint],
    ["watMarks", STEPS.watMarks],

    [null, STEPS.foldWat],
    ["watFolded", STEPS.wat],
    [null, STEPS.unfoldWat],
    ["watAgain", STEPS.wat],

    // The name section of the module stands at the end of it: reading it is scrolling to it.
    [null, STEPS.scrollWatEnd],
    ["watEnd", STEPS.wat, WAITS.watEnd],

    // The status line of the inspector is in front whatever stage is shown: a buffer that
    // reports nothing says so while the WAT is in front, and picking the line opens the list.
    ["glance", STEPS.status],
    ["fades", STEPS.fades],

    // The configuration of the compiler is a tab of its own: the source map of a browser, the
    // tables of DWARF, and nothing are alternatives, and a module carries one of them. A change
    // is made in the config tab and read off the WAT of the module, which is where the head of
    // the tab lists what the option added.
    [null, STEPS.showConfig],
    ["config", STEPS.config, WAITS.config],

    [null, STEPS.debugDwarf],
    [null, STEPS.showWat],
    ["dwarfWat", STEPS.watSections, WAITS.debugTables],

    [null, STEPS.showConfig],
    [null, STEPS.debugNone],
    [null, STEPS.showWat],
    ["bareWat", STEPS.watSections, WAITS.debugNone],

    [null, STEPS.showConfig],
    [null, STEPS.debugMap],
    [null, STEPS.showWat],
    ["mapWat", STEPS.watSections, WAITS.debugMap],

    // And the program itself: the modules are instantiated in the order the link stage gives
    // them, and the `#[entry]` is called. A run of a browser is under the map, which is where
    // the option stands; what `fib(5)` prints is what the program console holds.
    [null, STEPS.run],
    ["ran", STEPS.ran, WAITS.ran],

    // The same project, built rather than run: every module of it is compiled and linked, and
    // each is handed over as a file. A page writes a file by starting the download of a link,
    // and what this reads is the names and the console, not the file system a page has none of.
    [null, STEPS.compile],
    ["zip", STEPS.packed, WAITS.packed, { raw: true }],
    [
        "built",
        STEPS.built,
        WAITS.built,
        {
            // The archive a person gets: what it holds is read by the reader the editor
            // writes it with, and the manifest is read out of the archive, because it is
            // what a host that reads the build from files reads.
            transform: (value, page) => {
                const files = readArchive(Buffer.from(page.zip, "base64"));
                const described = files.find(
                    (it) => it.path === "manifest.json",
                );

                if (!described) throw new Error("the build holds no manifest");

                return {
                    ...value,
                    entries: files.map((it) => it.path),
                    manifest: JSON.parse(
                        new TextDecoder().decode(described.bytes),
                    ),
                };
            },
        },
    ],

    // The sources a debugger reads beside a build: an archive whose entries are the paths of
    // the debug information without their root, so one mapping rule points a debugger at the
    // directory they are unpacked into. The name of the archive is the name of the build's.
    [null, STEPS.sources],
    ["sourceZip", STEPS.packed, WAITS.sources, { raw: true }],
    [
        "sources",
        STEPS.sourcesRead,
        null,
        {
            transform: (value, page) => {
                const files = readArchive(
                    Buffer.from(page.sourceZip, "base64"),
                );
                const main = files.find((it) => it.path === "main.mlk");

                if (!main) throw new Error("the sources hold no main.mlk");

                return {
                    ...value,
                    entries: files.map((it) => it.path),
                    main: new TextDecoder().decode(main.bytes),
                };
            },
        },
    ],

    // A body with a choice in it: the CFG form reads the block that branches and the blocks the
    // arms meet in, and the SSA form gives the value the arms agree on a parameter of the block
    // they meet in. The buffer is typed into the one that is in front.
    [null, STEPS.typeBranch],
    ["branchWords", STEPS.branchWords, WAITS.branchWords],

    [null, STEPS.showMir],
    ["branch", STEPS.mir, WAITS.branch],

    [null, STEPS.showSsa],
    ["branchSsa", STEPS.ssa, WAITS.branchSsa],

    // A choice that selects nothing: the block control falls into when no condition holds
    // writes the unit the choice is, and the SSA form passes it to the join.
    [null, STEPS.typeUnit],

    [null, STEPS.showMir],
    ["unit", STEPS.mir, WAITS.unit],

    [null, STEPS.showSsa],
    ["unitSsa", STEPS.ssa, WAITS.unitSsa],

    // A lambda is a body of the module like any other: the tabs read it flat, named under the
    // body that wrote it, and a lambda written in a lambda is a body of the same set.
    [null, STEPS.typeLambda],

    [null, STEPS.showMir],
    ["lambdaMir", STEPS.mir, WAITS.lambda],

    [null, STEPS.showLir],
    ["lambdaLir", STEPS.lir, WAITS.lambdaLir],

    [null, STEPS.open],
    [null, STEPS.typePath],
    [null, STEPS.submitPath],
    ["made", STEPS.made],

    // Type into the buffer that was just made, the way a person would.
    [null, STEPS.type],

    [null, STEPS.showDiagnostics],
    ["broken", STEPS.seen, WAITS.broken],

    // A project with a mistake in it offers no build, and the tool says why.
    ["brokenGlance", STEPS.status],
    ["compileGuard", STEPS.compileGuard],

    ["marks", STEPS.painted],

    // The words the language reads out of names are painted whatever they are written as,
    // and the buffer in front is asked about the ones a module is written with.
    [null, STEPS.typeKeywords],
    ["words", STEPS.keywordColours, WAITS.keywords],

    // A mistake the parser cannot place is a node of the tree, and the ast shows it as one.
    [null, STEPS.typeStray],
    [null, STEPS.showAst],
    ["bogus", STEPS.bogus, WAITS.bogus],

    [null, STEPS.hoverBogus],
    ["bogusMark", STEPS.bogusMark],

    // A file of the library opens as a tab like any other, and closing the tab is not dropping
    // the file: the library is read, not locked away.
    [null, STEPS.openPrelude],

    [null, STEPS.closeTab],
    ["closed", STEPS.closed],

    [null, STEPS.askDrop],
    [null, STEPS.confirmDrop],
    ["dropped", STEPS.dropped],

    // A wide screen draws every panel at once, and its switcher is asked about here: once the
    // page is narrowed below, the panels take turns being on the screen.
    ["wide", STEPS.wide],

    // A phone: the page at the width of one, where the panels are picked rather than laid out
    // side by side. The questions below are the ones a person asks with a thumb: what is on
    // the screen now, and what a tap puts there.
    [
        "phone",
        STEPS.phone,
        WAITS.phone,
        {
            device: {
                width: 320,
                height: 568,
                deviceScaleFactor: 2,
                mobile: true,
                screenWidth: 320,
                screenHeight: 568,
            },
        },
    ],

    [null, STEPS.pickFiles],
    ["phoneFiles", STEPS.shownFiles],

    [null, STEPS.pickBuffer],
    ["phoneBuffer", STEPS.shownBuffer],

    [null, STEPS.pickInspector],
    ["phoneInspector", STEPS.shownInspector],

    [null, STEPS.pickConsole],
    ["phoneConsole", STEPS.shownConsole],

    // A row of a tree that holds nothing is a place in the source rather than a thing to fold:
    // picking one puts the editor in front, where the mark the row makes is read. A buffer is
    // longer than the screen it is read on, so what a pick asks for is the place itself.
    [null, STEPS.pickCode],
    [null, STEPS.typeLong],
    [null, STEPS.pickInspector],
    [null, STEPS.showCst],
    ["phoneToken", STEPS.pickLastToken, WAITS.long],
    ["phoneToken", STEPS.pickedToken, null, { merge: true }],

    // The standard library is in the files with everything else, and nothing of it is a
    // person's to write in or to drop.
    [null, STEPS.pickFiles],
    ["library", STEPS.library],

    [null, STEPS.openLibrary],
    ["readLibrary", STEPS.readLibrary],
    [null, STEPS.writeLibrary],
    ["writtenLibrary", STEPS.readLibrary],

    // And a buffer cannot be made in the directory of the library: the form says why.
    [null, STEPS.pickFiles],
    [null, STEPS.open],
    [null, STEPS.typeStdPath],
    [null, STEPS.submitStdPath],
    ["refusedStdPath", STEPS.refusedStdPath],

    // What a read cost: the buffer in front is filled with a module, and then the body of it
    // is edited, which is one value read again and nothing else. The look before the edit is
    // the one the second is read against: it is what the driver holds afterwards.
    [null, STEPS.pickBuffer],
    [null, STEPS.typeEdited],
    [null, null, WAITS.edited],
    [null, STEPS.typeEditedAgain],
    [null, null, WAITS.editedAgain],
    [null, STEPS.showStats],
    ["stats", STEPS.stats],
];

/**
 * What a check wants of a value equality cannot say, said the way a person reads it.
 */
class Want {
    /**
     * @param {string} said what a person is told the value should be
     * @param {(actual: any) => boolean} holds whether it is
     */
    constructor(said, holds) {
        this.said = said;
        this.holds = holds;
    }
}

/** The words a value is said in: the JSON of it, or what `String` makes of it, cut short.
 *
 * @param {any} value
 * @returns {string}
 */
function show(value) {
    if (value === undefined) return "(nothing)";

    const text = JSON.stringify(value) ?? String(value);

    return text.length > 200 ? `${text.slice(0, 200)}…` : text;
}

/** Whether two read values are the same, deep: a matcher says so itself where one stands.
 *
 * @param {any} expected
 * @param {any} actual
 * @returns {boolean}
 */
function same(expected, actual) {
    if (expected instanceof Want) return expected.holds(actual);

    if (Array.isArray(expected) && Array.isArray(actual))
        return (
            expected.length === actual.length &&
            expected.every((it, at) => same(it, actual[at]))
        );

    if (object(expected) && object(actual)) {
        const keys = new Set([
            ...Object.keys(expected),
            ...Object.keys(actual),
        ]);

        return [...keys].every((key) => same(expected[key], actual[key]));
    }

    return expected === actual;
}

/** Whether a value is an object to be read key by key, and not a matcher or an array.
 *
 * @param {any} value
 * @returns {boolean}
 */
function object(value) {
    return (
        typeof value === "object" &&
        value !== null &&
        !Array.isArray(value) &&
        !(value instanceof Want)
    );
}

/**
 * Where what was read parts from what was expected: what a check of one value says, or a line
 * per place in an object or a list. An empty list is a check that holds.
 *
 * @param {any} expected
 * @param {any} actual
 * @param {string} [at]
 * @returns {{at: string, expected: string, actual: string}[]}
 */
function departures(expected, actual, at = "") {
    // A matcher says what it wanted of the value in its own words.
    if (expected instanceof Want)
        return expected.holds(actual)
            ? []
            : [{ at, expected: expected.said, actual: show(actual) }];

    if (Array.isArray(expected) && Array.isArray(actual)) {
        const lines = [];

        if (expected.length !== actual.length)
            lines.push({
                at: `${at}.length`,
                expected: `${expected.length} element(s)`,
                actual: `${actual.length} element(s): ${show(actual)}`,
            });

        const shared = Math.min(expected.length, actual.length);

        for (let index = 0; index < shared; index++)
            lines.push(
                ...departures(
                    expected[index],
                    actual[index],
                    at ? `${at}.${index}` : `${index}`,
                ),
            );

        return lines;
    }

    if (object(expected) && object(actual)) {
        const keys = new Set([
            ...Object.keys(expected),
            ...Object.keys(actual),
        ]);
        const lines = [];

        for (const key of keys)
            lines.push(
                ...departures(
                    expected[key],
                    actual[key],
                    at ? `${at}.${key}` : key,
                ),
            );

        return lines;
    }

    return same(expected, actual)
        ? []
        : [{ at, expected: show(expected), actual: show(actual) }];
}

/** What a person is told a value should be, and whether it is.
 *
 * @param {string} said
 * @param {(actual: any) => boolean} holds
 */
const want = (said, holds) => new Want(said, holds);

/** More than a number. @param {number} than */
const greater = (than) =>
    want(`more than ${than}`, (it) => typeof it === "number" && it > than);

/** At least a number. @param {number} than */
const atLeast = (than) =>
    want(`at least ${than}`, (it) => typeof it === "number" && it >= than);

/** Less than a number. @param {number} than */
const less = (than) =>
    want(`less than ${than}`, (it) => typeof it === "number" && it < than);

/** Anything but a value, or anything but what a matcher wants. @param {any} value */
const not = (value) =>
    value instanceof Want
        ? want(`not ${value.said}`, (it) => !value.holds(it))
        : want(`not ${show(value)}`, (it) => !same(it, value));

/** A string or a list that holds a part. @param {any} part */
const includes = (part) =>
    want(`a value that contains ${show(part)}`, (it) => {
        return (
            (typeof it === "string" || Array.isArray(it)) && it.includes(part)
        );
    });

/** A string a pattern reads. @param {RegExp} pattern */
const matches = (pattern) =>
    want(`a value that matches ${pattern}`, (it) => {
        return typeof it === "string" && pattern.test(it);
    });

/**
 * A list with no element that is something.
 *
 * @param {string} said
 * @param {(it: any) => boolean} holds
 */
const none = (said, holds) =>
    want(`no element that is ${said}`, (it) => {
        return Array.isArray(it) && !it.some(holds);
    });

/** A value that satisfies every one of several matchers. @param {...Want} matchers */
const allOf = (...matchers) =>
    want(matchers.map((it) => it.said).join(" and "), (actual) => {
        return matchers.every((it) => it.holds(actual));
    });

/**
 * The shape of every capture, named where it is read. They are written down rather than left
 * to be inferred from the steps, because a step is a string the page evaluates: what a step
 * reads is known to the step and to the checks that read it, and this is that knowledge
 * written once.
 *
 * @typedef {{panels: number, file: string}} State
 * @typedef {{label: string, aria: string, title: string, mark: boolean, primary: boolean}} Tool
 * @typedef {{brand: string, name: boolean, tools: Tool[]}} Header
 * @typedef {{nodes: number, root: boolean}} Cst
 * @typedef {{keyword: string, number: string, type: string, name: string, attribute: string, green: string, marks: number}} Painted
 * @typedef {{count: number, marked: string}} Mark
 * @typedef {{says: string, count: number, marked: string}} Reach
 * @typedef {{diagnostics: number}} Clean
 * @typedef {{empty: boolean, tab: boolean}} Program
 * @typedef {{files: number}} Width
 * @typedef {{root: boolean, decl: boolean}} Ast
 * @typedef {{module: string, item: boolean, body: boolean, pat: boolean, path: boolean}} Hir
 * @typedef {{says: string, at: string, names: string}} Places
 * @typedef {{part: string, path: string}} TypeColour
 * @typedef {{name: string, ty: string}} Entity
 * @typedef {{kind: string, code: string, ty: string}} Typed
 * @typedef {{surface: Entity[], bodies: number, nodes: Typed[], errors: number}} Types
 * @typedef {{form: string, owners: string[], lambdas: string[], blocks: string[], entry: number, stmts: string[], term: string[], locals: string[], blockParams: string[], branches: number}} Mir
 * @typedef {{owners: string[], lambdas: string[], blocks: string[], entry: number, insts: string[], kinds: string[], term: string[], termKinds: string[], params: string[], locals: string[], blockParams: string[], structure: string[], structureKinds: string[], structureDepths: number[]}} Lir
 * @typedef {{shown: boolean, text: string, lines: number, marks: number, head: string, gutters: number}} Wat
 * @typedef {{keyword: string, type: string, string: string, accent: string, typeColour: string, ok: string}} WatPaint
 * @typedef {{rows: number, marks: number, list: string[], bodyMarked: boolean, headMarked: boolean, moduleMarked: boolean}} WatMarks
 * @typedef {{shown: boolean, text: string, active: boolean}} Glance
 * @typedef {{overflows: boolean, start: boolean, end: boolean}} Fades
 * @typedef {{debug: string, opt: string, options: string}} Config
 * @typedef {{sections: string[], head: string}} Sections
 * @typedef {{tab: boolean, printed: string[]}} Ran
 * @typedef {{size: number, type: string}} Saved
 * @typedef {{name: string, blob: boolean}} File
 * @typedef {{external: boolean, module: string, name: string}} Imported
 * @typedef {{file: string, name: string, imports: Imported[]}} Module
 * @typedef {{project: string, host: string, entry: {module: string, name: string}, modules: Module[]}} Manifest
 * @typedef {{files: File[], saved: Saved[], lines: string[], entries: string[], manifest: Manifest}} Built
 * @typedef {{files: File[], saved: Saved[], lines: string[], entries: string[], main: string}} Sources
 * @typedef {{disabled: boolean, title: string}} Guard
 * @typedef {{keyword: string, pub: string, use: string, as: string}} Words
 * @typedef {{keyword: string, if: string, then: string, else: string, number: string, truth: string}} ChoiceWords
 * @typedef {{classes: string[], code: string, message: string, number: string, caret: string, whole: string}} Diagnostic
 * @typedef {{object: boolean, shown: boolean}} Bogus
 * @typedef {{open: number, listed: boolean}} Closed
 * @typedef {{file: boolean}} Dropped
 * @typedef {{file: boolean, open: number}} Made
 * @typedef {{width: number, switches: number}} Wide
 * @typedef {{width: number, shown: string, switches: number, handles: number, overflows: boolean}} Phone
 * @typedef {{shown: string, files: number, overflows: boolean}} PhoneFiles
 * @typedef {{shown: string, file: string, overflows: boolean}} PhoneBuffer
 * @typedef {{shown: string, tabs: number, overflows: boolean}} PhoneInspector
 * @typedef {{shown: string, lines: boolean, overflows: boolean}} PhoneConsole
 * @typedef {{says: string, shown: string, marked: string, scrolls: boolean, scrolled: boolean}} PhoneToken
 * @typedef {{files: string[], drop: boolean, file: string, locked: string}} Library
 * @typedef {{editable: string, text: string, tab: string}} Read
 * @typedef {{problem: string, made: boolean}} Refused
 * @typedef {{pass: string, unit: string, hits: number, misses: number, stales: number, kept: number, dropped: number, took: number}} StatsRow
 * @typedef {{took: number, rows: StatsRow[]}} Stats
 */

/**
 * What the run captured, by name: the run and the checks agree on the names, and the shape of
 * each is what the step that read it made of the page (see `RUN`). A step that could not run
 * leaves its capture missing at runtime, which is the one thing this shape does not say; the
 * checks that read it say so rather than the report crashing.
 *
 * @typedef {object} Page
 * @property {State} state
 * @property {Header} header
 * @property {Cst} cst
 * @property {Painted} painted
 * @property {Reach} hover
 * @property {Reach} hirHover
 * @property {Clean} clean
 * @property {Program} program
 * @property {Width} width
 * @property {Ast} ast
 * @property {Mark} astHover
 * @property {Hir} hir
 * @property {Places} pathHover
 * @property {Places} typeHover
 * @property {TypeColour} typeColour
 * @property {Types} tc
 * @property {Mark} tcHover
 * @property {Mir} mir
 * @property {Reach} mirHover
 * @property {Mir} ssa
 * @property {Lir} lir
 * @property {Lir} lirFolded
 * @property {Lir} lirAgain
 * @property {Mir} branch
 * @property {Mir} branchSsa
 * @property {ChoiceWords} branchWords
 * @property {Mir} unit
 * @property {Mir} unitSsa
 * @property {Mir} lambdaMir
 * @property {Lir} lambdaLir
 * @property {Painted} marks
 * @property {Made} made
 * @property {Closed} closed
 * @property {Dropped} dropped
 * @property {Wat} wat
 * @property {Wat} watFolded
 * @property {Wat} watAgain
 * @property {Wat} watEnd
 * @property {WatPaint} watPaint
 * @property {WatMarks} watMarks
 * @property {Glance} glance
 * @property {Fades} fades
 * @property {Config} config
 * @property {Sections} dwarfWat
 * @property {Sections} bareWat
 * @property {Sections} mapWat
 * @property {Ran} ran
 * @property {string} zip
 * @property {Built} built
 * @property {string} sourceZip
 * @property {Sources} sources
 * @property {Guard} compileGuard
 * @property {Words} words
 * @property {Diagnostic[]} broken
 * @property {Glance} brokenGlance
 * @property {Bogus} bogus
 * @property {Mark} bogusMark
 * @property {Wide} wide
 * @property {Phone} phone
 * @property {PhoneFiles} phoneFiles
 * @property {PhoneBuffer} phoneBuffer
 * @property {PhoneInspector} phoneInspector
 * @property {PhoneConsole} phoneConsole
 * @property {PhoneToken} phoneToken
 * @property {Library} library
 * @property {Read} readLibrary
 * @property {Read} writtenLibrary
 * @property {Refused} refusedStdPath
 * @property {Stats} stats
 */

/** @typedef {object} Found
 * @property {string[]} problems what the page said that it should not have
 * @property {string[]} warnings what it said that is worth reading
 * @property {string[]} asked what it asked the network for
 * @property {{capture: string | null, error: string}[]} failed the steps that could not run
 *
 * Everything the run found besides the captures. */

/** @typedef {(page: Page, found: Found) => any} Reading
 *
 * What a check reads: a value, or what a function of the captures makes of them. */

/** @typedef {Reading | string | number | boolean | object | null | undefined} Anything */

/** @typedef {[string, Anything, Anything]} Check */

/**
 * What the run asks of what it captured.
 *
 * A check is [what it asks, what the run read, what it expected]. Both sides may be values or
 * functions of the captures, because an expectation is often read off another capture --- "the
 * same blocks as the cfg form". A node of an expectation may be a `Want`, for what equality
 * cannot say. Every check is read even when one fails, so one run says all that is broken.
 *
 * @type {Check[]}
 */
const CHECKS = [
    ["the page hydrated", (page) => page.state.panels, greater(0)],

    ["the cst of a clean buffer is a module", (page) => page.cst.root, true],
    ["the tree is more than its root", (page) => page.cst.nodes, greater(5)],
    ["a clean buffer reports nothing", (page) => page.clean.diagnostics, 0],

    ["the ast names its root", (page) => page.ast.root, true],
    ["the ast names a declaration", (page) => page.ast.decl, true],

    ["the editor paints a keyword", (page) => page.painted.keyword, not("")],
    [
        "the editor paints code in more than one colour",
        (page) => ({
            keyword: page.painted.keyword !== "",
            number: page.painted.number !== page.painted.keyword,
            type: page.painted.type !== page.painted.number,
            name: page.painted.name !== page.painted.keyword,
        }),
        { keyword: true, number: true, type: true, name: true },
    ],
    [
        "the editor leaves a name the colour of text",
        (page) => page.painted.name,
        "",
    ],
    [
        "the editor paints an attribute the green of the theme",
        (page) => ({
            painted: page.painted.attribute !== "",
            green: page.painted.attribute === page.painted.green,
        }),
        { painted: true, green: true },
    ],
    [
        "the editor paints pub, use and as the way it paints fun",
        (page) => ({
            keyword: page.words.keyword,
            pub: page.words.pub,
            use: page.words.use,
            as: page.words.as,
        }),
        (page) => {
            const keyword = page.words.keyword;

            return {
                keyword: not(""),
                pub: keyword,
                use: keyword,
                as: keyword,
            };
        },
    ],

    ["a row of a tree marks code in the editor", (page) => page.hover.count, 1],
    [
        "the mark is the code the row says it stands for",
        (page) => page.hover.marked,
        (page) => JSON.parse(page.hover.says),
    ],
    [
        "a row of the ast marks code in the editor",
        (page) => page.astHover.count,
        greater(0),
    ],
    [
        "a node of the ast marks what it holds",
        (page) => ({
            count: page.astHover.count > 0,
            // What a node covers is written over as many lines as it takes, and a mark is a
            // piece of a line: the pieces together are what the node covers.
            covers: page.astHover.marked.includes(page.hover.marked.trim()),
            more:
                page.astHover.marked.trim().length >
                page.hover.marked.trim().length,
        }),
        { count: true, covers: true, more: true },
    ],

    [
        "the hir is headed by the module and the path it is called by",
        (page) => page.hir.module,
        "MODULE #0 project::main",
    ],
    ["the hir names the items of the module", (page) => page.hir.item, true],
    ["the hir reads a body", (page) => page.hir.body, true],
    [
        "the hir reads the patterns and the paths of the body",
        (page) => [page.hir.pat, page.hir.path],
        [true, true],
    ],
    [
        "a line of the hir marks code in the editor",
        (page) => page.hirHover.count,
        1,
    ],
    [
        "the mark is the code the line says it stands for",
        (page) => ({
            says: page.hirHover.says.includes("literal 5"),
            marked: page.hirHover.marked.trim(),
        }),
        { says: true, marked: "5" },
    ],
    [
        "a path of the hir marks the code it is written as",
        (page) => page.pathHover.at,
        "fib",
    ],
    [
        "and marks what the path resolved to",
        // What a path leads to is the declaration it names, which is a place in the same
        // buffer: the path taken above is the call of `fib` in `main`.
        (page) => page.pathHover.names,
        includes("fun fib(n : Int) : Int"),
    ],
    [
        "a type of a signature marks the type the declaration wrote",
        (page) => ({ at: page.typeHover.at, names: page.typeHover.names }),
        { at: "Int", names: "" },
    ],
    [
        "a type of a signature is painted the way a path of a body is",
        (page) => ({
            painted: page.typeColour.part !== "",
            same: page.typeColour.part === page.typeColour.path,
        }),
        { painted: true, same: true },
    ],

    [
        "the tc tab reads the surface of the module and the types of its bodies",
        (page) => ({
            main: page.tc.surface.some(
                (it) => it.name === "fun main" && it.ty === "() -> Unit",
            ),
            aux: page.tc.surface.some(
                (it) =>
                    it.name === "fun fib-aux" &&
                    it.ty === "(Int, Int, Int) -> Int",
            ),
            bodies: page.tc.bodies,
            pat: page.tc.nodes.some(
                (it) => it.kind === "pat" && it.code === "n" && it.ty === "Int",
            ),
            expr: page.tc.nodes.some(
                (it) =>
                    it.kind === "expr" && it.code === "5" && it.ty === "Int",
            ),
            errors: page.tc.errors,
        }),
        { main: true, aux: true, bodies: 3, pat: true, expr: true, errors: 0 },
    ],
    [
        "a row of the types is a type of a piece of the code, and marks it in the editor",
        (page) => ({
            count: page.tcHover.count,
            marked: page.tcHover.marked !== "",
        }),
        { count: 1, marked: true },
    ],

    [
        // The example holds three bodies, and the choice in `fib-aux` is what makes one of
        // them more than one block: the entry is the block that branches, every arm writes
        // the slot the expression is, and the value is read where the arms meet.
        "the cfg tab reads a body of the buffer as its blocks",
        (page) => ({
            form: page.mir.form,
            fib: page.mir.owners.some((it) => it.includes("fun fib")),
            main: page.mir.owners.some((it) => it.includes("fun main")),
            blocks: page.mir.blocks.length,
            entry: page.mir.entry,
            branches: page.mir.branches,
            cast: page.mir.stmts.some((it) => it.includes("const 5")),
            returns: page.mir.term.some((it) => it.startsWith("return l")),
            params: page.mir.blockParams.length,
        }),
        {
            form: "cfg",
            fib: true,
            main: true,
            blocks: 6,
            entry: 3,
            branches: 1,
            cast: true,
            returns: true,
            params: 0,
        },
    ],
    [
        // The CFG form is the lowering as it leaves it: an assignment writes a slot, a
        // branch reads one, and a body with no choice in it ends by giving one back.
        "the cfg tab reads the slots of the lowering",
        (page) => ({
            stmts: page.mir.stmts.every((it) => /^l\d/.test(it)),
            terms: page.mir.term.every(
                (it) =>
                    it.startsWith("return l") ||
                    it.startsWith("branch l") ||
                    it.startsWith("goto b"),
            ),
        }),
        { stmts: true, terms: true },
    ],
    [
        "a line of the cfg marks the code it was read from",
        (page) => ({
            count: page.mirHover.count,
            marked: page.mirHover.marked.trim(),
        }),
        { count: 1, marked: "5" },
    ],
    [
        // The slots of the CFG form: the lowering binds every expression to one, and a slot
        // a pattern bound says the name it was bound under. The SSA form needs none.
        "the cfg tab reads the slots of the body, and the ssa tab has none",
        (page) => ({
            named: page.mir.locals.some((it) => it.includes("(n)")),
            slots: page.mir.locals.every((it) => /^l\d/.test(it)),
            none: page.ssa.locals.length,
        }),
        { named: true, slots: true, none: 0 },
    ],
    [
        // The SSA form is built from the CFG form: the same bodies, with a value of its own
        // in place of every slot, and the value `fib-aux` selects is born at the join its
        // arms branch into.
        "the ssa tab reads the same body with values in place of slots",
        (page) => ({
            form: page.ssa.form,
            blocks: page.ssa.blocks.length,
            cast: page.ssa.stmts.some((it) => it.includes("const 5")),
            values: page.ssa.stmts.every((it) => /^v\d/.test(it)),
            params: page.ssa.blockParams.length,
            returns: page.ssa.term.some((it) => it.startsWith("return v")),
        }),
        (page) => ({
            form: "ssa",
            blocks: page.mir.blocks.length,
            cast: true,
            values: true,
            params: 1,
            returns: true,
        }),
    ],
    [
        // The LIR is what the back end encodes: the same bodies, one instruction of the
        // target per line, and a local for every value that cannot be emitted where it is
        // read. The dispatch of `fib-aux` keeps its program counter in one of them.
        "the lir tab reads the target's instructions and where the values live",
        (page) => ({
            aux: page.lir.owners.some((it) => it.includes("fun fib-aux")),
            blocks: page.lir.blocks.length,
            entry: page.lir.entry,
            get: page.lir.kinds.includes("i31.get_s"),
            add: page.lir.kinds.includes("i32.add"),
            eq: page.lir.kinds.includes("i32.eq"),
            i31: page.lir.kinds.includes("ref.i31"),
            call: page.lir.kinds.includes("call"),
            branch: page.lir.termKinds.includes("branch"),
            returns: page.lir.term.some((it) => it.startsWith("return v")),
            params: page.lir.params.some((it) => it.includes("(ref i31)")),
        }),
        (page) => ({
            aux: true,
            blocks: page.mir.blocks.length,
            entry: 3,
            get: true,
            add: true,
            eq: true,
            i31: true,
            call: true,
            branch: true,
            returns: true,
            params: true,
        }),
    ],
    [
        // A value the allocation gave a local to says which one; the dispatch form would
        // keep a program counter in one of them, and every body of the buffer is structured.
        "the lir reads the locals a value lives in",
        (page) => ({
            named: page.lir.locals.some((it) => it.includes(" = v")),
            param: page.lir.blockParams.some((it) => it.includes("(local ")),
            pc: page.lir.locals.some((it) => it.includes("(pc)")),
        }),
        { named: true, param: true, pc: false },
    ],
    [
        // The structure is the tree of frames the encoder writes around the instructions:
        // a join is a `block` a branch leaves, an `if` is a choice, a `leaf` is where a
        // block is written, and what a person folds is a frame and what it holds.
        "the lir tab reads the structure the encoder writes",
        (page) => ({
            block: page.lir.structureKinds.includes("block"),
            if: page.lir.structureKinds.includes("if"),
            br: page.lir.structureKinds.includes("br"),
            leaf: page.lir.structureKinds.includes("leaf"),
            return: page.lir.structureKinds.includes("return"),
            frame: page.lir.structure.some((it) => it.startsWith("block b")),
            write: page.lir.structure.some((it) => it.startsWith("leaf b")),
            depths: Math.max(...page.lir.structureDepths),
        }),
        {
            block: true,
            if: true,
            br: true,
            leaf: true,
            return: true,
            frame: true,
            write: true,
            depths: greater(0),
        },
    ],
    [
        // The structure is a view rather than a note: a frame folds the lines it holds
        // away, and unfolding it brings them back.
        "the lir folds and unfolds a frame of the structure",
        (page) => ({
            folded: page.lirFolded.structure.length < page.lir.structure.length,
            again: page.lirAgain.structure.length === page.lir.structure.length,
        }),
        { folded: true, again: true },
    ],

    [
        // A choice is what makes a body more than one block: the entry evaluates the
        // condition and branches, every arm writes the slot the expression is and goes to
        // the block the arms meet in, and what is written after the `if` is written there.
        // The condition holds a truth value, which is read as the constant it is.
        "the cfg of a choice reads the blocks it branches into",
        (page) => ({
            form: page.branch.form,
            blocks: page.branch.blocks.length,
            branches: page.branch.branches,
            truth: page.branch.stmts.some((it) => it.includes("const true")),
            one: page.branch.stmts.some((it) => it.includes("const 1")),
            returns: page.branch.term.some((it) => it.startsWith("return l")),
            params: page.branch.blockParams.length,
        }),
        {
            form: "cfg",
            blocks: 4,
            branches: 1,
            truth: true,
            one: true,
            returns: true,
            params: 0,
        },
    ],
    [
        // The keywords of a choice are read out of names like every other keyword, and the
        // editor paints them the same way.
        "the editor paints the keywords of a choice as it paints fun",
        (page) => ({
            keyword: page.branchWords.keyword,
            if: page.branchWords.if,
            then: page.branchWords.then,
            else: page.branchWords.else,
        }),
        (page) => {
            const keyword = page.branchWords.keyword;

            return {
                keyword: not(""),
                if: keyword,
                then: keyword,
                else: keyword,
            };
        },
    ],
    [
        // A truth value is a literal like an integer: the words are keywords, and what a
        // reader reads at them is the value.
        "the editor paints a truth value as it paints a number",
        (page) => ({
            painted: page.branchWords.truth !== "",
            same: page.branchWords.truth === page.branchWords.number,
        }),
        { painted: true, same: true },
    ],
    [
        // The value the arms agree on is born at the join: the SSA form enters the block
        // they meet in through a parameter, and every arm passes its own value to it.
        "the ssa of a choice is entered through a parameter of the join",
        (page) => ({
            form: page.branchSsa.form,
            blocks: page.branchSsa.blocks.length,
            branches: page.branchSsa.branches,
            params: page.branchSsa.blockParams.length,
            returns: page.branchSsa.term.some((it) =>
                it.startsWith("return v"),
            ),
        }),
        (page) => ({
            form: "ssa",
            blocks: page.branch.blocks.length,
            branches: 1,
            params: 1,
            returns: true,
        }),
    ],
    [
        // A choice without an `else` selects no value: the block control falls into writes
        // the unit the choice is, and what is written after the `if` reads one slot.
        "the cfg of a choice without an else reads the unit it selects",
        (page) => ({
            form: page.unit.form,
            blocks: page.unit.blocks.length,
            branches: page.unit.branches,
            unit: page.unit.stmts.some((it) => it.includes("const unit")),
            returns: page.unit.term.some((it) => it.startsWith("return l")),
            params: page.unit.blockParams.length,
        }),
        {
            form: "cfg",
            blocks: 4,
            branches: 1,
            unit: true,
            returns: true,
            params: 0,
        },
    ],
    [
        "the ssa of a choice without an else passes the unit to the join",
        (page) => ({
            form: page.unitSsa.form,
            blocks: page.unitSsa.blocks.length,
            branches: page.unitSsa.branches,
            params: page.unitSsa.blockParams.length,
            returns: page.unitSsa.term.some((it) => it.startsWith("return v")),
        }),
        (page) => ({
            form: "ssa",
            blocks: page.unit.blocks.length,
            branches: 1,
            params: 1,
            returns: true,
        }),
    ],
    [
        // A lambda is a function of the module, read flat and named under the body that
        // wrote it; the closure that creates it points at the lifted function.
        "the cfg tab reads the body of a lambda as a function of the module",
        (page) => ({
            form: page.lambdaMir.form,
            main: page.lambdaMir.owners.some((it) => it.startsWith("fun main")),
            lambdas: page.lambdaMir.lambdas.length,
            first:
                page.lambdaMir.lambdas[0]?.startsWith(
                    "fun main::<mlkc@lambda-0>",
                ) ?? false,
            second:
                page.lambdaMir.lambdas[1]?.startsWith(
                    "fun main::<mlkc@lambda-1>",
                ) ?? false,
            closure: page.lambdaMir.stmts.some((it) =>
                it.includes("closure lambda#0"),
            ),
            blocks: page.lambdaMir.blocks.length,
        }),
        {
            form: "cfg",
            main: true,
            lambdas: 2,
            first: true,
            second: true,
            closure: true,
            blocks: 3,
        },
    ],
    [
        // A lifted lambda is a function of the module in everything but its name: the tab
        // reads its body, and the closure it makes is what `ref.func` and `struct.new` are.
        "the lir tab reads the body of a lifted lambda",
        (page) => ({
            lambdas: page.lambdaLir.lambdas.length,
            first:
                page.lambdaLir.lambdas[0]?.startsWith(
                    "fun main::<mlkc@lambda-0>",
                ) ?? false,
            second:
                page.lambdaLir.lambdas[1]?.startsWith(
                    "fun main::<mlkc@lambda-1>",
                ) ?? false,
            blocks: page.lambdaLir.blocks.length,
            func: page.lambdaLir.kinds.includes("ref.func"),
            struct: page.lambdaLir.kinds.includes("struct.new"),
            call: page.lambdaLir.kinds.includes("call-ref"),
        }),
        {
            lambdas: 2,
            first: true,
            second: true,
            blocks: 3,
            func: true,
            struct: true,
            call: true,
        },
    ],

    [
        "the editor marks what it reported",
        (page) => page.marks.marks,
        greater(0),
    ],
    ["a buffer can be made at a path", (page) => page.made.file, true],
    ["a buffer opens as a tab", (page) => page.made.open, 2],
    [
        "a tab closes without the file",
        (page) => ({ open: page.closed.open, listed: page.closed.listed }),
        { open: 2, listed: true },
    ],
    ["a buffer can be dropped", (page) => page.dropped.file, true],

    [
        // The WASM the buffer assembles to, read as text in a view of its own: the module
        // names itself, what it imports is what it calls, and the head says how much of it
        // there is --- and there is enough of it for the forms to fold.
        "the wat tab reads the module the buffer assembles to",
        (page) => ({
            shown: page.wat.shown,
            module: page.wat.text.includes("(module"),
            print: page.wat.text.includes("print-int"),
            head: /\d+ lines/.test(page.wat.head),
            gutters: page.wat.gutters > 0,
        }),
        { shown: true, module: true, print: true, head: true, gutters: true },
    ],
    [
        // The end of the module is read by scrolling to it, where the name section stands.
        "the name section of the module is read at the end of it",
        (page) => page.watEnd.text,
        includes("app::main"),
    ],
    [
        // Diagnostics and Config are in front whatever stage is shown: a buffer that
        // reports nothing says so while the WAT is in front, and the line is not the view
        // that is in front.
        "the diagnostics are read at a glance",
        (page) => ({
            shown: page.glance.shown,
            text: page.glance.text,
            active: page.glance.active,
        }),
        { shown: true, text: "no diagnostics", active: false },
    ],
    [
        // A fade stands at an end of the row of stages only when something is out of
        // sight on that side: the row is one line that scrolls, and the fade is how a
        // person knows that it does.
        "a fade says when a stage is out of sight",
        (page) =>
            page.fades.overflows
                ? page.fades.start || page.fades.end
                : !page.fades.start && !page.fades.end,
        true,
    ],
    [
        // The configuration of the compiler is a tab of the inspector rather than tools of
        // the header: it grows with the pipeline, and the options of a run are read where
        // a person sets them.
        "the compiler is configured in a tab of its own",
        (page) => page.config,
        {
            debug: "source-map",
            opt: "none",
            options: "none source-map dwarf-lines dwarf-full",
        },
    ],
    [
        // The debug option is what a module carries: the source map of a browser, the
        // tables of DWARF, or nothing but the name section. A module carries one of them,
        // so an engine has no DWARF to prefer to the map (ADR-0025).
        "the debug option decides what a module carries",
        (page) => ({
            dwarf: page.dwarfWat.sections,
            bare: page.bareWat.sections,
            map: page.mapWat.sections,
        }),
        {
            dwarf: allOf(
                includes("name"),
                includes(".debug_line"),
                includes(".debug_info"),
                includes(".debug_abbrev"),
                not(includes("sourceMappingURL")),
            ),
            bare: allOf(
                includes("name"),
                none("a .debug section", (it) => it.startsWith(".debug")),
                not(includes("sourceMappingURL")),
            ),
            map: allOf(
                includes("sourceMappingURL"),
                none("a .debug section", (it) => it.startsWith(".debug")),
            ),
        },
    ],
    [
        // The format is painted: an instruction is the accent of a keyword, a value type
        // the colour of a type, and a quoted name the green of a string.
        "the format is painted in the colours of the theme",
        (page) => ({
            keyword: page.watPaint.keyword !== "",
            accent: page.watPaint.keyword === page.watPaint.accent,
            type: page.watPaint.type !== "",
            typeColour: page.watPaint.type === page.watPaint.typeColour,
            string: page.watPaint.string !== "",
            green: page.watPaint.string === page.watPaint.ok,
        }),
        {
            keyword: true,
            accent: true,
            type: true,
            typeColour: true,
            string: true,
            green: true,
        },
    ],
    [
        // A marker belongs to the line a form opens on: the module and the head of each
        // `func` fold, and a line of a body does not --- a marker on `local.get $n` was a
        // bug of the scan reading a line's parenthesis from a later line.
        "a fold marker belongs to the line a form opens on",
        (page) => ({
            marks: page.watMarks.marks,
            opens: page.watMarks.list.every((it) => it.startsWith("(")),
            module: page.watMarks.moduleMarked,
            head: page.watMarks.headMarked,
            body: page.watMarks.bodyMarked,
        }),
        { marks: 3, opens: true, module: true, head: true, body: false },
    ],
    [
        // Folding a form takes its body off the screen and leaves the placeholder where it
        // was, inside the form: the head and the parenthesis that closes it stay, so a form
        // reads as `(module $app::main…)` rather than as a head and a line of its own.
        "a form of the module folds, and unfolds again",
        (page) => ({
            folded: page.watFolded.marks > 0,
            fewer: page.watFolded.lines < page.wat.lines,
            placeholder: page.watFolded.text.includes("…"),
            again: page.watAgain.marks === 0,
            back: page.watAgain.lines > page.watFolded.lines,
        }),
        {
            folded: true,
            fewer: true,
            placeholder: true,
            again: true,
            back: true,
        },
    ],
    [
        // The whole program: every buffer compiled and linked, the modules instantiated,
        // and the entry point called. `fib(5)` is 5, and 5 is what was printed.
        "running the program prints what it computes",
        (page) => ({ tab: page.ran.tab, printed: page.ran.printed }),
        { tab: true, printed: includes("5") },
    ],
    [
        // The header is the tools of the project, and the name of the buffer is not repeated
        // in it: the tab of the editor says which buffer is in front. A tool is a mark and a
        // word, and the run is the one painted as the action.
        "the header holds the tools of the project, and not the name of the buffer",
        (page) => ({
            brand: page.header.brand,
            name: page.header.name,
            tools: page.header.tools.map((it) => it.label).join(" "),
            count: page.header.tools.length,
            marked: page.header.tools.every((it) => it.mark && it.aria !== ""),
            primary: page.header.tools[3]?.primary ?? false,
            others: page.header.tools.slice(0, 3).every((it) => !it.primary),
        }),
        {
            brand: "MLK",
            name: false,
            tools: "Check Compile Sources Run",
            count: 4,
            marked: true,
            primary: true,
            others: true,
        },
    ],
    [
        // The chords are written where a pointer reads them without pressing either tool:
        // a synthetic key is not sent here, because the engine this check drives hands one
        // to the page's capture listener only sometimes, and the promise of the tools is
        // what is read instead.
        "the tools say which keys ask for them",
        (page) => ({
            check:
                page.header.tools[0]?.title.includes("Ctrl+Shift+Enter") ??
                false,
            run: page.header.tools[3]?.title.includes("Ctrl+Enter") ?? false,
        }),
        { check: true, run: true },
    ],
    [
        // A build is one archive rather than a file per module: a download cannot make a
        // folder, and an archive is where the folders of a project survive. Every module is
        // in it under the file its manifest names, with the module of the host functions
        // and the manifest itself beside it, and the archive is named by the project.
        "a build is handed over as one archive of the program",
        (page) => ({
            saved: page.built.saved.length,
            type: page.built.saved[0]?.type ?? "",
            sized: (page.built.saved[0]?.size ?? 0) > 0,
            files: page.built.files.map((it) => it.name).join(" "),
            blob: page.built.files[0]?.blob ?? false,
            entries: [...page.built.entries].sort().join(" "),
            said: page.built.lines.some((it) =>
                it.includes("the build is 6 files in app.zip"),
            ),
        }),
        {
            saved: 1,
            type: "application/zip",
            sized: true,
            files: "app.zip",
            blob: true,
            entries:
                "app/main.wasm host.wasm manifest.json std/core.wasm std/prelude.wasm std/runtime.wasm",
            said: true,
        },
    ],
    [
        // The manifest is what a host that reads the build from files reads: the project,
        // the file every module is written as, and where the program begins.
        "the manifest of the build travels beside the modules",
        (page) => ({
            project: page.built.manifest.project,
            host: page.built.manifest.host,
            module: page.built.manifest.entry.module,
            name: page.built.manifest.entry.name,
            modules: page.built.manifest.modules
                .map((it) => it.file)
                .sort()
                .join(" "),
            print:
                page.built.manifest.modules
                    .find((it) => it.name === "app::main")
                    ?.imports.some(
                        (it) =>
                            it.external &&
                            it.module === "std::runtime" &&
                            it.name === "print-int",
                    ) ?? false,
        }),
        {
            project: "app",
            host: "host.wasm",
            module: "app::main",
            name: "main",
            modules:
                "app/main.wasm std/core.wasm std/prelude.wasm std/runtime.wasm",
            print: true,
        },
    ],
    [
        // The sources are handed over as an archive of their own, and its name says which
        // build it belongs to. An entry stands where the debug information of a module reads
        // its path, so unpacking the archive and mapping `/` to the directory finds every
        // source, the library included.
        "the sources are handed over as one archive, under the paths of the debug information",
        (page) => ({
            files: page.sources.files.length,
            name: page.sources.files[1]?.name ?? "",
            blob: page.sources.files[1]?.blob ?? false,
            saved: page.sources.saved.length,
            entries: [...page.sources.entries].sort().join(" "),
            main: page.sources.main.includes("fun fib"),
            said: page.sources.lines.some((it) =>
                it.includes("the sources are 4 files in app.sources.zip"),
            ),
        }),
        {
            files: 2,
            name: "app.sources.zip",
            blob: true,
            saved: 2,
            entries: "main.mlk std/core.mlk std/prelude.mlk std/runtime.mlk",
            main: true,
            said: true,
        },
    ],

    [
        "the console has a program tab",
        (page) => ({ tab: page.program.tab, empty: page.program.empty }),
        { tab: true, empty: true },
    ],
    ["a panel can be sized", (page) => page.width.files, greater(220)],

    [
        "a node the grammar has no room for is shown as a node",
        (page) => page.bogus.shown,
        true,
    ],
    [
        "nothing the ast shows reads as an object",
        (page) => page.bogus.object,
        false,
    ],
    [
        "a node the grammar has no room for marks what it holds",
        (page) => ({
            count: page.bogusMark.count > 0,
            marked: page.bogusMark.marked.includes("abc"),
        }),
        { count: true, marked: true },
    ],
    [
        "a broken buffer reports a diagnostic",
        (page) => page.broken?.length ?? 0,
        greater(0),
    ],
    [
        // The status line counts what the list holds, and it is picked: the line is the
        // diagnostics view, and the count says how bad the buffer is.
        "the status line counts the diagnostics",
        (page) => ({
            text: page.brokenGlance.text,
            active: page.brokenGlance.active,
        }),
        { text: includes("error"), active: true },
    ],
    [
        // A project with a mistake in it is not a project to build: the tool is read rather
        // than pressed, and what it says is the mistake.
        "a project with a mistake in it offers no build",
        (page) => ({
            disabled: page.compileGuard.disabled,
            title: page.compileGuard.title.includes("error"),
        }),
        { disabled: true, title: true },
    ],
    [
        "the diagnostic is an error",
        (page) => page.broken?.[0]?.classes ?? [],
        includes("error"),
    ],
    [
        // The code a person reads leads with the level, then says the stage and the kind:
        // this one is the parser's first, and the parser is the second stage of the pipeline.
        "the code says the stage and the kind",
        (page) => page.broken?.[0]?.code ?? "",
        matches(/^E0201$/),
    ],
    [
        "the diagnostic shows the line it is about",
        (page) => ({
            number: page.broken?.[0]?.number ?? "",
            caret: page.broken?.[0]?.caret ?? "",
        }),
        { number: matches(/^\d+$/), caret: includes("^") },
    ],

    [
        "a wide screen draws the panels together, with no switcher to pick one",
        (page) => ({ width: page.wide.width, switches: page.wide.switches }),
        { width: atLeast(860), switches: 0 },
    ],
    [
        "a phone shows one panel at a time, and the switcher is how it is picked",
        (page) => ({
            width: page.phone.width,
            shown: page.phone.shown,
            switches: page.phone.switches,
            handles: page.phone.handles,
            overflows: page.phone.overflows,
        }),
        {
            width: less(860),
            shown: "editor",
            switches: 4,
            handles: 0,
            overflows: false,
        },
    ],
    [
        "picking Files shows the files",
        (page) => ({
            shown: page.phoneFiles.shown,
            files: page.phoneFiles.files,
            overflows: page.phoneFiles.overflows,
        }),
        { shown: "files", files: 4, overflows: false },
    ],
    [
        "picking a buffer shows the editor, with the buffer in front",
        (page) => ({
            shown: page.phoneBuffer.shown,
            file: page.phoneBuffer.file,
            overflows: page.phoneBuffer.overflows,
        }),
        { shown: "editor", file: "main.mlk", overflows: false },
    ],
    [
        "picking Inspect shows the inspector",
        (page) => ({
            shown: page.phoneInspector.shown,
            tabs: page.phoneInspector.tabs,
            overflows: page.phoneInspector.overflows,
        }),
        { shown: "inspector", tabs: 11, overflows: false },
    ],
    [
        "picking Console shows the console",
        (page) => ({
            shown: page.phoneConsole.shown,
            lines: page.phoneConsole.lines,
            overflows: page.phoneConsole.overflows,
        }),
        { shown: "console", lines: true, overflows: false },
    ],
    [
        "picking a token of a tree shows the editor, with the token marked in it",
        (page) => ({
            shown: page.phoneToken.shown,
            marked: page.phoneToken.marked !== "",
            same: page.phoneToken.marked === page.phoneToken.says,
        }),
        { shown: "editor", marked: true, same: true },
    ],
    [
        // A place is brought to a person, not merely marked: a buffer is read through a
        // window onto it, and a pick moves that window. A browser that lays the editor out
        // with nothing to scroll — the one this check drives is one — has no window to move,
        // and is asked for the mark alone.
        "picking a token brings the place it stands for into view",
        (page) => !page.phoneToken.scrolls || page.phoneToken.scrolled,
        true,
    ],

    [
        // The library is the compiler's: it is in the files with everything else, and the
        // directory of the library is what says so --- once, for everything under it.
        "the standard library is in the files, and offers nothing to drop",
        (page) => ({
            core: page.library.files.includes("/std/core.mlk"),
            prelude: page.library.files.includes("/std/prelude.mlk"),
            drop: page.library.drop,
            file: page.library.file,
            locked: page.library.locked,
        }),
        {
            core: true,
            prelude: true,
            drop: false,
            file: "",
            locked: "read-only",
        },
    ],
    [
        // What the editor shows of a file of the library is what the compiler holds: a
        // state that is read-only takes no text, and the tab says what the file is.
        "a file of the library is read, and not written in",
        (page) => ({
            editable: page.readLibrary.editable,
            tab: page.readLibrary.tab,
            core: page.readLibrary.text.includes("module project::core"),
            same: page.writtenLibrary.text === page.readLibrary.text,
        }),
        { editable: "false", tab: "r/o", core: true, same: true },
    ],
    [
        "a buffer cannot be made in the directory of the library",
        (page) => ({
            problem: page.refusedStdPath.problem,
            made: page.refusedStdPath.made,
        }),
        { problem: "the standard library is read-only", made: false },
    ],

    [
        // An edit of a body is read out of what the driver held: the parse of the edited
        // buffer is a stale read and never a miss, nothing that was held was dropped, and a
        // buffer that was dropped before is not read at all --- a host that says a file is
        // gone takes its module out of the project, and the project is not read over it.
        // How many looks the two edits are read in is the editor's to decide, so what is
        // asked is that the edit was read rather than that it was read exactly once.
        "an edit of a body is paid for out of what the driver held",
        (page) => ({
            stales: counted(page, "parse", "stales", "/main.mlk"),
            misses: counted(page, "parse", "misses", "/main.mlk"),
            lib: page.stats.rows.every((it) => !it.unit.startsWith("/lib/")),
            kept: page.stats.rows.every((it) => it.dropped === 0),
        }),
        { stales: greater(0), misses: 0, lib: true, kept: true },
    ],
    [
        // What a module shows is a function of its own text and of no body, so the edit
        // reads the surface of it again and keeps the types it had.
        "the signature surface of an edited module is read again and kept",
        (page) => ({
            stales: counted(page, "signatures", "stales", "/main.mlk"),
            kept: counted(page, "signatures", "kept", "/main.mlk"),
        }),
        { stales: greater(0), kept: greater(0) },
    ],
    [
        "the body that was edited is checked again",
        (page) => counted(page, "check", "stales"),
        greater(0),
    ],
    [
        // A check is a value of a body, and the counters say which body: the name of the
        // entity it belongs to and the file it is written in.
        "a check is counted for the body it is about",
        (page) =>
            page.stats.rows.some(
                (it) => it.pass === "check" && it.unit === "/main.mlk: main",
            ),
        true,
    ],
    [
        // What a look cost is the time of the page around it, and every row carries what
        // its pass cost. The times of the rows are zero in the browser this check drives
        // --- its clock reads the same value twice within one task, and a pull is one task
        // --- which is why the time of a pass is what the tests of the driver measure,
        // with a clock they control.
        "a look is timed, and its rows carry the time of their passes",
        (page) => ({
            took: page.stats.took > 0,
            finite: page.stats.rows.every(
                (it) => Number.isFinite(it.took) && it.took >= 0,
            ),
        }),
        { took: true, finite: true },
    ],

    [
        "the page said nothing it should not have",
        (page, found) => found.problems,
        [],
    ],
];

/**
 * What the run saw, said the way a person reads it.
 *
 * A note is a function of the captures rather than a line of output: a step that could not
 * run leaves its notes nothing to read, and a note is not a check --- it is skipped rather
 * than reported.
 */
const NOTES = /** @type {((page: Page) => string)[]} */ ([
    (page) => `the page at ${base} shows: ${page.state.file || "(nothing)"}`,
    (page) => `the cst holds ${page.cst.nodes} elements`,
    (page) =>
        `the editor paints a keyword ${page.painted.keyword}, a number ${page.painted.number}, a type ${page.painted.type}`,
    (page) =>
        `it paints pub ${page.words.pub}, use ${page.words.use} and as ${page.words.as}, against fun ${page.words.keyword}`,
    (page) =>
        `it paints an attribute ${page.painted.attribute}, which is the green of the theme ${page.painted.green}`,
    (page) =>
        `it paints a truth value ${page.branchWords.truth || "(nothing)"}, against a number ${page.branchWords.number || "(nothing)"} and a keyword ${page.branchWords.keyword || "(nothing)"}`,
    (page) =>
        `the row that says ${page.hover.says} marks ${page.hover.marked || "nothing"} in the editor`,
    (page) =>
        `a row of the ast marks ${JSON.stringify(page.astHover.marked.slice(0, 40))}`,
    (page) =>
        `the hir reads ${page.hir.module}, and ${page.hirHover.says} marks ${JSON.stringify(page.hirHover.marked)}`,
    (page) =>
        `the path ${page.pathHover.says} marks ${JSON.stringify(page.pathHover.at)} and ${JSON.stringify(page.pathHover.names)}`,
    (page) =>
        `the type line ${page.typeHover.says} marks ${JSON.stringify(page.typeHover.at)} and is painted ${page.typeColour.part}`,
    (page) =>
        `the mir reads a ${page.mir.form} form of ${page.mir.blocks.length} block(s), which ${page.mirHover.says.trim()} marks ${JSON.stringify(page.mirHover.marked)}, and the ssa ${page.ssa.blocks.length} block(s)`,
    (page) =>
        `the lir reads ${page.lir.blocks.length} block(s) of ${page.lir.kinds.length} instruction(s) of ${new Set(page.lir.kinds).size} kind(s), in ${page.lir.locals.length} local line(s), and the structure is ${page.lir.structure.length} line(s) folded to ${page.lirFolded.structure.length}`,
    (page) =>
        `a choice reads as ${page.branch.blocks.length} block(s) of a ${page.branch.form} form and ${page.branchSsa.blockParams.length} parameter(s) of the join of the ssa form`,
    (page) =>
        `a choice without an else reads ${page.unit.stmts.filter((it) => it.includes("const unit")).length} unit(s) in its ${page.unit.blocks.length} block(s)`,
    (page) =>
        `a lambda reads as ${JSON.stringify(page.lambdaMir.lambdas)} in the mir and ${JSON.stringify(page.lambdaLir.lambdas)} in the lir, of ${page.lambdaMir.blocks.length} and ${page.lambdaLir.blocks.length} block(s)`,
    (page) =>
        `the module assembles to ${page.wat.head}, which ${page.watPaint.keyword === page.watPaint.accent ? "is" : "is NOT"} painted, and folds to ${JSON.stringify(page.watFolded.text.trim())}, and the program printed ${JSON.stringify(page.ran.printed)}`,
    (page) =>
        `the build is ${page.built.files.map((it) => it.name).join(", ") || "(nothing)"}, holding ${page.built.entries.join(", ")}`,
    (page) =>
        `the manifest of the build is of the project ${page.built.manifest.project}, and its entry is ${page.built.manifest.entry ? `${page.built.manifest.entry.module}::${page.built.manifest.entry.name}` : "nothing"}`,
    (page) =>
        `a broken buffer ${page.compileGuard.disabled ? "refuses" : "takes"} a build`,
    (page) =>
        `the debug option gives the module ${page.dwarfWat.sections.length} custom section(s) of DWARF, ${page.bareWat.sections.length} with none, and ${page.mapWat.sections.length} with the source map`,
    (page) =>
        `of ${page.watMarks.rows} line(s) on the screen, ${page.watMarks.marks} carry a fold marker: the module ${page.watMarks.moduleMarked ? "folds" : "does not fold"}, a func head ${page.watMarks.headMarked ? "folds" : "does not fold"}, and a line of a body ${page.watMarks.bodyMarked ? "FOLDS" : "does not fold"}`,
    (page) => `the markers stand at ${JSON.stringify(page.watMarks.list)}`,
    (page) =>
        `a node the grammar has no room for reads as ${page.bogus.shown ? "a node" : "nothing"}`,
    (page) =>
        `its elements mark ${JSON.stringify(page.bogusMark.marked.slice(0, 24))}`,
    (page) => `a broken buffer gives ${page.broken?.length ?? 0} diagnostic(s)`,
    (page) =>
        `a phone at ${page.phone.width}px shows ${page.phone.shown}, and the switcher has ${page.phone.switches} panels to pick`,
    (page) =>
        `the files hold ${page.library.files.length} buffers, and the editor ${page.writtenLibrary.text === page.readLibrary.text ? "did not take" : "TOOK"} what was typed into a file of the library`,
    (page) =>
        `a pick of ${JSON.stringify(page.phoneToken.says)} in a tree marks ${JSON.stringify(page.phoneToken.marked)}, which the editor ${page.phoneToken.scrolls ? (page.phoneToken.scrolled ? "is taken to" : "stays away from") : "has nothing to scroll to"}`,
    (page) =>
        `a read of an edited body took ${page.stats.took} ms, of which ${counted(page, "check", "stales")} check(s) read again and ${page.stats.rows.filter((it) => it.took > 0).length} pass(es) timed`,
    (page) =>
        `the rows of that read: ${page.stats.rows.map((it) => `${it.pass}(${it.unit}) ${it.hits}/${it.misses}/${it.stales}/${it.kept}/${it.dropped}`).join(" ")}`,
    (page) =>
        page.broken?.[0]?.whole
            ? `  ${page.broken[0].whole.trim().replace(/\s+/g, " ")}`
            : "",
]);

/**
 * What the counters of a read come to, by pass and by row: what a check of one assertion
 * asks about is stated over the rows and not over the rows of one pass. A unit is named
 * when the assertion is about the buffer a person edited and not about every buffer the
 * driver read around it.
 *
 * @param {Page} page
 * @param {string} pass
 * @param {"hits" | "misses" | "stales" | "kept" | "dropped" | "took"} field
 * @param {string} [unit]
 */
function counted(page, pass, field, unit) {
    return page.stats.rows
        .filter(
            (it) =>
                it.pass === pass && (unit === undefined || it.unit === unit),
        )
        .reduce((all, it) => all + (it[field] ?? 0), 0);
}

/** A CDP connection: commands are answered by id, events go to whoever listens. */
class Connection {
    /** @type {WebSocket} */
    #socket;

    /** @type {number} */
    #next = 1;

    /** @type {Map<number, {accept: (value: any) => void, reject: (error: Error) => void}>} */
    #pending = new Map();

    /** @type {Set<(message: any) => void>} */
    #listeners = new Set();

    /** @param {string} url @returns {Promise<Connection>} */
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

    /** @param {WebSocket} socket */
    constructor(socket) {
        this.#socket = socket;
        socket.addEventListener("message", (event) =>
            this.#receive(String(event.data)),
        );
    }

    /**
     * @param {string} method
     * @param {any} [params]
     * @param {string} [sessionId]
     * @returns {Promise<any>}
     */
    send(method, params = {}, sessionId) {
        const id = this.#next++;

        return new Promise((accept, reject) => {
            this.#pending.set(id, { accept, reject });
            this.#socket.send(
                JSON.stringify({ id, method, params, sessionId }),
            );
        });
    }

    /** @param {(message: any) => void} listener */
    on(listener) {
        this.#listeners.add(listener);
    }

    /** @param {string} raw */
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
    // What the check asks is a table, so the list is read without driving anything.
    if (options.list) {
        for (const [label] of selectedChecks()) console.log(label);

        process.exit(0);
    }

    if (!options.base) base = `http://127.0.0.1:${await freePort(4173)}`;

    await ensureStaticServer();
    await ensureBrowser();

    const { connection, session } = await openPage();

    /** @type {Found} */
    const found = { problems: [], warnings: [], asked: [], failed: [] };

    /** Everything the page said that it should not have. */
    connection.on((message) => {
        if (message.method === "Runtime.exceptionThrown") {
            found.problems.push(describe(message.params.exceptionDetails));
        }

        if (message.method === "Runtime.consoleAPICalled") {
            /** @type {any[]} */
            const args = message.params.args ?? [];
            const said = args
                .map((argument) => argument.value ?? argument.description ?? "")
                .join(" ");

            if (message.params.type === "error")
                found.problems.push(`console.error: ${said}`);
            else if (message.params.type === "warning")
                found.warnings.push(said);
        }

        if (message.method === "Log.entryAdded") {
            const entry = message.params.entry;

            if (entry.level === "error")
                found.problems.push(`${entry.source}: ${entry.text}`);
            else if (entry.level === "warning") found.warnings.push(entry.text);
        }

        /** What the page asked for: a page that does not run is usually a request. */
        if (message.method === "Network.responseReceived")
            found.asked.push(
                `${message.params.response.url} ${message.params.response.status}`,
            );
    });

    await connection.send("Runtime.enable", {}, session);
    await connection.send("Log.enable", {}, session);
    await connection.send("Page.enable", {}, session);
    await connection.send("Network.enable", {}, session);

    const loaded = deferred();
    connection.on((message) => {
        if (message.method === "Page.loadEventFired") loaded.accept(undefined);
    });

    await connection.send("Page.navigate", { url: `${base}/` }, session);
    await Promise.race([
        loaded.promise,
        timeout(20000, `the page at ${base} never finished loading`),
    ]);

    /** One question, and the answer the page gave. @param {string} body */
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

    /**
     * Waits until the page answers a question with yes.
     *
     * A pull of the compiler is answered a message later, and the inspector paints it a render
     * after that: what a step reads is read after this says it is there, rather than after a
     * while that happens to be long enough.
     *
     * @param {Wait} wait
     */
    const until = async (wait) => {
        const found = await waitFor(async () => await ask(wait.test), {
            timeout: wait.timeout ?? 20000,
            interval: 50,
        });

        if (!found) throw new Error(`the page never showed ${wait.what}`);
    };

    /** What the run captured, under the names the checks read. @type {Page} */
    const page = /** @type {Page} */ (/** @type {unknown} */ ({}));

    /** The captures are filled by name, which is the one thing their shape does not say. */
    const bag = /** @type {Record<string, any>} */ (
        /** @type {unknown} */ (page)
    );

    for (const [capture, step, wait, settings = {}] of RUN) {
        try {
            if (settings.device)
                await connection.send(
                    "Emulation.setDeviceMetricsOverride",
                    settings.device,
                    session,
                );

            if (wait) await until(wait);

            if (!step) continue;

            const answer = await ask(step);

            if (!capture) continue;

            const value = settings.raw ? answer : JSON.parse(answer);
            const kept = settings.transform
                ? settings.transform(value, page)
                : value;

            bag[capture] = settings.merge ? { ...bag[capture], ...kept } : kept;
        } catch (error) {
            found.failed.push({ capture, error: message(error) });

            // A page that never came up fails every step below it in the same way,
            // and saying so once is the whole of what can be said about it.
            if (!page.state) break;
        }
    }

    return report(page, found);
}

/** The checks to ask: all of them, or the ones a `--only` names by their wording. */
function selectedChecks() {
    const said = options.only;

    if (!said?.length) return CHECKS;

    const asked = said.map((it) => it.toLowerCase());

    return CHECKS.filter(([label]) => {
        return asked.some((it) => label.toLowerCase().includes(it));
    });
}

/**
 * What the run read, or what its reading makes of the captures.
 *
 * @param {any} reading
 * @param {Page} page
 * @param {Found} found
 */
function read(reading, page, found) {
    return typeof reading === "function" ? reading(page, found) : reading;
}

/**
 * What the run found, said the way a person reads it.
 *
 * @param {Page} page
 * @param {Found} found
 * @returns {boolean}
 */
function report(page, found) {
    // The shape of a capture says it is there, and a page that never came up has none: the
    // one branch that reads a capture without trusting the shape is this one.
    const state = /** @type {State | undefined} */ (
        /** @type {unknown} */ (page.state)
    );

    // A page that never came up fails everything below it in the same way,
    // and saying so once is the whole of what can be said about it.
    if (!state) {
        console.log(`the page at ${base} did not come up`);
        console.log(
            `what it answered: ${/** @type {any} */ (state)?.file || "(nothing)"}`,
        );
        console.log(
            `what it asked for:\n  ${[...new Set(found.asked)].join("\n  ")}`,
        );
        console.log(
            `console: ${[...found.problems, ...found.warnings].join(" | ")}`,
        );

        return false;
    }

    // A run whose checks are picked by a name is a run a person is looking at a check through,
    // and the notes of everything else are noise.
    if (!options.only?.length) {
        for (const note of NOTES) {
            try {
                const said = note(page);

                if (said) console.log(said);
            } catch {
                // A note about what a failed step would have read is skipped.
            }
        }
    }

    const checks = selectedChecks();

    if (checks.length === 0) {
        console.log(`no check matches ${options.only?.join(", ")}`);

        return false;
    }

    let held = 0;
    let broken = 0;

    for (const [label, seen, expected] of checks) {
        let lines;

        try {
            lines = departures(
                read(expected, page, found),
                read(seen, page, found),
            );
        } catch (error) {
            lines = [
                {
                    at: "",
                    expected: "a value",
                    actual: `could not be read: ${message(error)}`,
                },
            ];
        }

        if (lines.length === 0) {
            held++;
            console.log(`ok   ${label}`);

            continue;
        }

        broken++;
        console.log(`FAIL ${label}`);

        for (const line of lines)
            console.log(
                line.at
                    ? `  ${line.at}: expected ${line.expected}, saw ${line.actual}`
                    : `  expected ${line.expected}, saw ${line.actual}`,
            );
    }

    if (found.failed.length > 0) {
        console.log("steps that could not run:");

        for (const { capture, error } of found.failed)
            console.log(`  ${capture ?? "an action"}: ${error}`);
    }

    if (found.warnings.length > 0)
        console.log(`warnings:\n  ${found.warnings.join("\n  ")}`);

    if (found.problems.length > 0)
        console.log(`problems:\n  ${found.problems.join("\n  ")}`);

    console.log(`${checks.length} check(s): ${held} ok, ${broken} failed`);

    if (broken > 0 || found.failed.length > 0)
        console.log(
            `what it asked for:\n  ${[...new Set(found.asked)].join("\n  ")}`,
        );

    return (
        broken === 0 && found.failed.length === 0 && found.problems.length === 0
    );
}

/**
 * The page to drive: the browser names the session of a new page on the connection,
 * and everything said about that page carries the session it names.
 */
async function openPage() {
    const version = await json(`${cdp}/json/version`);
    const connection = await Connection.open(version.webSocketDebuggerUrl);

    const attached =
        /** @type {{promise: Promise<string>, accept: (value: string) => void}} */ (
            deferred()
        );
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

/** Starts a process of its own group, so that taking it down takes down what it spawned.
 *
 * @param {string} name
 * @param {string} command
 * @param {string[]} args
 */
function start(name, command, args) {
    const child = spawn(command, args, { cwd: root, detached: true });
    /** @type {string[]} */
    const said = [];

    child.stdout?.on("data", (chunk) => said.push(String(chunk)));
    child.stderr?.on("data", (chunk) => said.push(String(chunk)));

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

/** Whether something answers at a URL, which is all this needs to know about it.
 *
 * @param {string} url
 * @returns {Promise<boolean>}
 */
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
 *
 * @param {number} preferred
 * @returns {Promise<number>}
 */
async function freePort(preferred) {
    for (let port = preferred; port < preferred + 20; port++) {
        if (await available(port)) return port;
    }

    throw new Error(`every port from ${preferred} upwards is taken`);
}

/** @param {number} port @returns {Promise<boolean>} */
function available(port) {
    return new Promise((accept) => {
        const probe = createServer();

        probe.once("error", () => accept(false));
        probe.once("listening", () => probe.close(() => accept(true)));
        probe.listen(port, "127.0.0.1");
    });
}

/**
 * @param {() => Promise<boolean> | boolean} check
 * @param {{timeout?: number, interval?: number}} [settings]
 */
async function waitFor(check, { timeout: limit = 30000, interval = 250 } = {}) {
    const deadline = Date.now() + limit;

    while (Date.now() < deadline) {
        if (await check()) return true;
        await sleep(interval);
    }

    return false;
}

/** @param {number} ms */
function sleep(ms) {
    return new Promise((accept) => setTimeout(accept, ms));
}

/** @param {string} url */
async function json(url) {
    return await (await fetch(url)).json();
}

/**
 * @param {number} after
 * @param {string} message
 * @returns {Promise<never>}
 */
function timeout(after, message) {
    return new Promise((_, reject) =>
        setTimeout(() => reject(new Error(message)), after),
    );
}

/**
 * @template T
 * @returns {{promise: Promise<T>, accept: (value: T) => void}}
 */
function deferred() {
    /** @type {(value: T) => void} */
    let accept = () => {};
    const promise = new Promise((resolve) => {
        accept = resolve;
    });

    return { promise, accept };
}

/** What an error says, whatever kind of thing was thrown. @param {unknown} error */
function message(error) {
    return error instanceof Error ? error.message : String(error);
}

/** @param {any} details */
function describe(details) {
    return (
        details?.exception?.description ??
        details?.text ??
        JSON.stringify(details)
    );
}

/**
 * @param {string[]} argv
 * @returns {Options}
 */
function parseArguments(argv) {
    /** @type {Options} */
    const options = {};

    for (let index = 0; index < argv.length; index++) {
        const argument = argv[index];
        const [name, value] = argument.split("=");

        if (name === "--help" || name === "-h") {
            console.log(
                "usage: node scripts/browser-check.mjs [--base URL] [--cdp URL] [--keep] [--list] [--only TEXT]",
            );
            process.exit(0);
        }

        if (name === "--keep") options.keep = true;
        else if (name === "--list") options.list = true;
        else if (name === "--base") options.base = value ?? argv[++index];
        else if (name === "--cdp") options.cdp = value ?? argv[++index];
        else if (name === "--only") {
            options.only = [...(options.only ?? []), value ?? argv[++index]];
        } else throw new Error(`unknown argument ${argument}`);
    }

    return options;
}

let ok = false;

try {
    ok = await main();
} catch (error) {
    console.error(`the check could not run: ${message(error)}`);
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
            if (child.pid === undefined) {
                child.kill();

                continue;
            }

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
