/**
 * The driver, as the editor sees it.
 *
 * The compiler runs in a worker of its own (see `driver.worker.ts`): the page never blocks on it,
 * and a pull that takes a while takes it where nothing a person does has to wait. What crosses the
 * boundary is the driver's own set of pulls --- one method per value, each computing only what
 * it answers --- so a view asks for the tree it shows and not for every tree there is. The
 * protocol is the shape of an editor's server as well: a notification for what a host tells the
 * driver, a request and an answer for what it asks.
 *
 * The shapes below are the mirror of the Rust types in
 * `crates/mlkc-wasm/src/lib.rs`, and this file is the one place they have to be kept in step.
 *
 * Nothing here decides anything about the compiler: the editor pushes the text of a buffer
 * and reads back what the pipeline made of it. The same driver answers the CLI,
 * which is the point of keeping this boundary this thin.
 */

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

    /**
     * The two digits of the category, in the order the pipeline runs its stages: `02` for the
     * parser. A code a person reads is these digits and then the kind's.
     */
    categoryCode: string;

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
 * A node or a token of a syntax tree, as the compiler hands it over.
 *
 * The shape is the one the syntax tree serializes itself into:
 * a node is its kind, the range of text it covers and its children,
 * a token is its kind, its range and its text, with no children of its own.
 * A token's text carries the trivia around it — the spaces and newlines before and after it —
 * because the tree is lossless and keeps them where they were.
 *
 * Nothing here parses the tree: the editor folds and prints the same values
 * the compiler walks, and there is no second representation of them anywhere.
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

/**
 * One line of the HIR of a module, and the lines under it.
 *
 * The compiler reads the HIR the way a fixture shows it --- the module and its items, and
 * then a body per entity that owns one --- and hands the reading over as a tree: a line is
 * what a person folds, and a range is what the editor marks in the buffer while a pointer is
 * on it. A line that is about nothing a module wrote --- a section header, a field of
 * a declaration --- carries no range.
 */
export interface HirNode {
    /** What the line says: `fun main  @2.0`, `param: Int -> use Int`. */
    text: string;

    /**
     * The same words, as the parts a host paints.
     *
     * A line is one part unless a part of it says something that reads differently from the
     * rest: the type a signature writes is a path, and a path is painted the way a path is
     * painted wherever it is written.
     */
    parts: HirPart[];

    /**
     * What the line is: `module`, `body`, `section`, `item`, `field`, `expr`, `pat`, or
     * `path`, which is what the view paints it by.
     */
    kind: string;

    /** The part of the buffer the line is about, in bytes, or nothing. */
    range: [number, number] | null;

    /**
     * The part of the buffer what the line resolves to is written at, or nothing.
     *
     * A path of the HIR names something — an entity of the module, an entry of its import
     * table, a binding of the body — and where that name comes from is another place in the
     * same buffer: the view marks both while a pointer is on the line.
     */
    resolves: [number, number] | null;

    /** The lines under it, which the view folds. */
    children: HirNode[];
}

/** One part of the text of a line of the HIR. */
export interface HirPart {
    /** What this part of the line says. */
    text: string;

    /**
     * What it is, when it is not what the line is — a type written as a path is a `path` —
     * and `null` for the part of a line that says what the line says.
     */
    kind: string | null;
}

/** The HIR of the module in the buffer, as the compiler reads it. */
export interface Hir {
    /** The lines of the reading: the module and its items first, the bodies after them. */
    nodes: HirNode[];
}

/** One entity of the surface of a module and the type it was resolved to. */
export interface SurfaceType {
    /** What the entity is: `fun main`, `type Point`. */
    name: string;

    /** The type it was resolved to, as it reads: `() -> Int`. */
    ty: string;

    /** Where the declaration is written, in bytes, or nothing where it is written nowhere. */
    range: [number, number] | null;
}

/** One node of a body and the type it was checked to. */
export interface TypedNode {
    /** What the node is: `expr` or `pat`. */
    kind: string;

