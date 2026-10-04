<script lang="ts">
    import { onMount, tick } from "svelte";

    import { archive, type ArchivedFile } from "$lib/archive";
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

    let driver = $state.raw<Driver | null>(null);

    const diagnostics = $derived(reading?.diagnostics ?? []);
    const errors = $derived(
        diagnostics.filter((it) => it.level === "error").length,
    );
    const warnings = $derived(
        diagnostics.filter((it) => it.level === "warning").length,
    );
    const notes = $derived(
        diagnostics.filter((it) => it.level === "note" || it.level === "help")
            .length,
    );

    /**
     * How bad the buffer is at worst, which is what the dot of the status line says.
     *
     * A diagnostic that is not one of the levels a person reads is still something the
     * compiler reported, so it counts as a note rather than disappearing.
     */
    const health = $derived(
        errors > 0
            ? "error"
            : warnings > 0
              ? "warning"
              : diagnostics.length > 0
                ? "note"
                : "ok",
    );

    /** What the status line counts, in the order a person reads it. */
    const report = $derived.by(() => {
        const count = (n: number, one: string) =>
            `${n} ${n === 1 ? one : one + "s"}`;
        const parts: string[] = [];

        if (errors > 0) parts.push(count(errors, "error"));
        if (warnings > 0) parts.push(count(warnings, "warning"));
        if (notes > 0) parts.push(count(notes, "note"));

        return parts.length > 0 ? parts.join(" · ") : "no diagnostics";
    });

    /**
     * Why the build cannot begin, when it cannot: a tool that is read rather than pressed
     * says what stands in the way of it.
     *
     * The errors are the ones the buffer in front reports, and they are read as the errors of
     * the project: a body the checker could not type is a body the pipeline refuses to lower,
     * and the module it belongs to is a module the link stage refuses to link ([ADR-0019]).
     *
     * [adr-0019]: ../../../docs/adr/0019-mir.md
     */
    const buildTitle = $derived(
        driver === null
            ? "the driver is loading"
            : errors > 0
              ? `${errors} error${errors === 1 ? " stands" : "s stand"} in the way of a build`
              : "Compile the project and download its modules",
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

    /**
     * Checks every buffer: what a look does for the buffer in front, said out loud for all of
     * them, with the counters each read spent.
     */
    async function checkProject() {
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
     * Compiles the project and hands it to a person as an archive ([ADR-0021]).
     *
     * A build is of the whole project rather than of the buffer in front: the driver compiles
     * every module of it, the modules of the library included, links them into the manifest a
     * run is of, and instantiates nothing. What a person gets is one ZIP holding a `.wasm` per
     * module, under the file its manifest names, with the module of the host functions and the
     * manifest itself beside them: a browser has no file system, and an archive is the one
     * file a download can carry the folders of a project in.
     *
     * [adr-0021]: ../../../docs/adr/0021-translation-units.md
     */
    async function compile() {
        if (!driver) return;

        say("note", "building the project…");

        try {
            const program = await driver.build();
            const manifest = program.manifest;
            const held = new Map(
                program.modules.map((it) => [it.name, it.bytes]),
            );
            const files: ArchivedFile[] = manifest.modules.map((it) => {
                const bytes = held.get(it.name);

                if (!bytes)
                    throw new Error(
                        `the manifest names the module \`${it.name}\`, and the build holds none`,
                    );

                return { path: it.file, bytes };
            });

            if (manifest.host !== null)
                files.push({ path: manifest.host, bytes: program.host });

            // The manifest travels beside the modules, under the name a host reads it by.
            files.push({
                path: "manifest.json",
                bytes: new TextEncoder().encode(
                    `${JSON.stringify(manifest, null, 4)}\n`,
                ),
            });

            for (const diagnostic of program.diagnostics)
                say(
                    diagnostic.level === "error" ? "error" : "note",
                    `${diagnostic.level}[${diagnostic.category}::${diagnostic.code}]: ${diagnostic.message}`,
                );

            const named = `${pathOf(manifest.project)}.zip`;

            save(named, archive(files));

            say(
                "note",
                `the build is ${files.length} file${files.length === 1 ? "" : "s"} in ${named}: ${files
                    .map((it) => it.path)
                    .join(", ")}`,
            );

            for (const problem of program.problems) say("error", problem);

            consoleTab = "compiler";
            view = "console";
        } catch (error) {
            say("error", `the driver refused the build: ${String(error)}`);
            tab = "diagnostics";
            view = "inspector";
        }
    }

    /**
     * Hands the sources of the project to a person as an archive ([ADR-0025]).
     *
     * A debugger that reads DWARF reads a path where a source stands, and the paths of the
     * language are virtual: `/main.mlk` is not a file on a disk. The entries of the archive are
     * those paths without their root --- `main.mlk`, `std/core.mlk` --- so unpacking it into a
     * directory and mapping `/` to that directory is the whole of the setup, and the same rule
     * in every debugger.
     *
     * What goes in is every buffer the editor holds, the library included: it is what the
     * modules of the build were compiled from, and stepping into a function of it should find
     * the source it was written in.
     *
     * [adr-0025]: ../../../docs/adr/0025-debug-information-formats.md
     */
    async function sources() {
        if (!driver) return;

        try {
            const project = await driver.project();
            const encoder = new TextEncoder();
            const files: ArchivedFile[] = [];
            const seen = new Set<string>();

            for (const it of buffers) {
                const path = it.path.replace(/^\/+/, "");

                if (path === "" || seen.has(path)) continue;

                seen.add(path);
                files.push({ path, bytes: encoder.encode(it.text) });
            }

            files.sort((left, right) =>
                left.path < right.path ? -1 : left.path > right.path ? 1 : 0,
            );

            const named = `${pathOf(project)}.sources.zip`;

            save(named, archive(files));

            say(
                "note",
                `the sources are ${files.length} file${files.length === 1 ? "" : "s"} in ${named}: ${files
                    .map((it) => it.path)
                    .join(", ")}`,
            );
            say(
                "note",
                "unpack them and point a debugger at the directory: lldb `settings set target.source-map / DIR`, gdb `set substitute-path / DIR`",
            );

            consoleTab = "compiler";
            view = "console";
        } catch (error) {
            say("error", `the driver refused the sources: ${String(error)}`);
        }
    }

    /**
     * A canonical name as a path: `app::main` is `app/main`, and `app` is `app`.
     *
     * A canonical name is the project and the path of the module inside it, and a name a file
     * system reads is a path under a directory. Anything a path may not hold becomes a dash,
     * so a name can never name another directory, and a name of nothing is named rather than
     * nameless.
     */
    function pathOf(name: string): string {
        const path = name
            .replace(/::/g, "/")
            .replace(/[^A-Za-z0-9._/-]/g, "-")
            .replace(/^\/+/, "");

        return path || "module";
    }

    /**
     * Hands one file to a person as a download.
     *
     * A page has no file system, and the way it gives a file is the download of a link: the
     * bytes are a blob, the link stands in the document while it is taken, and the URL is let
     * go of after it, so a build does not hold its own bytes alive.
     */
    function save(file: string, bytes: Uint8Array) {
        const url = URL.createObjectURL(
            new Blob([new Uint8Array(bytes)], { type: "application/zip" }),
        );
        const link = document.createElement("a");

        link.href = url;
        link.download = file;
        link.hidden = true;
        document.body.appendChild(link);
        link.click();
        link.remove();
        setTimeout(() => URL.revokeObjectURL(url));
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
     * The row of the stages, and whether it has more to show on either side.
     *
     * The row is one line that scrolls, and a fade at an end of it says that something is out
     * of sight on that side; the fades are drawn only on a side that hides something.
     */
    let stages = $state<HTMLElement | undefined>(undefined);
    let stagesStart = $state(false);
    let stagesEnd = $state(false);

    /** Reads how much of the row of stages is out of sight. */
    function measureStages() {
        if (!stages) return;

        stagesStart = stages.scrollLeft > 1;
        stagesEnd =
            stages.scrollLeft + stages.clientWidth < stages.scrollWidth - 1;
    }

    $effect(() => {
        if (!stages) return;

        // The row changes with the panel a splitter sizes and with the pipeline itself, so
        // what is out of sight is measured whenever the row does.
        const observer = new ResizeObserver(measureStages);

        observer.observe(stages);
        measureStages();

        return () => observer.disconnect();
    });

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

        // The stages are one line that scrolls: what was picked is brought into view, so a
        // pick of a stage that stands past the edge is seen where a person is looking.
        void tick().then(() => {
            document
                .querySelector(`[data-tab="${next}"]`)
                ?.scrollIntoView({ block: "nearest", inline: "nearest" });
            measureStages();
        });

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

    /**
     * The chords of the header: `Ctrl`/`Cmd` with `Enter` runs the program, and the same with
     * `Shift` checks the project.
     *
     * A chord is taken in the capture phase, before the editor reads the key: `Ctrl+Enter` is
     * a key of the editor's own, and a person asking for a run asks for the run rather than
     * for the line the editor would open.
     */
    function chord(event: KeyboardEvent) {
        if (!(event.ctrlKey || event.metaKey) || event.key !== "Enter") return;

        event.preventDefault();
        event.stopPropagation();

        if (event.shiftKey) checkProject();
        else run();
    }
</script>

<svelte:head>
    <title>MLK editor</title>
</svelte:head>

<!-- A chord is of the page rather than of a panel, and it is taken before the panels read it. -->
<svelte:window onkeydowncapture={chord} />

<div
    class="ide"
    data-view={view}
    style="--files: {files}px; --inspector: {inspector}px; --console: {console}px"
>
    <!--
        What a person reaches for from anywhere: the tools of the project, and not the name of
        the buffer, which is on its tab in the editor. A tool is a mark and a word, and the word
        is what a narrow header runs out of room for first (see the styles).
    -->
    <header class="top">
        <span class="brand">MLK</span>
        <span class="grow"></span>
        <button
            class="tool"
            data-check
            aria-label="Check the project"
            title="Check every buffer (Ctrl+Shift+Enter)"
            onclick={checkProject}
        >
            <svg
                class="mark"
                viewBox="0 0 24 24"
                width="13"
                height="13"
                fill="none"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
                focusable="false"
            >
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path
                    d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                />
            </svg>
            <span class="label">Check</span>
        </button>
        <button
            class="tool"
            data-compile
            aria-label="Compile the project"
            title={buildTitle}
            disabled={!driver || errors > 0}
            onclick={compile}
        >
            <svg
                class="mark"
                viewBox="0 0 24 24"
                width="13"
                height="13"
                fill="none"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
                focusable="false"
            >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            <span class="label">Compile</span>
        </button>
        <button
            class="tool"
            data-sources
            aria-label="Download the sources of the project"
            title="Download the sources as an archive, to point a debugger at them"
            disabled={!driver}
            onclick={sources}
        >
            <svg
                class="mark"
                viewBox="0 0 24 24"
                width="13"
                height="13"
                fill="none"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
                focusable="false"
            >
                <path
                    d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"
                />
                <path d="M14 2v4a2 2 0 0 0 2 2h4" />
                <path d="m10 13-2 2 2 2" />
                <path d="m14 17 2-2-2-2" />
            </svg>
            <span class="label">Sources</span>
        </button>
        <button
            class="tool primary"
            data-run
            aria-label="Run the program"
            title="Run the program (Ctrl+Enter)"
            onclick={run}
        >
            <svg
                class="mark"
                viewBox="0 0 24 24"
                width="13"
                height="13"
                fill="currentColor"
                aria-hidden="true"
                focusable="false"
            >
                <path d="M8 5v14l11-7z" />
            </svg>
            <span class="label">Run</span>
        </button>
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
        <!--
            What the compiler reported and what it is asked for: these two are read at a
            glance whatever stage is shown, so they stand in a line of their own that does not
            scroll with the stages.
        -->
        <nav
            class="head"
            aria-label="What the compiler reported and how it is configured"
        >
            <button
                class="status"
                class:clear={diagnostics.length === 0}
                data-tab="diagnostics"
                class:active={tab === "diagnostics"}
                title="What the parser, the checks, and the back end reported"
                onclick={() => show("diagnostics")}
            >
                <span
                    class="dot"
                    class:error={health === "error"}
                    class:warning={health === "warning"}
                    class:note={health === "note"}
                    class:ok={health === "ok"}
                ></span>
                {report}
            </button>
            <button
                class="config"
                data-tab="config"
                class:active={tab === "config"}
                aria-label="Config"
                title="What the compiler is asked for"
                onclick={() => show("config")}
            >
                <svg
                    viewBox="0 0 24 24"
                    width="14"
                    height="14"
                    fill="currentColor"
                    aria-hidden="true"
                    focusable="false"
                >
                    <path
                        d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.03 7.03 0 0 0-1.62-.94l-.36-2.54a.48.48 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96a.48.48 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"
                    />
                </svg>
            </button>
        </nav>

        <!--
            The stages of the pipeline: one line, in the order the compiler runs them. The row
            scrolls rather than wraps, so the panel does not grow as more stages appear, and a
            fade at an end of it says that something is out of sight on that side.
        -->
        <div class="stages">
            <nav
                bind:this={stages}
                onscroll={measureStages}
                aria-label="The stages of the pipeline"
            >
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
            </nav>

            {#if stagesStart}
                <span class="fade start" aria-hidden="true"></span>
            {/if}
            {#if stagesEnd}
                <span class="fade end" aria-hidden="true"></span>
            {/if}
        </div>

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

    .empty {
        margin: 0;
        padding: 1rem;
        color: var(--muted);
    }

    .empty code {
        font-family: var(--mono);
    }

    /*
     * The tools of the header: a mark and a word each. A tool is quiet until a pointer is on
     * it --- what a person reaches for from anywhere is the run, and it is the one tool painted
     * as the action rather than as a tool.
     */
    .tool {
        display: inline-flex;
        gap: 0.4rem;
        align-items: center;
        padding: 0.3rem 0.65rem;
        border: 1px solid transparent;
        border-radius: var(--radius);
        color: var(--muted);
        font-size: 12px;
    }

    .tool:hover:not(:disabled) {
        background: var(--raised);
        border-color: var(--border);
        color: var(--text);
    }

    /* A build the project cannot give is not a tool a person can ask: it is read, not pressed. */
    .tool:disabled {
        cursor: default;
        opacity: 0.4;
    }

    .tool.primary {
        background: #2a3f63;
        border-color: #35507d;
        color: #d8e6ff;
    }

    .tool.primary:hover:not(:disabled) {
        background: #33507f;
    }

    .mark {
        flex: none;
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

    /*
     * The head of the inspector: what the compiler reported and how it is configured. It does
     * not scroll with the stages --- a person reads it at a glance whatever stage is shown.
     */
    .head {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 0.5rem;
        padding: 0 0.25rem;
        border-bottom: 1px solid var(--border);
    }

    .head button {
        display: flex;
        gap: 0.4rem;
        align-items: center;
        padding: 0.4rem 0.55rem;
        border-bottom: 2px solid transparent;
        color: var(--muted);
        font-size: 12px;
    }

    .head button.status {
        color: var(--text);
    }

    /* A buffer that reports nothing is the usual state, and it is said quietly. */
    .head button.status.clear {
        color: var(--muted);
    }

    .dot {
        flex: none;
        width: 0.45rem;
        height: 0.45rem;
        background: var(--muted);
        border-radius: 50%;
    }

    .dot.error {
        background: var(--error);
    }

    .dot.warning {
        background: var(--warning);
    }

    .dot.note {
        background: var(--accent);
    }

    .dot.ok {
        background: var(--ok);
    }

    /*
     * The stages of the pipeline: one line, in the order the compiler runs them. The row
     * scrolls rather than wraps, so the panel does not grow as more stages appear.
     */
    .stages {
        position: relative;
        flex: none;
        min-width: 0;
    }

    .stages nav {
        display: flex;
        overflow-x: auto;
        padding: 0 0.25rem;
        border-bottom: 1px solid var(--border);
        scrollbar-width: thin;
    }

    /*
     * A fade at an end of the row says that something is out of sight on that side. It is
     * painted over the buttons rather than stands beside them, so it takes no room of its own.
     */
    .fade {
        position: absolute;
        top: 0;
        bottom: 1px;
        width: 1.25rem;
        pointer-events: none;
    }

    .fade.start {
        left: 0;
        background: linear-gradient(to right, var(--surface), transparent);
    }

    .fade.end {
        right: 0;
        background: linear-gradient(to left, var(--surface), transparent);
    }

    .stages button {
        display: flex;
        flex: none;
        gap: 0.35rem;
        align-items: center;
        padding: 0.4rem 0.55rem;
        border-bottom: 2px solid transparent;
        color: var(--muted);
        font-size: 11px;
        letter-spacing: 0.03em;
        text-transform: uppercase;
    }

    .head button:hover,
    .stages button:hover {
        color: var(--text);
    }

    .head button.active,
    .stages button.active {
        border-bottom-color: var(--accent);
        color: var(--text);
    }

    /* The gear of the configuration: a mark rather than a word, so it is a square to land in. */
    .head button.config {
        padding: 0.4rem 0.7rem;
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
        .head button,
        .stages button {
            padding: 0.55rem 0.7rem;
        }

        /* A header is one line here, and the words of the tools are what it runs out of room
           for first: a tool keeps its mark, and its name is read by a reader of the label. */
        .tool .label {
            display: none;
        }

        .tool {
            min-height: 36px;
            padding: 0.35rem 0.7rem;
        }
    }
</style>
