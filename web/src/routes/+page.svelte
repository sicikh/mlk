<script lang="ts">
    import { onMount } from "svelte";

    import AstView from "$lib/components/AstView.svelte";
    import ConfigView from "$lib/components/ConfigView.svelte";
    import Console, { type Line } from "$lib/components/Console.svelte";
    import Diagnostics from "$lib/components/Diagnostics.svelte";
    import Editor from "$lib/components/Editor.svelte";
    import FileList from "$lib/components/FileList.svelte";
    import HirView from "$lib/components/HirView.svelte";
    import LirView from "$lib/components/LirView.svelte";
    import MirView from "$lib/components/MirView.svelte";
    import Splitter from "$lib/components/Splitter.svelte";
    import StatsView from "$lib/components/StatsView.svelte";
    import TreeView from "$lib/components/TreeView.svelte";
    import TypeView from "$lib/components/TypeView.svelte";
    import WatView from "$lib/components/WatView.svelte";
    import {
        loadDriver,
        type Cost,
        type DebugInfo,
        type Diagnostic,
        type Driver,
        type Hir,
        type Lir,
        type Mir,
        type OptLevel,
        type StatsRow,
        type StdFile,
        type SyntaxNode,
        type Types,
        type Wat,
    } from "$lib/driver";

    interface Buffer {
        path: string;
        text: string;
    }

    /**
     * What the inspector read of the active buffer, one value at a time.
     *
     * A value is read when something shows it, so a field is `undefined` until the tab it
     * belongs to has been in front, `null` when the driver was asked and had nothing to show,
     * and what the driver handed over once it did. What every tab counts on --- the diagnostics
     * and the types --- is read with every look, because more than one tab reads them.
     */
    interface Reading {
        diagnostics: Diagnostic[];
        types: Types | null;

        /** What the look cost: the work the driver did, and what a person waited for it. */
        cost: Cost;

        /** The parse, read when the tab of the concrete tree is in front. */
        cst?: SyntaxNode;

        /** The typed view over the parse, read when the tab of the AST is in front. */
        ast?: unknown | null;

        /** The HIR, read when the tab of the HIR is in front. */
        hir?: Hir | null;

        /** The MIR in the CFG form, read when its tab is in front. */
        mir?: Mir | null;

        /** The MIR in the SSA form, read when its tab is in front. */
        mirSsa?: Mir | null;

        /** The LIR, read when its tab is in front. */
        lir?: Lir | null;

        /** The WASM of the module, read when the tab of the WAT is in front. */
        wat?: Wat | null;
    }

    /**
     * The buffers the editor opens with: modules that parse cleanly,
     * so the trees have something to show before anybody types.
     */
    const STARTER: Buffer[] = [
        {
            path: "/main.mlk",
            // The names of the prelude are the module's without it writing them: `Int`, `Unit`
            // and `print-int` are imports the compiler makes (`#[no-prelude]` refuses them).
            text: `fun fib(n : Int) : Int =
    fib-aux(n, 0, 1)

fun fib-aux(n : Int, a : Int, b : Int) : Int =
    if n == 0 then
        a
    else
        fib-aux(n - 1, b, a + b)

#[entry]
pub fun main() : Unit =
    print-int(fib(5))
`,
        },
    ];

    /** The name of the buffer the editor opens with. */
    const FIRST = STARTER[0].path;

    /** The views of what the compiler makes of the buffer on the right. */
    type Tab =
        | "diagnostics"
        | "cst"
        | "ast"
        | "hir"
        | "mir"
        | "mir-ssa"
        | "lir"
        | "wat"
        | "tc"
        | "stats"
        | "config";

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

    /**
     * What has been read of the active buffer.
     *
     * A reading is replaced whole rather than changed in place, and the trees inside it are
     * the compiler's own values: nothing here makes them reactive, which is what `raw` says.
     */
    let reading = $state.raw<Reading | null>(null);
    let tab = $state<Tab>("diagnostics");

    /**
     * How the pipeline is configured: what debug information the modules carry, and how hard
     * the passes optimize ([ADR-0025](../../../docs/adr/0025-debug-information-formats.md)).
     *
     * The options belong to the driver rather than to a buffer, and every value read after
     * they are pushed is built under them: the WAT shows the custom sections they add, and a
     * program run under them is the program they describe.
     */
    let debug = $state<DebugInfo>("source-map");
    let opt = $state<OptLevel>("none");

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
     * What the program printed, and what happened to it, as the console shows it.
     *
     * The program console is the second console of the page: the compiler says what it thinks of
     * the buffers, and running the program says what the program did. A run is of every buffer
     * rather than of the one in front, so this belongs to the page and not to a buffer.
     */
    let program = $state<Line[]>([]);

    /** Which console a run puts in front: a person pressed Run to read what the program says. */
    let consoleTab = $state<"compiler" | "program">("compiler");

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

    const diagnostics = $derived(reading?.diagnostics ?? []);
    const errors = $derived(
        diagnostics.filter((it) => it.level === "error").length,
    );

    /**
     * How many nodes of the buffer were checked to the type of a mistake.
     *
     * A type of a mistake is a place the checker could not type, which is one thing to look at
     * in the tab of the types and another to read in the diagnostics; this is the one that says
     * how much of the tab is that.
     */
    const untyped = $derived(
        (reading?.types?.bodies ?? [])
            .flatMap((it) => it.nodes)
            .filter((it) => it.error).length,
    );

    onMount(async () => {
        try {
            driver = await loadDriver();
            say("info", "the driver is loaded");

            // A driver knows nothing until a host says what it holds: every buffer goes in first.
            for (const it of buffers) push(it.path);

            // The library of the language goes in beside them, and is shown beside them: it is
            // the compiler's, so the editor reads it where the compiler put it.
            library = await driver.useStd();
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

        driver.push(it.path, it.text);
    }

    /**
     * The look that is on its way to the driver, if one is, and whether something asked for
     * another one while it runs.
     *
     * A look reads what the inspector shows, and a keystroke does not wait for it: what is
     * asked while one runs is remembered, and the look after it reads the buffer as it is by
     * then. The driver is told each keystroke either way, so what a look reads is the text of
     * the moment it runs rather than the text the one before it started with.
     */
    let running: Promise<void> | null = null;
    let queued = false;

    /**
     * Reads back what the driver made of the active buffer.
     *
     * A pull per value, and only for the values something shows: the diagnostics and the types
     * every tab counts on, and the tree of the tab in front. Reading a tree that no tab shows
     * would be work with nowhere to go, which is the point of a driver that answers one
     * question at a time.
     */
    function check(): Promise<void> {
        if (!driver || active === "") return Promise.resolve();

        if (running) {
            queued = true;

            return running;
        }

        running = (async () => {
            try {
                do {
                    queued = false;

                    await look();
                } while (queued);
            } finally {
                running = null;
            }
        })();

        return running;
    }

    /** One look at the active buffer: everything the editor shows of it, read at once. */
    async function look() {
        if (!driver || active === "") return;

        const path = active;

        // Whatever a pointer was on belongs to the parse this one replaces.
        hovered = null;
        resolved = null;

        try {
            const started = performance.now();
            const [diagnostics, types, cst, ast, hir, mir, mirSsa, lir, wat] =
                await Promise.all([
                    driver.diagnostics(path),
                    driver.types(path),
                    tab === "cst" ? driver.cst(path) : undefined,
                    tab === "ast" ? driver.ast(path) : undefined,
                    tab === "hir" ? driver.hir(path) : undefined,
                    tab === "mir" ? driver.mir(path) : undefined,
                    tab === "mir-ssa" ? driver.mirSsa(path) : undefined,
                    tab === "lir" ? driver.lir(path) : undefined,
                    tab === "wat" ? driver.wat(path) : undefined,
                ]);

            // The time of a look is the time of its values: the counters are a read of their
            // own, and the clock is stopped before it. Reading them is what clears them, so
            // what comes back is what this look did and nothing of the one before it.
            const took = performance.now() - started;
            const stats = await driver.stats();

            // The buffer under the look is not always the buffer in front of a person: what
            // came back for one that is gone is dropped, and the look that follows it reads
            // the one that took its place.
            if (path !== active) return;

            reading = {
                diagnostics,
                types,
                cst,
                ast,
                hir,
                mir,
                mirSsa,
                lir,
                wat,
                cost: { rows: stats.rows, took },
            };
        } catch (error) {
            if (path !== active) return;

            reading = null;
            say("error", `the driver refused ${name(path)}: ${String(error)}`);
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
        reading = null;

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
        reading = null;

        if (next !== "") select(next);
    }

    /** Compiles every buffer: the same work the driver does per keystroke, said out loud. */
    async function compile() {
        if (!driver) return;

        for (const it of buffers) {
            const started = performance.now();

            push(it.path);

            try {
                const result = await driver.diagnostics(it.path);
                const took = Math.round(performance.now() - started);
                const counters = await driver.stats();
                const count = result.length;

                say(
                    "note",
                    `${name(it.path)}: ${count === 0 ? "no diagnostics" : `${count} diagnostic${count === 1 ? "" : "s"}`} in ${took} ms — ${counted(counters.rows)}`,
                );
            } catch (error) {
                say(
                    "error",
                    `the driver refused ${name(it.path)}: ${String(error)}`,
                );
            }
        }

        await check();

        if (errors > 0) {
            tab = "diagnostics";

            // What a person asked the compiler for is what it has to say about the buffer,
            // and a narrow screen can only put one panel in front: it is the one that says it.
            view = "inspector";
        }
    }

    /**
     * What the counters of one read come to, as a line of the console reads them.
     *
     * A person reads a summary, not a table: what a read reused, what it had to read again,
     * how much of that came out the same, what went, and how long the read spent on it.
     */
    function counted(rows: StatsRow[]): string {
        const total = (of: (it: StatsRow) => number) =>
            rows.reduce((all, it) => all + of(it), 0);

        const hits = total((it) => it.hits);
        const read = total((it) => it.misses + it.stales);
        const kept = total((it) => it.kept);
        const dropped = total((it) => it.dropped);
        const took = total((it) => it.took);

        return `${hits} hit${hits === 1 ? "" : "s"}, ${read} read again (${kept} kept), ${dropped} dropped, ${took.toFixed(1)} ms in the passes`;
    }

    /**
     * A tab of the inspector in front.
     *
     * A tab shows one value, and a value is read when it is shown: picking a tab is what asks
     * the driver for the tree, and the tabs picked before keep what they read. The tab of the
     * counters is the one that is not read: what it shows is what the look before it cost, and
     * a look of its own would replace that with the cost of asking for it.
     */
    function show(next: Tab) {
        tab = next;

        if (next !== "stats") check();
    }

    /**
     * Configures the pipeline for every read that follows ([ADR-0025](../../../docs/adr/0025-debug-information-formats.md)).
     *
     * The options are pushed to the driver, which is what makes a module carry the debug
     * information the option names; what the tabs hold was built under the options before it,
     * so the values on the screen are read again. A push that changes nothing builds nothing:
     * the driver hands back whether the options moved.
     */
    async function configure(nextDebug: DebugInfo, nextOpt: OptLevel) {
        if (!driver) return;

        debug = nextDebug;
        opt = nextOpt;

        try {
            const changed = await driver.setOptions(debug, opt);

            if (changed)
                say(
                    "info",
                    `the options are ${debug} debug information and ${opt} optimization`,
                );

            check();
        } catch (error) {
            say("error", `the driver refused the options: ${String(error)}`);
        }
    }

    /**
     * Runs the program every buffer makes, and shows what it printed.
     *
     * A run is of the whole project: the driver compiles every module, links them into the
     * manifest of a run ([ADR-0021](../../../docs/adr/0021-translation-units.md)), and the
     * worker instantiates the modules and calls the entry point. What a person reads is the
     * program console, which is where a program that cannot be run says why.
     */
    async function run() {
        if (!driver) return;

        program = [{ level: "note", text: "running…" }];
        consoleTab = "program";
        view = "console";

        try {
            const result = await driver.run();
            const lines: Line[] = [
                ...result.diagnostics.map((it) => ({
                    level: "error" as const,
                    text: `${it.level}[${it.category}::${it.code}]: ${it.message}`,
                })),
                ...result.printed.map((it) => ({
                    level: "info" as const,
                    text: it,
                })),
            ];

            if (result.error)
                lines.push({ level: "error", text: result.error });

            if (lines.length === 0) {
                lines.push({
                    level: "note",
                    text: "the program printed nothing",
                });
            }

            program = lines;
        } catch (error) {
            program = [
                {
                    level: "error",
                    text: `the driver refused the run: ${String(error)}`,
                },
            ];
        }
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
        <button class="tool" data-run onclick={run}>Run</button>
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
                onclick={() => show("diagnostics")}
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
                onclick={() => show("cst")}>CST</button
            >
            <button
                data-tab="ast"
                class:active={tab === "ast"}
                onclick={() => show("ast")}>AST</button
            >
            <button
                data-tab="hir"
                class:active={tab === "hir"}
                onclick={() => show("hir")}>HIR</button
            >
            <button
                data-tab="tc"
                class:active={tab === "tc"}
                onclick={() => show("tc")}
            >
                TC
                {#if untyped > 0}
                    <span class="badge error">{untyped}</span>
                {/if}
            </button>
            <button
                data-tab="mir"
                class:active={tab === "mir"}
                onclick={() => show("mir")}>MIR/CFG</button
            >
            <button
                data-tab="mir-ssa"
                class:active={tab === "mir-ssa"}
                onclick={() => show("mir-ssa")}>MIR/SSA</button
            >
            <button
                data-tab="lir"
                class:active={tab === "lir"}
                onclick={() => show("lir")}>LIR</button
            >
            <button
                data-tab="wat"
                class:active={tab === "wat"}
                onclick={() => show("wat")}>WAT</button
            >
            <button
                data-tab="stats"
                class:active={tab === "stats"}
                onclick={() => show("stats")}>Stats</button
            >
            <button
                data-tab="config"
                class:active={tab === "config"}
                onclick={() => show("config")}>Config</button
            >
        </nav>

        <div class="view">
            {#if tab === "diagnostics"}
                <Diagnostics {diagnostics} text={find(active)?.text ?? ""} />
            {:else if tab === "config"}
                <ConfigView
                    {debug}
                    {opt}
                    onDebug={(next) => configure(next, opt)}
                    onOpt={(next) => configure(debug, next)}
                />
            {:else if !reading}
                <p class="empty">Waiting for a parse.</p>
            {:else if tab === "cst"}
                {#if reading.cst === undefined}
                    <p class="empty">Reading the tree.</p>
                {:else}
                    <TreeView
                        node={reading.cst}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if tab === "ast"}
                {#if reading.ast === undefined}
                    <p class="empty">Reading the tree.</p>
                {:else if reading.ast === null}
                    <p class="empty">The root of the tree is not a module.</p>
                {:else}
                    <AstView
                        value={reading.ast}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if tab === "tc"}
                {#if reading.types === null}
                    <p class="empty">There is nothing to check.</p>
                {:else}
                    <TypeView
                        types={reading.types}
                        text={find(active)?.text ?? ""}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if tab === "mir"}
                {#if reading.mir === undefined}
                    <p class="empty">Reading the MIR.</p>
                {:else if reading.mir === null}
                    <p class="empty">There is nothing to lower.</p>
                {:else if reading.mir.bodies.length === 0}
                    <p class="empty">
                        No body of the module checks clean, so none has MIR.
                    </p>
                {:else}
                    <MirView
                        mir={reading.mir}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if tab === "mir-ssa"}
                {#if reading.mirSsa === undefined}
                    <p class="empty">Reading the MIR.</p>
                {:else if reading.mirSsa === null}
                    <p class="empty">There is nothing to lower.</p>
                {:else if reading.mirSsa.bodies.length === 0}
                    <p class="empty">
                        No body of the module checks clean, so none has MIR.
                    </p>
                {:else}
                    <MirView
                        mir={reading.mirSsa}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if tab === "lir"}
                {#if reading.lir === undefined}
                    <p class="empty">Lowering the body.</p>
                {:else if reading.lir === null}
                    <p class="empty">
                        No body of the module checks clean, so none has a LIR.
                    </p>
                {:else if reading.lir.bodies.length === 0}
                    <p class="empty">
                        No body of the module checks clean, so none has a LIR.
                    </p>
                {:else}
                    <LirView
                        lir={reading.lir}
                        onHover={pointed}
                        onPick={picked}
                    />
                {/if}
            {:else if tab === "wat"}
                {#if reading.wat === undefined}
                    <p class="empty">Compiling the module.</p>
                {:else if reading.wat === null}
                    <p class="empty">
                        No body of the module checks clean, so there is nothing
                        to compile.
                    </p>
                {:else}
                    <WatView wat={reading.wat} />
                {/if}
            {:else if tab === "stats"}
                <StatsView cost={reading.cost} />
            {:else if reading.hir === undefined}
                <p class="empty">Reading the module.</p>
            {:else if reading.hir === null}
                <p class="empty">There is nothing to lower.</p>
            {:else}
                {#each reading.hir.nodes as node, index (index)}
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
        <Console lines={log} {program} show={consoleTab} />
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
        flex-wrap: wrap;
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
        flex-wrap: wrap;
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