    /** What a node that is written nowhere is called by: `expr #3`. */
    label: string;

    /** The part of the buffer the node covers, in bytes, or nothing when nothing was written. */
    range: [number, number] | null;

    /** The type, as it reads: `Int`, `(Int) -> Bool`, `{error}`. */
    ty: string;

    /** Whether the type is the type of a mistake, which the view paints as one. */
    error: boolean;
}

/** The types of the nodes of one checked body. */
export interface BodyTypes {
    /** The entity the body belongs to: `fun main`. */
    owner: string;

    /** Where the declaration that owns the body is written, in bytes. */
    range: [number, number] | null;

    /** The types of the nodes of the body, in the order they are written in. */
    nodes: TypedNode[];
}

/**
 * What checking the types of a module left, as a host reads it.
 *
 * The types are values of the compiler, and what a host shows of them is what a person reads
 * at a place: the type an entity was resolved to, and the type every node of every body was
 * checked to.
 */
export interface Types {
    /** The types of the entities of the module, in the order it declares them. */
    surface: SurfaceType[];

    /** The checked bodies, in the order the module declares them. */
    bodies: BodyTypes[];
}

/** The driver, typed for the editor. */
export interface Driver {
    /**
     * Feeds the text of a buffer into the driver; `null` means the buffer is gone.
     *
     * A notification rather than a question: the driver is told, and the pulls that follow it
     * see the text, because the worker answers in the order it is told.
     */
    push(path: string, text: string | null): void;

    /**
     * Records the standard library of the language in the driver, and hands over the files it
     * is made of.
     *
     * The library is part of the compiler rather than of the editor: a host asks for it
     * instead of pushing it, and what comes back is what a host shows --- files of the
     * compiler's, with nothing in them for a person to write.
     */
    useStd(): Promise<StdFile[]>;

    /** The concrete syntax tree of the buffer: lossless, tokens and trivia included. */
    cst(path: string): Promise<SyntaxNode>;

    /**
     * The typed view over the same tree, or `null` when the parse found no module root.
     *
     * Its shape is the one the typed tree gives itself: a field of the compiler's AST is a key here,
     * a list is an array, and a token is a node of the concrete tree.
     */
    ast(path: string): Promise<unknown | null>;

    /** The HIR of the module, or `null` when there is nothing to lower. */
    hir(path: string): Promise<Hir | null>;

    /** What checking the types of the module left, or `null` when there is nothing to check. */
    types(path: string): Promise<Types | null>;

    /** What the stages of the pipeline reported, in the shape an editor marks the buffer with. */
    diagnostics(path: string): Promise<Diagnostic[]>;
}

/** One file of the standard library of the language, as the driver hands it over. */
export interface StdFile {
    /** The path the driver knows the file by, which is the one the editor shows. */
    path: string;

    /** The source of the file. */
    text: string;
}

/**
 * A question the editor asks the driver.
 *
 * Passing the text of a buffer is a notification: there is nothing to answer, and nothing to
 * wait for. Everything else is a question, and carries the id the answer comes back under.
 */
export type DriverRequest =
    | { kind: "setText"; path: string; text: string | null }
    | { kind: "useStd"; id: number }
    | { kind: "cst"; id: number; path: string }
    | { kind: "ast"; id: number; path: string }
    | { kind: "hir"; id: number; path: string }
    | { kind: "types"; id: number; path: string }
    | { kind: "diagnostics"; id: number; path: string };

/**
 * What the worker says back.
 *
 * `ready` is said once, when the wasm module is instantiated and the driver can be asked;
 * `fatal` replaces it when the module did not load, and the worker is not going to answer
 * anything. An answer or an error closes one question: a question is asked once and answered
 * once, in the order it was asked.
 */
export type DriverResponse =
    | { kind: "ready" }
    | { kind: "fatal"; message: string }
    | { kind: "answer"; id: number; value: unknown }
    | { kind: "error"; id: number; message: string };

