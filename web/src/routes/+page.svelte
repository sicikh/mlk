<script lang="ts">
    import { onMount } from "svelte";

    import AstView from "$lib/components/AstView.svelte";
    import Console, { type Line } from "$lib/components/Console.svelte";
    import Diagnostics from "$lib/components/Diagnostics.svelte";
    import Editor from "$lib/components/Editor.svelte";
    import FileList from "$lib/components/FileList.svelte";
    import HirView from "$lib/components/HirView.svelte";
    import Splitter from "$lib/components/Splitter.svelte";
    import TreeView from "$lib/components/TreeView.svelte";
    import {
        loadDriver,
        type Analysis,
        type Driver,
        type StdFile,
    } from "$lib/driver";

    interface Buffer {
        path: string;
        text: string;
    }

    /**
     * The buffers the editor opens with: modules that parse cleanly,
     * so the trees have something to show before anybody types.
     */
    const STARTER: Buffer[] = [
        {
            path: "/main.mlk",
            // The names of the prelude are the module's without it writing them: `Int` and
            // `Unit` are imports the compiler makes (`#[no-prelude]` refuses them).
            text: `#[extern]
pub fun println-int(x: Int): Unit

fun main(): Unit =
    let x = 42 * 2 - 10 in
    println-int(x + 20)
`,
        },
        {
            path: "/lib/arith.mlk",
            text: `fun nested(): Int =
    let x = 1 in
    let y = 2 in
    x + y
`,
        },
    ];

    /** The name of the buffer the editor opens with. */
    const FIRST = STARTER[0].path;

    /** The views of what the compiler makes of the buffer on the right. */
    type Tab = "diagnostics" | "cst" | "ast" | "hir";

    /**
     * Which panel a narrow screen shows, where there is room for one at a time.
     *
     * A wide screen lays the panels out side by side and has no use for this: there,
     * whichever panel is picked is already on the screen, so picking one is only felt
     * where they are stacked.
     */
    type View = "files" | "code" | "inspector" | "console";

    let buffers = $state<Buffer[]>(STARTER);
    let active = $state(FIRST);

    /**
     * The files of the standard library of the language.
     *
     * The library is part of the compiler: the driver holds it and hands it over, and the
     * editor shows it as it shows a buffer a person wrote. What it is not is a place to write:
     * a file of it is read, and nothing is made beside it.
     */
    let library = $state<StdFile[]>([]);

    /** The buffers open in the editor, in the order of their tabs: opening is not reading. */
    let open = $state<string[]>(STARTER.map((it) => it.path));

    let analysis = $state<Analysis | null>(null);
    let tab = $state<Tab>("diagnostics");

    /**
     * The panel a narrow screen has in front, which is the editor until asked otherwise.
     *
     * Nothing is hidden where there is room for everything: the panels take their places
     * side by side whatever this says, and a person is left where they were.
     */
    let view = $state<View>("code");

    /**
     * What a pointer is on in a tree of the inspector, as the compiler counts it.
     *
     * A tree and the editor show the same buffer, so a range of one is a range of the other:
     * the editor marks the part of the source the pointer stands for, and — when what it is on
     * names something — the part of the source the name comes from.
     */
    let hovered = $state<[number, number] | null>(null);
    let resolved = $state<[number, number] | null>(null);

    /**
     * How many times a person has asked to be taken to a place in the code rather than
     * merely pointed at one: a row of a tree that holds nothing is picked, and the editor
     * answers by scrolling the mark the row makes into view.
     */
    let reveals = $state(0);
    let log = $state<Line[]>([
        { level: "note", text: "loading the wasm driver…" },
    ]);

    /**
     * What a view of the inspector tells the page about the pointer: the part of the source
     * a line stands for, and what that line names.
     */
    function pointed(
        at: [number, number] | null,
        names: [number, number] | null = null,
    ) {
        hovered = at;
        resolved = names;
    }

    /**
     * A row of a tree that holds nothing is a place in the source rather than a thing to fold,
     * and picking one is a request to be shown that place. A narrow screen has room for one
     * panel, so it answers by putting the editor in front, where the mark it makes is read.
     */
    function picked(
        at: [number, number] | null,
        names: [number, number] | null = null,
    ) {
        if (!at) return;

        pointed(at, names);
        view = "code";
        reveals += 1;
    }

    /**
     * How much room each panel takes, which a person drags the handles between them for.
     * The editor takes what is left, so only these three are kept.
     */
    let files = $state(220);
    let inspector = $state(380);
    let console = $state(160);

    let driver: Driver | null = null;

    const diagnostics = $derived(analysis?.diagnostics ?? []);
    const errors = $derived(
        diagnostics.filter((it) => it.level === "error").length,
    );

    onMount(async () => {
        try {
            driver = await loadDriver();
            say("info", "the driver is loaded");

            // A driver knows nothing until a host says what it holds: every buffer goes in first.
            for (const it of buffers) push(it.path);

            // The library of the language goes in beside them, and is shown beside them: it is
            // the compiler's, so the editor reads it where the compiler put it.
            library = driver.useStd();
            buffers = [
                ...buffers,
                ...library.map((it) => ({ path: it.path, text: it.text })),
            ];

            check();
        } catch (error) {
            say("error", `the driver did not load: ${String(error)}`);
        }
    });

    /** The directory a path sits in, which is everything up to its last separator. */
    function directoryOf(path: string): string {
        return path.slice(0, path.lastIndexOf("/") + 1);
    }

    /**
     * Whether a person may write at a path.
     *
     * The standard library is the compiler's, and so is the directory it sits in: a file of it
     * is read rather than written, and a buffer made in its directory would be a file of it.
     */
    function writable(path: string): boolean {
        return !library.some((it) => path.startsWith(directoryOf(it.path)));
    }

    /** Says one line to the console. */
    function say(level: Line["level"], text: string) {
        log = [...log, { level, text }];
    }

    /** The buffer a path names, if the editor holds it. */
    function find(path: string): Buffer | undefined {
        return buffers.find((it) => it.path === path);
    }

    /** Hands the text of a buffer to the driver; nothing is computed until it is asked for. */
    function push(path: string) {
        const it = find(path);

        if (!driver || !it) return;

        // A file of the library is the driver's own: what the editor shows of it came from
        // there, and pushing it back would only let a copy overwrite what the compiler holds.
        if (!writable(path)) return;

        try {
            driver.push(it.path, it.text);
        } catch (error) {
            say("error", `the driver refused ${name(path)}: ${String(error)}`);
        }
    }

    /** Reads back what the driver made of the active buffer. */
    function check() {
        if (!driver || active === "") return;

        // Whatever a pointer was on belongs to the parse this one replaces.
        hovered = null;
        resolved = null;

        try {
            analysis = driver.analyze(active);
        } catch (error) {
            analysis = null;
            say(
                "error",
                `the driver refused ${name(active)}: ${String(error)}`,
            );
        }
    }

    /** A buffer has a name, not a directory: there is no file system under the editor. */
    function name(path: string): string {
        return path.replace(/^\//, "");
    }

    /** A keystroke: the text is the buffer's, and the driver is told about it. */
    function onText(path: string, text: string) {
        const it = find(path);

        if (!it) return;

        // A file of the library is not a person's to change, and the editor takes no keystrokes
        // from one: a change that arrives by some other way is dropped rather than kept.
        if (!writable(path)) return;

        it.text = text;

        if (path !== active) return;

        push(path);
        check();
    }

    /** Another buffer in front, opened as a tab if it was not one. */
    function select(path: string) {
        if (!open.includes(path)) open = [...open, path];

        active = path;
        push(path);
        check();
    }

    /**
     * A buffer picked among the files, which is a request to write in it rather than
     * to read its name again: a narrow screen answers by putting the editor in front.
     */
    function edit(path: string) {
        view = "code";
        select(path);
    }

    /**
     * A tab goes away, and the buffer stays: closing what a person is reading
     * is not the same as dropping what it holds.
     */
    function closeTab(path: string) {
        const at = open.indexOf(path);

        open = open.filter((it) => it !== path);

        if (active !== path) return;

        const next = open[at] ?? open[at - 1] ?? "";

        active = next;
        analysis = null;

        if (next !== "") select(next);
    }

    /** A buffer at the path a person typed, in the directories that path names. */
    function create(path: string) {
        if (find(path)) return;

        if (!writable(path)) {
            say("note", `refused ${name(path)}: the library is the compiler's`);
            return;
        }

        buffers = [...buffers, { path, text: "" }];
        open = [...open, path];
        active = path;
        // A buffer that was just made is a buffer that was made to be written in.
        view = "code";
        say("note", `made ${name(path)}`);
        push(path);
        check();
    }

    /** Drops a buffer, and tells the driver the file is gone. */
    function remove(path: string) {
        // Nothing of the library is a person's to drop, and the files do not offer it.
        if (!writable(path)) return;

        const at = open.indexOf(path);

        buffers = buffers.filter((it) => it.path !== path);
        open = open.filter((it) => it !== path);
        driver?.push(path, null);
        say("note", `${name(path)} is gone`);

        if (active !== path) return;

        const next = open[at] ?? open[at - 1] ?? buffers[0]?.path ?? "";

        active = next;
        analysis = null;

        if (next !== "") select(next);
    }

    /** Compiles every buffer: the same work the driver does per keystroke, said out loud. */
    function compile() {
        if (!driver) return;

        for (const it of buffers) {
            const started = performance.now();

            push(it.path);

            try {
                const result = driver.analyze(it.path);
                const took = Math.round(performance.now() - started);
                const count = result.diagnostics.length;

                say(
                    "note",
                    `${name(it.path)}: ${count === 0 ? "no diagnostics" : `${count} diagnostic${count === 1 ? "" : "s"}`} in ${took} ms`,
                );
            } catch (error) {
                say(
                    "error",
                    `the driver refused ${name(it.path)}: ${String(error)}`,
                );
            }
        }

        check();

        if (errors > 0) {
            tab = "diagnostics";

            // What a person asked the compiler for is what it has to say about the buffer,
            // and a narrow screen can only put one panel in front: it is the one that says it.
            view = "inspector";
        }
    }

    /** Running needs a code generator, which the pipeline does not reach yet. */
    function run() {
        say("note", "nothing to run yet: the compiler stops at the typed tree");
    }
</script>

<svelte:head>
    <title>MLK editor</title>
</svelte:head>

<div
    class="ide"
    data-view={view}
    style="--files: {files}px; --inspector: {inspector}px; --console: {console}px"
>
    <header class="top">
        <span class="brand">MLK</span>
        <span class="grow"></span>
        <span class="tool-name">{name(active)}</span>
        <button class="tool" onclick={run}>Run</button>
        <button class="tool primary" onclick={compile}>Compile</button>
    </header>

    <!--
        A narrow screen has room for one panel, and this is how a person says which.
        Where there is room for all of them the switcher is not drawn at all (see the styles).
    -->
    <nav class="switch" aria-label="Which panel is shown">
        <button
            data-pane="files"
            class:active={view === "files"}
            aria-pressed={view === "files"}
            onclick={() => (view = "files")}>Files</button
        >
        <button
            data-pane="code"
            class:active={view === "code"}
            aria-pressed={view === "code"}
            onclick={() => (view = "code")}>Code</button
        >
        <button
            data-pane="inspector"
            class:active={view === "inspector"}
            aria-pressed={view === "inspector"}
            onclick={() => (view = "inspector")}
        >
            Inspect
            {#if diagnostics.length > 0}
                <span class="badge" class:error={errors > 0}
                    >{diagnostics.length}</span
                >
            {/if}
        </button>
        <button
            data-pane="console"
            class:active={view === "console"}
            aria-pressed={view === "console"}
            onclick={() => (view = "console")}>Console</button
        >
    </nav>

    <aside class="files">
        <FileList
            files={buffers.map((it) => it.path)}
            {active}
            {writable}
            onSelect={edit}
            onCreate={create}
            onRemove={remove}
        />
    </aside>

    <Splitter
        style="grid-column: 2; grid-row: 2"
        direction="x"
        size={files}
        min={150}
        max={480}
        label="Size of the files panel"
        onResize={(size) => (files = size)}
    />

    <main class="editor">
        {#if active !== ""}
            <Editor
                tabs={open}
                path={active}
                text={(it) => find(it)?.text ?? ""}
                {diagnostics}
                {hovered}
                {resolved}
                {reveals}
                {writable}
                onInput={onText}
                onSelect={select}
                onClose={closeTab}
            />
        {:else}
            <p class="empty">
                No buffer is open. Pick one in Files, or make one with <code
                    >+</code
                >.
            </p>
        {/if}
    </main>

    <Splitter
        style="grid-column: 4; grid-row: 2"
        direction="x"
        size={inspector}
        min={240}
        max={760}
        flip
        label="Size of the inspector panel"
        onResize={(size) => (inspector = size)}
    />

    <aside class="inspector" data-panel="inspector">
        <nav class="tabs">
            <button
                data-tab="diagnostics"
                class:active={tab === "diagnostics"}
                onclick={() => (tab = "diagnostics")}
            >
                Diagnostics
                {#if diagnostics.length > 0}
                    <span class="badge" class:error={errors > 0}
                        >{diagnostics.length}</span
                    >
                {/if}
            </button>
            <button
                data-tab="cst"
                class:active={tab === "cst"}
                onclick={() => (tab = "cst")}>CST</button
            >
            <button
                data-tab="ast"
                class:active={tab === "ast"}
                onclick={() => (tab = "ast")}>AST</button
            >
            <button
                data-tab="hir"
                class:active={tab === "hir"}
                onclick={() => (tab = "hir")}>HIR</button
            >
        </nav>

        <div class="view">
            {#if tab === "diagnostics"}
                <Diagnostics {diagnostics} text={find(active)?.text ?? ""} />
            {:else if !analysis}
                <p class="empty">Waiting for a parse.</p>
            {:else if tab === "cst"}
                <TreeView
                    node={analysis.cst}
                    onHover={pointed}
                    onPick={picked}
                />
            {:else if tab === "ast"}
                {#if analysis.ast === null}
                    <p class="empty">The root of the tree is not a module.</p>
                {:else}
                    <AstView
                        value={analysis.ast}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if analysis.hir === null}
                <p class="empty">There is nothing to lower.</p>
            {:else}
                {#each analysis.hir.nodes as node, index (index)}
                    <HirView {node} onHover={pointed} onPick={picked} />
                {/each}
            {/if}
        </div>
    </aside>

    <Splitter
        style="grid-column: 1 / -1; grid-row: 3"
        direction="y"
        size={console}
        min={80}
        max={480}
        flip
        label="Size of the console"
        onResize={(size) => (console = size)}
    />

    <section class="console">
        <Console lines={log} />
    </section>
</div>

<style>
    /*
     * The shell: a header, the three panels, and the console under them.
     *
     * The lanes the handles sit in are nothing wide: a handle lies on the seam itself,
     * so that the panels meet along the borders they already have
     * and no lane is drawn behind them.
     */
    .ide {
        display: grid;
        grid-template-columns: var(--files) 0 minmax(0, 1fr) 0 var(--inspector);
        grid-template-rows: auto minmax(0, 1fr) 0 var(--console);
        height: 100dvh;
    }

    .files {
        grid-column: 1;
        grid-row: 2;
    }

    .editor {
        grid-column: 3;
        grid-row: 2;
    }

    .inspector {
        display: flex;
        flex-direction: column;
        grid-column: 5;
        grid-row: 2;
        min-height: 0;
        min-width: 0;
        background: var(--surface);
    }

    .console {
        grid-column: 1 / -1;
        grid-row: 4;
    }

    /*
     * Each of these holds one panel, and a one-cell grid is the shortest way to say
     * that whatever is inside it fills it: a percentage height asks the same question
     * and does not always get an answer.
     */
    .files,
    .editor,
    .console {
        display: grid;
        min-height: 0;
        min-width: 0;
    }

    .top {
        display: flex;
        gap: 0.75rem;
        align-items: center;
        grid-column: 1 / -1;
        padding: 0.4rem 0.75rem;
        background: var(--surface);
        border-bottom: 1px solid var(--border);
    }

    .brand {
        font-weight: 700;
        letter-spacing: 0.12em;
    }

    .grow {
        flex: 1;
    }

    .tool-name {
        color: var(--muted);
        font-family: var(--mono);
        font-size: 12px;
    }

    .empty {
        margin: 0;
        padding: 1rem;
        color: var(--muted);
    }

    .empty code {
        font-family: var(--mono);
    }

    .tool {
        padding: 0.2rem 0.7rem;
        background: var(--raised);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        font-size: 12px;
    }

    .tool:hover {
        border-color: #38414f;
    }

    .tool.primary {
        background: #2a3f63;
        border-color: #35507d;
        color: #d8e6ff;
    }

    .tool.primary:hover {
        background: #33507f;
    }

    /*
     * The switcher of a narrow screen: a panel is picked here rather than sized by hand.
     * It is not drawn where the panels fit together, so nothing is said about it until then.
     */
    .switch {
        display: none;
    }

    .switch button {
        display: flex;
        flex: 1;
        gap: 0.35rem;
        align-items: center;
        justify-content: center;
        /* A finger is not a pointer: a control it aims at is roomy enough to hit. */
        min-height: 44px;
        padding: 0 0.35rem;
        border-bottom: 2px solid transparent;
        color: var(--muted);
        font-size: 11px;
        letter-spacing: 0.03em;
        text-transform: uppercase;
    }

    .switch button:hover {
        color: var(--text);
    }

    .switch button.active {
        border-bottom-color: var(--accent);
        color: var(--text);
    }

    .tabs {
        display: flex;
        padding: 0 0.25rem;
        border-bottom: 1px solid var(--border);
    }

    .tabs button {
        display: flex;
        gap: 0.35rem;
        align-items: center;
        padding: 0.4rem 0.55rem;
        border-bottom: 2px solid transparent;
        color: var(--muted);
        font-size: 11px;
        letter-spacing: 0.03em;
        text-transform: uppercase;
    }

    .tabs button:hover {
        color: var(--text);
    }

    .tabs button.active {
        border-bottom-color: var(--accent);
        color: var(--text);
    }

    .badge {
        padding: 0 0.3rem;
        background: var(--raised);
        border-radius: 999px;
        color: var(--muted);
        font-size: 10px;
    }

    .badge.error {
        color: var(--error);
    }

    .view {
        flex: 1;
        min-height: 0;
        padding: 0.4rem 0.5rem;
        overflow: auto;
    }

    .empty {
        margin: 0;
        padding: 0.35rem 0.25rem;
        color: var(--muted);
    }

    /*
     * A phone: one panel at a time, the whole way across, under the header and the switcher.
     *
     * The panels are the same elements in the same grid; what changes is that a narrow screen
     * draws only the one a person picked, in the row under the switcher rather than in a column
     * of its own. The handles between them are gone with the room between them (see `Splitter`),
     * so there is nothing left for them to size.
     */
    @media (max-width: 860px), (max-height: 520px) {
        .ide {
            grid-template-columns: minmax(0, 1fr);
            grid-template-rows: auto auto minmax(0, 1fr);
        }

        .switch {
            display: flex;
            grid-column: 1 / -1;
            grid-row: 2;
            background: var(--surface);
            border-bottom: 1px solid var(--border);
        }

        /* Nothing is drawn until it is asked for: the panels share the one cell there is. */
        .files,
        .editor,
        .inspector,
        .console {
            display: none;
            grid-column: 1;
            grid-row: 3;
        }

        .ide[data-view="files"] .files,
        .ide[data-view="code"] .editor,
        .ide[data-view="console"] .console {
            display: grid;
        }

        .ide[data-view="inspector"] .inspector {
            display: flex;
        }

        .top {
            gap: 0.5rem;
            padding: 0.35rem 0.5rem;
        }

        /* A finger picks a tab the way a pointer does, but it needs more room to land in. */
        .tabs button {
            padding: 0.55rem 0.7rem;
        }

        /* A header is one line here, and a name too long for it says so rather than pushing
           the tools off the screen. */
        .tool-name {
            min-width: 0;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .tool {
            min-height: 36px;
            padding: 0.35rem 0.8rem;
        }
    }
</style>
