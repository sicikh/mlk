<script lang="ts">
    import { onMount } from "svelte";

    import AstView from "$lib/components/AstView.svelte";
    import Console, { type Line } from "$lib/components/Console.svelte";
    import Diagnostics from "$lib/components/Diagnostics.svelte";
    import Editor from "$lib/components/Editor.svelte";
    import FileList from "$lib/components/FileList.svelte";
    import Splitter from "$lib/components/Splitter.svelte";
    import TreeView from "$lib/components/TreeView.svelte";
    import { loadDriver, type Analysis, type Driver } from "$lib/driver";

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
            text: `fun main(): Unit =
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
    type Tab = "diagnostics" | "cst" | "ast";

    let buffers = $state<Buffer[]>(STARTER);
    let active = $state(FIRST);
    let analysis = $state<Analysis | null>(null);
    let tab = $state<Tab>("diagnostics");
    let log = $state<Line[]>([
        { level: "note", text: "loading the wasm driver…" },
    ]);

    /**
     * How much room each panel takes, which a person drags the handles between them for.
     * The editor takes what is left, so only these three are kept.
     */
    let files = $state(220);
    let inspector = $state(380);
    let console = $state(160);

    let driver: Driver | null = null;

    const buffer = $derived(buffers.find((it) => it.path === active));
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

            check();
        } catch (error) {
            say("error", `the driver did not load: ${String(error)}`);
        }
    });

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

        try {
            driver.push(it.path, it.text);
        } catch (error) {
            say("error", `the driver refused ${name(path)}: ${String(error)}`);
        }
    }

    /** Reads back what the driver made of the active buffer. */
    function check() {
        if (!driver || active === "") return;

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
    function onInput(text: string) {
        const it = find(active);

        if (!it) return;

        it.text = text;
        push(active);
        check();
    }

    /** Another buffer, and what the compiler already made of it. */
    function select(path: string) {
        active = path;
        push(path);
        check();
    }

    /** A buffer at the path a person typed, in the directories that path names ([ADR-0007]). */
    function create(path: string) {
        if (find(path)) return;

        buffers = [...buffers, { path, text: "" }];
        active = path;
        say("note", `made ${name(path)}`);
        push(path);
        check();

        // [ADR-0007]: https://github.com/sicikh/mlk/blob/main/docs/adr/0007-vfs-file-state.md
    }

    /** Drops a buffer, and tells the driver the file is gone ([ADR-0007]). */
    function remove(path: string) {
        buffers = buffers.filter((it) => it.path !== path);
        driver?.push(path, null);
        say("note", `${name(path)} is gone`);

        if (active !== path) return;

        const next = buffers[0];

        active = next?.path ?? "";
        analysis = null;

        if (next) select(next.path);
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

        if (errors > 0) tab = "diagnostics";
    }

    /** Running needs a code generator, which the pipeline does not reach yet ([ADR-0005]). */
    function run() {
        say("note", "nothing to run yet: the compiler stops at the typed tree");

        // [ADR-0005]: https://github.com/sicikh/mlk/blob/main/docs/adr/0005-compiler-pipeline.md
    }
</script>

<svelte:head>
    <title>MLK editor</title>
</svelte:head>

<div
    class="ide"
    style="--files: {files}px; --inspector: {inspector}px; --console: {console}px"
>
    <header class="top">
        <span class="brand">MLK</span>
        <span class="grow"></span>
        <span class="tool-name">{name(active)}</span>
        <button class="tool" onclick={run}>Run</button>
        <button class="tool primary" onclick={compile}>Compile</button>
    </header>

    <aside class="files">
        <FileList
            files={buffers.map((it) => it.path)}
            {active}
            onSelect={select}
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
        {#if buffer}
            <Editor path={buffer.path} text={buffer.text} {onInput} />
        {:else}
            <p class="empty">No buffer. Make one with <code>+</code>.</p>
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
        </nav>

        <div class="view">
            {#if tab === "diagnostics"}
                <Diagnostics {diagnostics} />
            {:else if !analysis}
                <p class="empty">Waiting for a parse.</p>
            {:else if tab === "cst"}
                <TreeView node={analysis.cst} />
            {:else if analysis.ast === null}
                <p class="empty">The root of the tree is not a module.</p>
            {:else}
                <AstView value={analysis.ast} />
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
</style>