/**
 * Starts the driver: a worker of its own, and the door to it.
 *
 * The worker fetches and instantiates the wasm module, and everything the editor asks of the
 * compiler is a message to it: the page draws while the compiler works, and a pull that takes
 * a while takes it where nothing a person does has to wait.
 *
 * What comes back is the driver itself --- one method per value, each a question --- and its
 * questions and the text pushed into it keep their order, so a pull sees every push before it.
 */
export async function loadDriver(): Promise<Driver> {
    // The worker is asked for by its URL, as a string: the hosts this runs on are browsers, and
    // not every one of them reads a `URL` where a worker script is named.
    const worker = new Worker(
        new URL("./driver.worker.ts", import.meta.url).href,
        {
            type: "module",
        },
    );

    const ready = Promise.withResolvers<void>();

    /** What each question is waiting for, by the id it was asked under. */
    const waiting = new Map<
        number,
        { resolve: (value: unknown) => void; reject: (error: Error) => void }
    >();

    /**
     * The questions asked before the worker was up.
     *
     * A worker starts when it starts: the wasm module takes a moment to arrive, and a message
     * sent meanwhile would be a question to nobody. They wait here, in the order they were
     * asked, and go out together when the worker says it is up --- which is also the order the
     * worker answers in, so a pull still sees every push before it.
     */
    const early: DriverRequest[] = [];

    let serial = 0;
    let up = false;

    /** Why the worker stopped answering, if it did. */
    let broken: Error | null = null;

    /** Says the driver is beyond asking, and lets go of everything that was waiting for it. */
    const fail = (error: Error) => {
        if (broken) return;

        broken = error;

        ready.reject(error);

        for (const it of waiting.values()) it.reject(error);

        waiting.clear();
        worker.terminate();
    };

    const post = (request: DriverRequest) => {
        if (broken) return;

        if (up) worker.postMessage(request);
        else early.push(request);
    };

    /** Asks a question and hands back the promise the answer keeps. */
    const ask = <T>(question: (id: number) => DriverRequest): Promise<T> => {
        return new Promise<T>((resolve, reject) => {
            if (broken) {
                reject(broken);
                return;
            }

            const id = ++serial;

            waiting.set(id, {
                resolve: (value) => resolve(value as T),
                reject,
            });
            post(question(id));
        });
    };

    worker.onmessage = (event: MessageEvent<DriverResponse>) => {
        const message = event.data;

        switch (message.kind) {
            case "ready":
                up = true;
                ready.resolve();

                for (const request of early) worker.postMessage(request);

                early.length = 0;
                break;

            case "fatal":
                fail(new Error(message.message));
                break;

            case "answer": {
                const it = waiting.get(message.id);

                waiting.delete(message.id);
                it?.resolve(message.value);
                break;
            }

            case "error": {
                const it = waiting.get(message.id);

                waiting.delete(message.id);
                it?.reject(new Error(message.message));
                break;
            }
        }
    };

    // A worker whose script never ran, or whose module did not load, has nobody to answer with:
    // the browser says so here rather than through the protocol.
    worker.onerror = (event: ErrorEvent) => {
        event.preventDefault();
        fail(new Error(event.message || "the driver worker did not start"));
    };

    await ready.promise;

    return {
        push: (path, text) => post({ kind: "setText", path, text }),
        useStd: () => ask<StdFile[]>((id) => ({ kind: "useStd", id })),
        cst: (path) => ask<SyntaxNode>((id) => ({ kind: "cst", id, path })),
        ast: (path) => ask<unknown | null>((id) => ({ kind: "ast", id, path })),
        hir: (path) => ask<Hir | null>((id) => ({ kind: "hir", id, path })),
        types: (path) =>
            ask<Types | null>((id) => ({ kind: "types", id, path })),
        diagnostics: (path) =>
            ask<Diagnostic[]>((id) => ({ kind: "diagnostics", id, path })),
    };
}
