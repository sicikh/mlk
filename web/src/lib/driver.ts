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
import init, { WasmDriver } from "@mlk/wasm";

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

/**
 * A node or a token of a syntax tree, as the compiler hands it over ([ADR-0002]).
 *
 * The shape is the one the syntax tree serializes itself into ([ADR-0002]):
 * a node is its kind, the range of text it covers and its children,
 * a token is its kind, its range and its text, with no children of its own.
 * A token's text carries the trivia around it — the spaces and newlines before and after it —
 * because the tree is lossless and keeps them where they were.
 *
 * Nothing here parses the tree: the editor folds and prints the same values
 * the compiler walks, and there is no second representation of them anywhere.
 *
 * [ADR-0002]: https://github.com/sicikh/mlk/blob/main/docs/adr/0002-lossless-syntax-tree.md
 */
export interface SyntaxNode {
    /** The kind of the node: `MODULE_ROOT`, `FUN_DECL`, `IDENT`, `EOF`. */
    kind: string;

    /** The range of the buffer it covers, in bytes: `[start, end]`. */
    text_range: [number, number];

    /** What the node holds, in the order it holds it. Only a node has children. */
    children?: SyntaxNode[];

    /** The text of a token, trivia included. Only a token has text. */
    text?: string;
}

/** Whether a node is a token rather than a node of its own. */
export function isToken(node: SyntaxNode): boolean {
    return node.children === undefined;
}

/** Everything the editor shows about one buffer. */
export interface Analysis {
    /** The concrete syntax tree: lossless, tokens and trivia included. */
    cst: SyntaxNode;

    /**
     * The typed view over the same tree, as the compiler serializes it,
     * or `null` when the parse did not find a module root.
     *
     * Its shape is the one the typed tree gives itself: a field of the compiler's AST is a key here,
     * a list is an array, and a token is a node of the concrete tree.
     */
    ast: unknown;

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
        analyze: (path) => wasm.analyze(path) as Analysis,
    };
}
