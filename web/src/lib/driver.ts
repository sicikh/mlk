/**
 * The driver, as the editor sees it.
 *
 * The wasm module hands out one class, `WasmDriver`, whose methods cross the boundary
 * as untyped values; the shapes below are the mirror of the Rust types in
 * `crates/mlkc-wasm/src/lib.rs`, and this file is the one place they have to be kept in step.
 *
 * Nothing here decides anything about the compiler: the editor pushes the text of a buffer
 * and reads back what the pipeline made of it ([ADR-0008]). The same driver answers the CLI,
 * which is the point of keeping the boundary this thin.
 *
 * [ADR-0008]: https://github.com/sicikh/mlk/blob/main/docs/adr/0008-compiler-driver.md
 */
import init, { WasmDriver } from '@mlk/wasm';

/** One place a diagnostic points at. */
export interface Label {
	/** The start of the range, in bytes into the buffer. */
	start: number;

	/** The end of the range, in bytes into the buffer. */
	end: number;

	/** The line the range starts on, counted from zero. */
	line: number;

	/** The column of that line, counted in bytes from zero. */
	column: number;

	/** Whether it is the place the diagnostic is about, rather than one it mentions. */
	primary: boolean;

	/** What the label says: often empty for the primary one. */
	message: string;
}

/** What the parser reported about a buffer. */
export interface Diagnostic {
	/** `error`, `warning`, `note`, or `help`. */
	level: string;

	/** `parser` for everything the parser reports. */
	category: string;

	/** The code of the kind of mistake within its category. */
	code: string;

	/** The message, without the location: the location is in the labels. */
	message: string;

	/** Where the diagnostic points, the primary one first. */
	labels: Label[];

	/** What the diagnostic has to add. */
	notes: string[];
}

/** Everything the editor shows about one buffer. */
export interface Analysis {
	/** The dump of the concrete syntax tree: lossless, tokens and trivia included. */
	cst: string;

	/** The dump of the typed view over it, or `null` when the root is not a module. */
	ast: string | null;

	/** What the parser reported. */
	diagnostics: Diagnostic[];
}

/** The driver, typed for the editor. */
export interface Driver {
	/**
	 * Feeds the text of a buffer into the driver; `null` means the buffer is gone.
	 *
	 * Returns whether the contents changed: pushing the same text again changes nothing,
	 * which is what makes analyzing after every keystroke affordable ([ADR-0007]).
	 *
	 * [ADR-0007]: https://github.com/sicikh/mlk/blob/main/docs/adr/0007-vfs-file-state.md
	 */
	push(path: string, text: string | null): boolean;

	/** The trees and the diagnostics of a buffer, computed only for what changed. */
	analyze(path: string): Analysis;
}

/**
 * Loads the wasm module and hands out a driver over it.
 *
 * The module is fetched and instantiated on the first call; a page that loads once
 * loads it once.
 */
export async function loadDriver(): Promise<Driver> {
	await init();

	const wasm = new WasmDriver();

	return {
		push: (path, text) => wasm.setText(path, text),
		analyze: (path) => wasm.analyze(path) as Analysis
	};
}
