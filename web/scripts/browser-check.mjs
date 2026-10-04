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
 * Pass `--base` to check a site that already runs — the dev server, say. What the check
 * drives by default is the built site, which hands the driver's worker over as one script
 * every host runs; a dev server hands it over as a module, and obscura runs a worker as a
 * classic script whatever its type says, so a dev server is one this browser cannot drive.
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
 * What a tab of the MIR reads: which form it shows, the bodies, and the lines of their blocks.
 *
 * The lines are read as a person reads them: the text of every statement and terminator, which
 * is what says whether the tab shows the CFG form (slots) or the SSA form (values of their own).
 */
const MIR = `const lines = (kind) => [...inspector().querySelectorAll('[data-line=' + kind + ']')].map((it) => text(it));
	return JSON.stringify({
		form: inspector().querySelector('[data-form]')?.dataset.form,
		owners: lines('owner'),
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
\tconst row = (element) => ({ pass: element.dataset.stats, unit: element.dataset.unit, ...Object.fromEntries([...element.querySelectorAll('[data-count]')]
\t\t.map((cell) => [cell.dataset.count, value(cell)])) });
\treturn JSON.stringify({
\t\ttook: Number(inspector().querySelector('[data-cost=took]').dataset.took),
\t\trows: [...inspector().querySelectorAll('[data-stats]')].map(row)
\t})`,

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
			string: colour('\"std::runtime\"'),
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

    // The program: every buffer is compiled and linked, the modules are instantiated, and the
    // entry point is called. `fib(5)` prints 5, which is what the program console holds.
    run: `document.querySelector('[data-run]').click(); return true`,

    ran: `const lines = [...document.querySelectorAll('[data-panel=console] .line')];
		return JSON.stringify({
			tab: document.querySelector('[data-console=program].active') !== null,
			printed: lines.map((it) => text(it).trim())
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

/**
 * What a step waits to see before it reads what it is about.
 *
 * A pull of the compiler is a question the page asks the worker, and the inspector paints the
 * answer a message later rather than in the click that asked for it: what a step is about to
 * read is only there once the tab holds it. Each of these is the thing the step reads, asked
 * as a question until it answers yes.
 */
const WAITS = {
    cst: `return inspector().querySelector('[data-kind=MODULE_ROOT]') !== null`,
    ast: `return inspector().textContent.includes('ModuleRoot')`,
    hir: `return inspector().textContent.includes('BODY fun main')`,
    tc: `return inspector().querySelector('[data-tc=entity]') !== null`,
    mir: `return inspector().querySelector('[data-form=cfg] [data-block]') !== null`,
    ssa: `return inspector().querySelector('[data-form=ssa] [data-line=term]') !== null`,
    lir: `return inspector().querySelector('[data-lir] [data-block]') !== null && inspector().querySelector('[data-lir] [data-structure] [data-line=structure]') !== null`,
    branch: `return inspector().querySelectorAll('[data-form=cfg] [data-block]').length > 1`,
    branchSsa: `return inspector().querySelector('[data-form=ssa] [data-kind=branch]') !== null`,
    unit: `return inspector().textContent.includes('const unit')`,
    unitSsa: `return inspector().textContent.includes('call fun log')`,
    wat: `const view = inspector().querySelector('[data-wat]');
		return view !== null && view.querySelector('.cm-content') !== null`,
    debugTables: `const head = inspector().querySelector('[data-wat-sections]');
		return head !== null && head.textContent.includes('.debug_line')`,
    debugMap: `const head = inspector().querySelector('[data-wat-sections]');
		return head !== null && head.textContent.includes('sourceMappingURL')`,
    debugNone: `const head = inspector().querySelector('[data-wat-sections]');
		return head !== null && !head.textContent.includes('sourceMappingURL') &&
			!head.textContent.includes('.debug')`,
    config: `return document.querySelector('[data-debug]') !== null &&
		document.querySelector('[data-opt]') !== null`,
    ran: `const lines = [...document.querySelectorAll('[data-panel=console] .line')];
		return document.querySelector('[data-console=program].active') !== null &&
			lines.some((it) => text(it).trim() === '5')`,
    broken: `return diagnostics().length > 0`,
    bogus: `return inspector().textContent.includes('Bogus')`,
    long: `return inspector().textContent.includes('x79')`,
    edited: `return inspector().textContent.includes('80')`,
    editedAgain: `return inspector().textContent.includes('90')`,
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

    /**
     * Waits until the page answers a question with yes.
     *
     * A pull of the compiler is answered a message later, and the inspector paints it a render
     * after that: what a step reads is read after this says it is there, rather than after a
     * while that happens to be long enough.
     */
    const until = async (body, what) => {
        const found = await waitFor(async () => await ask(body), {
            timeout: 20000,
            interval: 50,
        });

        if (!found) throw new Error(`the page never showed ${what}`);
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
    await until(WAITS.cst, "the cst of the buffer");
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
    await until(WAITS.ast, "the ast of the buffer");
    const ast = JSON.parse(await ask(STEPS.ast));

    // A node of the typed tree covers the tokens under it, and a pointer on its row marks as much.
    await ask(STEPS.hoverAst);
    const astHover = JSON.parse(await ask(STEPS.astHovered));

    await ask(STEPS.showHir);
    await until(WAITS.hir, "the hir of the buffer");
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

    // The types of the buffer: what the checker resolved the surface to, and what it checked
    // every node of every body to. A row of the tab stands for a node of the HIR, so a pointer
    // on one marks the code the node was read from.
    await ask(STEPS.showTc);
    await until(WAITS.tc, "the types of the buffer");
    const tc = JSON.parse(await ask(STEPS.tc));
    await ask(STEPS.hoverTc);
    const tcHover = JSON.parse(await ask(STEPS.tcHovered));

    // The MIR of the buffer: the CFG form the checked body is lowered into, and the SSA form
    // built from it. A line of a body stands for the expression it was read from, so a pointer
    // on one asks the editor to mark that code.
    await ask(STEPS.showMir);
    await until(WAITS.mir, "the mir of the buffer");
    const mir = JSON.parse(await ask(STEPS.mir));

    const mirSays = JSON.parse(await ask(STEPS.hoverMir));
    const mirHover = {
        ...mirSays,
        ...JSON.parse(await ask(STEPS.mirHovered)),
    };

    await ask(STEPS.showSsa);
    await until(WAITS.ssa, "the ssa form of the buffer");
    const ssa = JSON.parse(await ask(STEPS.ssa));

    // The LIR of the buffer: what the WASM back end lowers the SSA form into, the target's own
    // instructions, where the values that need storage live, and the frames the encoder writes
    // them as.
    await ask(STEPS.showLir);
    await until(WAITS.lir, "the lir of the buffer");
    const lir = JSON.parse(await ask(STEPS.lir));

    // A frame of the structure folds what it holds away, and unfolding brings the lines back.
    await ask(STEPS.foldLir);
    const lirFolded = JSON.parse(await ask(STEPS.lir));

    await ask(STEPS.unfoldLir);
    const lirAgain = JSON.parse(await ask(STEPS.lir));

    // The module the link stage hands a host: the same bytes a run instantiates, read in a view
    // of its own, where the forms of it fold.
    await ask(STEPS.showWat);
    await until(WAITS.wat, "the wasm of the buffer");
    const wat = JSON.parse(await ask(STEPS.wat));
    const watPaint = JSON.parse(await ask(STEPS.watPaint));
    const watMarks = JSON.parse(await ask(STEPS.watMarks));

    await ask(STEPS.foldWat);
    const watFolded = JSON.parse(await ask(STEPS.wat));

    await ask(STEPS.unfoldWat);
    const watAgain = JSON.parse(await ask(STEPS.wat));

    // The name section of the module stands at the end of it: reading it is scrolling to it.
    await ask(STEPS.scrollWatEnd);
    await sleep(200);
    const watEnd = JSON.parse(await ask(STEPS.wat));

    // The configuration of the compiler is a tab of its own: the source map of a browser, the
    // tables of DWARF, and nothing are alternatives, and a module carries one of them. A change
    // is made in the config tab and read off the WAT of the module, which is where the head of
    // the tab lists what the option added.
    await ask(STEPS.showConfig);
    await until(WAITS.config, "the configuration of the compiler");
    const config = JSON.parse(await ask(STEPS.config));

    await ask(STEPS.debugDwarf);
    await ask(STEPS.showWat);
    await until(WAITS.debugTables, "the debug tables of the module");
    const dwarfWat = JSON.parse(await ask(STEPS.watSections));

    await ask(STEPS.showConfig);
    await until(WAITS.config, "the configuration of the compiler");
    await ask(STEPS.debugNone);
    await ask(STEPS.showWat);
    await until(WAITS.debugNone, "the module without debug information");
    const bareWat = JSON.parse(await ask(STEPS.watSections));

    await ask(STEPS.showConfig);
    await until(WAITS.config, "the configuration of the compiler");
    await ask(STEPS.debugMap);
    await ask(STEPS.showWat);
    await until(WAITS.debugMap, "the source map of the module");
    const mapWat = JSON.parse(await ask(STEPS.watSections));

    // And the program itself: the modules are instantiated in the order the link stage gives
    // them, and the `#[entry]` is called. A run of a browser is under the map, which is where
    // the option stands; what `fib(5)` prints is what the program console holds.
    await ask(STEPS.run);
    await until(WAITS.ran, "the program to run");
    const ran = JSON.parse(await ask(STEPS.ran));

    // A body with a choice in it: the CFG form reads the block that branches and the blocks the
    // arms meet in, and the SSA form gives the value the arms agree on a parameter of the block
    // they meet in. The buffer is typed into the one that is in front.
    await ask(STEPS.typeBranch);
    await sleep(300);
    const branchWords = JSON.parse(await ask(STEPS.branchWords));

    await ask(STEPS.showMir);
    await until(WAITS.branch, "the branches of the mir");
    const branch = JSON.parse(await ask(STEPS.mir));

    await ask(STEPS.showSsa);
    await until(WAITS.branchSsa, "the branches of the ssa form");
    const branchSsa = JSON.parse(await ask(STEPS.ssa));

    // A choice that selects nothing: the block control falls into when no condition holds
    // writes the unit the choice is, and the SSA form passes it to the join.
    await ask(STEPS.typeUnit);
    await sleep(300);

    await ask(STEPS.showMir);
    await until(WAITS.unit, "the unit of a choice that selects nothing");
    const unit = JSON.parse(await ask(STEPS.mir));

    await ask(STEPS.showSsa);
    await until(WAITS.unitSsa, "the unit of the ssa form");
    const unitSsa = JSON.parse(await ask(STEPS.ssa));

    await ask(STEPS.open);
    await ask(STEPS.typePath);
    await ask(STEPS.submitPath);
    const made = JSON.parse(await ask(STEPS.made));

    // Type into the buffer that was just made, the way a person would.
    await ask(STEPS.type);
    await sleep(300);

    await ask(STEPS.showDiagnostics);
    await until(WAITS.broken, "the diagnostics of the broken buffer");
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
    await until(WAITS.bogus, "the node the grammar has no room for");
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
    await until(WAITS.long, "the tree of the long buffer");
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

    // What a read cost: the buffer in front is filled with a module, and then the body of it
    // is edited, which is one value read again and nothing else. The look before the edit is
    // the one the second is read against: it is what the driver holds afterwards.
    await ask(STEPS.pickBuffer);
    await ask(STEPS.typeEdited);
    await until(WAITS.edited, "the tree of the edited buffer");
    await ask(STEPS.typeEditedAgain);
    await until(WAITS.editedAgain, "the tree of the buffer edited again");
    await ask(STEPS.showStats);
    const stats = JSON.parse(await ask(STEPS.stats));

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
            tc,
            tcHover,
            mir,
            mirHover,
            ssa,
            lir,
            lirFolded,
            lirAgain,
            branch,
            branchSsa,
            branchWords,
            unit,
            unitSsa,
            wat,
            watPaint,
            watMarks,
            watFolded,
            watAgain,
            watEnd,
            dwarfWat,
            bareWat,
            mapWat,
            config,
            ran,
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
            stats,
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

    // What the counters of a read come to, by pass and by row: what a check of one assertion
    // asks about is stated over the rows and not over the rows of one pass. A unit is named
    // when the assertion is about the buffer a person edited and not about every buffer the
    // driver read around it.
    const counted = (pass, field, unit) =>
        page.stats.rows
            .filter(
                (it) =>
                    it.pass === pass &&
                    (unit === undefined || it.unit === unit),
            )
            .reduce((all, it) => all + (it[field] ?? 0), 0);

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
            page.hirHover.says.includes("literal 5") &&
                page.hirHover.marked.trim() === "5",
        ],
        [
            "a path of the hir marks the code it is written as",
            page.pathHover.at === "fib",
        ],
        [
            "and marks what the path resolved to",
            // What a path leads to is the declaration it names, which is a place in the same
            // buffer: the path taken above is the call of `fib` in `main`.
            page.pathHover.names.includes("fun fib(n : Int) : Int"),
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
        [
            "the tc tab reads the surface of the module and the types of its bodies",
            page.tc.surface.some(
                (it) => it.name === "fun main" && it.ty === "() -> Unit",
            ) &&
                page.tc.surface.some(
                    (it) =>
                        it.name === "fun fib-aux" &&
                        it.ty === "(Int, Int, Int) -> Int",
                ) &&
                page.tc.bodies === 3 &&
                page.tc.nodes.some(
                    (it) =>
                        it.kind === "pat" && it.code === "n" && it.ty === "Int",
                ) &&
                page.tc.nodes.some(
                    (it) =>
                        it.kind === "expr" &&
                        it.code === "5" &&
                        it.ty === "Int",
                ) &&
                page.tc.errors === 0,
        ],
        [
            "a row of the types is a type of a piece of the code, and marks it in the editor",
            page.tcHover.count === 1 && page.tcHover.marked !== "",
        ],
        [
            // The example holds three bodies, and the choice in `fib-aux` is what makes one of
            // them more than one block: the entry is the block that branches, every arm writes
            // the slot the expression is, and the value is read where the arms meet.
            "the cfg tab reads a body of the buffer as its blocks",
            page.mir.form === "cfg" &&
                page.mir.owners.some((it) => it.includes("fun fib")) &&
                page.mir.owners.some((it) => it.includes("fun main")) &&
                page.mir.blocks.length === 6 &&
                page.mir.entry === 3 &&
                page.mir.branches === 1 &&
                page.mir.stmts.some((it) => it.includes("const 5")) &&
                page.mir.term.some((it) => it.startsWith("return l")) &&
                page.mir.blockParams.length === 0,
        ],
        [
            // The CFG form is the lowering as it leaves it: an assignment writes a slot, a
            // branch reads one, and a body with no choice in it ends by giving one back.
            "the cfg tab reads the slots of the lowering",
            page.mir.stmts.every((it) => /^l\d/.test(it)) &&
                page.mir.term.every(
                    (it) =>
                        it.startsWith("return l") ||
                        it.startsWith("branch l") ||
                        it.startsWith("goto b"),
                ),
        ],
        [
            "a line of the cfg marks the code it was read from",
            page.mirHover.count === 1 && page.mirHover.marked.trim() === "5",
        ],
        [
            // The slots of the CFG form: the lowering binds every expression to one, and a slot
            // a pattern bound says the name it was bound under. The SSA form needs none.
            "the cfg tab reads the slots of the body, and the ssa tab has none",
            page.mir.locals.some((it) => it.includes("(n)")) &&
                page.mir.locals.every((it) => /^l\d/.test(it)) &&
                page.ssa.locals.length === 0,
        ],
        [
            // The SSA form is built from the CFG form: the same bodies, with a value of its own
            // in place of every slot, and the value `fib-aux` selects is born at the join its
            // arms branch into.
            "the ssa tab reads the same body with values in place of slots",
            page.ssa.form === "ssa" &&
                page.ssa.blocks.length === page.mir.blocks.length &&
                page.ssa.stmts.some((it) => it.includes("const 5")) &&
                page.ssa.stmts.every((it) => /^v\d/.test(it)) &&
                page.ssa.blockParams.length === 1 &&
                page.ssa.term.some((it) => it.startsWith("return v")),
        ],
        [
            // The LIR is what the back end encodes: the same bodies, one instruction of the
            // target per line, and a local for every value that cannot be emitted where it is
            // read. The dispatch of `fib-aux` keeps its program counter in one of them.
            "the lir tab reads the target's instructions and where the values live",
            page.lir.owners.some((it) => it.includes("fun fib-aux")) &&
                page.lir.blocks.length === page.mir.blocks.length &&
                page.lir.entry === 3 &&
                page.lir.kinds.includes("i31.get_s") &&
                page.lir.kinds.includes("i32.add") &&
                page.lir.kinds.includes("i32.eq") &&
                page.lir.kinds.includes("ref.i31") &&
                page.lir.kinds.includes("call") &&
                page.lir.termKinds.includes("branch") &&
                page.lir.term.some((it) => it.startsWith("return v")) &&
                page.lir.params.some((it) => it.includes("(ref i31)")),
        ],
        [
            // A value the allocation gave a local to says which one; the dispatch form would
            // keep a program counter in one of them, and every body of the buffer is structured.
            "the lir reads the locals a value lives in",
            page.lir.locals.some((it) => it.includes(" = v")) &&
                page.lir.blockParams.some((it) => it.includes("(local ")) &&
                !page.lir.locals.some((it) => it.includes("(pc)")),
        ],
        [
            // The structure is the tree of frames the encoder writes around the instructions:
            // a join is a `block` a branch leaves, an `if` is a choice, a `leaf` is where a
            // block is written, and what a person folds is a frame and what it holds.
            "the lir tab reads the structure the encoder writes",
            page.lir.structureKinds.includes("block") &&
                page.lir.structureKinds.includes("if") &&
                page.lir.structureKinds.includes("br") &&
                page.lir.structureKinds.includes("leaf") &&
                page.lir.structureKinds.includes("return") &&
                page.lir.structure.some((it) => it.startsWith("block b")) &&
                page.lir.structure.some((it) => it.startsWith("leaf b")) &&
                Math.max(...page.lir.structureDepths) > 0,
        ],
        [
            // The structure is a view rather than a note: a frame folds the lines it holds
            // away, and unfolding it brings them back.
            "the lir folds and unfolds a frame of the structure",
            page.lirFolded.structure.length < page.lir.structure.length &&
                page.lirAgain.structure.length === page.lir.structure.length,
        ],
        [
            // A choice is what makes a body more than one block: the entry evaluates the
            // condition and branches, every arm writes the slot the expression is and goes to
            // the block the arms meet in, and what is written after the `if` is written there.
            // The condition holds a truth value, which is read as the constant it is.
            "the cfg of a choice reads the blocks it branches into",
            page.branch.form === "cfg" &&
                page.branch.blocks.length === 4 &&
                page.branch.branches === 1 &&
                page.branch.stmts.some((it) => it.includes("const true")) &&
                page.branch.stmts.some((it) => it.includes("const 1")) &&
                page.branch.term.some((it) => it.startsWith("return l")) &&
                page.branch.blockParams.length === 0,
        ],
        [
            // The keywords of a choice are read out of names like every other keyword, and the
            // editor paints them the same way.
            "the editor paints the keywords of a choice as it paints fun",
            page.branchWords.if !== "" &&
                page.branchWords.if === page.branchWords.keyword &&
                page.branchWords.then === page.branchWords.keyword &&
                page.branchWords.else === page.branchWords.keyword,
        ],
        [
            // A truth value is a literal like an integer: the words are keywords, and what a
            // reader reads at them is the value.
            "the editor paints a truth value as it paints a number",
            page.branchWords.truth !== "" &&
                page.branchWords.truth === page.branchWords.number,
        ],
        [
            // The value the arms agree on is born at the join: the SSA form enters the block
            // they meet in through a parameter, and every arm passes its own value to it.
            "the ssa of a choice is entered through a parameter of the join",
            page.branchSsa.form === "ssa" &&
                page.branchSsa.blocks.length === page.branch.blocks.length &&
                page.branchSsa.branches === 1 &&
                page.branchSsa.blockParams.length === 1 &&
                page.branchSsa.term.some((it) => it.startsWith("return v")),
        ],
        [
            // A choice without an `else` selects no value: the block control falls into writes
            // the unit the choice is, and what is written after the `if` reads one slot.
            "the cfg of a choice without an else reads the unit it selects",
            page.unit.form === "cfg" &&
                page.unit.blocks.length === 4 &&
                page.unit.branches === 1 &&
                page.unit.stmts.some((it) => it.includes("const unit")) &&
                page.unit.term.some((it) => it.startsWith("return l")) &&
                page.unit.blockParams.length === 0,
        ],
        [
            "the ssa of a choice without an else passes the unit to the join",
            page.unitSsa.form === "ssa" &&
                page.unitSsa.blocks.length === page.unit.blocks.length &&
                page.unitSsa.branches === 1 &&
                page.unitSsa.blockParams.length === 1 &&
                page.unitSsa.term.some((it) => it.startsWith("return v")),
        ],
        ["the editor marks what it reported", page.marks.marks > 0],
        ["a buffer can be made at a path", page.made.file],
        ["a buffer opens as a tab", page.made.open === 2],
        [
            "a tab closes without the file",
            page.closed.open === 2 && page.closed.listed,
        ],
        ["a buffer can be dropped", page.dropped.file],
        [
            // The WASM the buffer assembles to, read as text in a view of its own: the module
            // names itself, what it imports is what it calls, and the head says how much of it
            // there is — and there is enough of it for the forms to fold.
            "the wat tab reads the module the buffer assembles to",
            page.wat.shown &&
                page.wat.text.includes("(module") &&
                page.wat.text.includes("print-int") &&
                /\d+ lines/.test(page.wat.head) &&
                page.wat.gutters > 0,
        ],
        [
            // The end of the module is read by scrolling to it, where the name section stands.
            "the name section of the module is read at the end of it",
            page.watEnd.text.includes("app::main"),
        ],
        [
            // The configuration of the compiler is a tab of the inspector rather than tools of
            // the header: it grows with the pipeline, and the options of a run are read where
            // a person sets them.
            "the compiler is configured in a tab of its own",
            page.config.debug === "source-map" &&
                page.config.opt === "none" &&
                page.config.options ===
                    "none source-map dwarf-lines dwarf-full",
        ],
        [
            // The debug option is what a module carries: the source map of a browser, the
            // tables of DWARF, or nothing but the name section. A module carries one of them,
            // so an engine has no DWARF to prefer to the map (ADR-0025).
            "the debug option decides what a module carries",
            page.dwarfWat.sections.includes("name") &&
                page.dwarfWat.sections.includes(".debug_line") &&
                page.dwarfWat.sections.includes(".debug_info") &&
                page.dwarfWat.sections.includes(".debug_abbrev") &&
                !page.dwarfWat.sections.includes("sourceMappingURL") &&
                page.bareWat.sections.includes("name") &&
                !page.bareWat.sections.some((it) => it.startsWith(".debug")) &&
                !page.bareWat.sections.includes("sourceMappingURL") &&
                page.mapWat.sections.includes("sourceMappingURL") &&
                !page.mapWat.sections.some((it) => it.startsWith(".debug")),
        ],
        [
            // The format is painted: an instruction is the accent of a keyword, a value type
            // the colour of a type, and a quoted name the green of a string.
            "the format is painted in the colours of the theme",
            page.watPaint.keyword !== "" &&
                page.watPaint.keyword === page.watPaint.accent &&
                page.watPaint.type !== "" &&
                page.watPaint.type === page.watPaint.typeColour &&
                page.watPaint.string !== "" &&
                page.watPaint.string === page.watPaint.ok,
        ],
        [
            // A marker belongs to the line a form opens on: the module and the head of each
            // `func` fold, and a line of a body does not --- a marker on `local.get $n` was a
            // bug of the scan reading a line's parenthesis from a later line.
            "a fold marker belongs to the line a form opens on",
            page.watMarks.marks === 3 &&
                page.watMarks.list.every((it) => it.startsWith("(")) &&
                page.watMarks.moduleMarked &&
                page.watMarks.headMarked &&
                !page.watMarks.bodyMarked,
        ],
        [
            // Folding a form takes its body off the screen and leaves the placeholder where it
            // was, inside the form: the head and the parenthesis that closes it stay, so a form
            // reads as `(module $app::main…)` rather than as a head and a line of its own.
            "a form of the module folds, and unfolds again",
            page.watFolded.marks > 0 &&
                page.watFolded.lines < page.wat.lines &&
                page.watFolded.text.includes("…)") &&
                page.watAgain.marks === 0 &&
                page.watAgain.lines > page.watFolded.lines,
        ],
        [
            // The whole program: every buffer compiled and linked, the modules instantiated,
            // and the entry point called. `fib(5)` is 5, and 5 is what was printed.
            "running the program prints what it computes",
            page.ran.tab && page.ran.printed.includes("5"),
        ],
        [
            "the console has a program tab",
            page.program.tab && page.program.empty,
        ],
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
        // The code a person reads leads with the level, then says the stage and the kind:
        // this one is the parser's first, and the parser is the second stage of the pipeline.
        [
            "the code says the stage and the kind",
            /^E0201$/.test(diagnostic.code ?? ""),
        ],
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
                page.phoneFiles.files === 4 &&
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
                page.phoneInspector.tabs === 11 &&
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
        [
            // An edit of a body is read out of what the driver held: the parse of the edited
            // buffer is a stale read and never a miss, nothing that was held was dropped, and a
            // buffer that was dropped before is not read at all --- a host that says a file is
            // gone takes its module out of the project, and the project is not read over it.
            // How many looks the two edits are read in is the editor's to decide, so what is
            // asked is that the edit was read rather than that it was read exactly once.
            "an edit of a body is paid for out of what the driver held",
            counted("parse", "stales", "/main.mlk") >= 1 &&
                counted("parse", "misses", "/main.mlk") === 0 &&
                page.stats.rows.every((it) => !it.unit.startsWith("/lib/")) &&
                page.stats.rows.every((it) => it.dropped === 0),
        ],
        [
            // What a module shows is a function of its own text and of no body, so the edit
            // reads the surface of it again and keeps the types it had.
            "the signature surface of an edited module is read again and kept",
            counted("signatures", "stales", "/main.mlk") >= 1 &&
                counted("signatures", "kept", "/main.mlk") >= 1,
        ],
        [
            "the body that was edited is checked again",
            counted("check", "stales") >= 1,
        ],
        [
            // A check is a value of a body, and the counters say which body: the name of the
            // entity it belongs to and the file it is written in.
            "a check is counted for the body it is about",
            page.stats.rows.some(
                (it) => it.pass === "check" && it.unit === "/main.mlk: main",
            ),
        ],
        [
            // What a look cost is the time of the page around it, and every row carries what
            // its pass cost. The times of the rows are zero in the browser this check drives
            // --- its clock reads the same value twice within one task, and a pull is one task
            // --- which is why the time of a pass is what the tests of the driver measure,
            // with a clock they control.
            "a look is timed, and its rows carry the time of their passes",
            page.stats.took > 0 &&
                page.stats.rows.every(
                    (it) => Number.isFinite(it.took) && it.took >= 0,
                ),
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
        `it paints a truth value ${page.branchWords.truth || "(nothing)"}, against a number ${page.branchWords.number || "(nothing)"} and a keyword ${page.branchWords.keyword || "(nothing)"}`,
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
        `the mir reads a ${page.mir.form} form of ${page.mir.blocks.length} block(s), which ${page.mirHover.says.trim()} marks ${JSON.stringify(page.mirHover.marked)}, and the ssa ${page.ssa.blocks.length} block(s)`,
    );
    console.log(
        `the lir reads ${page.lir.blocks.length} block(s) of ${page.lir.kinds.length} instruction(s) of ${new Set(page.lir.kinds).size} kind(s), in ${page.lir.locals.length} local line(s), and the structure is ${page.lir.structure.length} line(s) folded to ${page.lirFolded.structure.length}`,
    );
    console.log(
        `a choice reads as ${page.branch.blocks.length} block(s) of a ${page.branch.form} form and ${page.branchSsa.blockParams.length} parameter(s) of the join of the ssa form`,
    );
    console.log(
        `a choice without an else reads ${page.unit.stmts.filter((it) => it.includes("const unit")).length} unit(s) in its ${page.unit.blocks.length} block(s)`,
    );
    console.log(
        `the module assembles to ${page.wat.head}, which ${page.watPaint.keyword === page.watPaint.accent ? "is" : "is NOT"} painted, and folds to ${JSON.stringify(page.watFolded.text.trim())}, and the program printed ${JSON.stringify(page.ran.printed)}`,
    );
    console.log(
        `the debug option gives the module ${page.dwarfWat.sections.length} custom section(s) of DWARF, ${page.bareWat.sections.length} with none, and ${page.mapWat.sections.length} with the source map`,
    );
    console.log(
        `of ${page.watMarks.rows} line(s) on the screen, ${page.watMarks.marks} carry a fold marker: the module ${page.watMarks.moduleMarked ? "folds" : "does not fold"}, a func head ${page.watMarks.headMarked ? "folds" : "does not fold"}, and a line of a body ${page.watMarks.bodyMarked ? "FOLDS" : "does not fold"}`,
    );
    console.log(`the markers stand at ${JSON.stringify(page.watMarks.list)}`);
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
    console.log(
        `a read of an edited body took ${page.stats.took} ms, of which ${counted("check", "stales")} check(s) read again and ${page.stats.rows.filter((it) => it.took > 0).length} pass(es) timed`,
    );
    console.log(
        `the rows of that read: ${page.stats.rows.map((it) => `${it.pass}(${it.unit}) ${it.hits}/${it.misses}/${it.stales}/${it.kept}/${it.dropped}`).join(" ")}`,
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
