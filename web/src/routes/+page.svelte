<script lang="ts">
    import { onMount } from "svelte";

    import { loadDriver, type Analysis, type Driver } from "$lib/driver";

    /**
     * A module that parses cleanly, so the trees have something to show
     * before anybody types.
     */
    const EXAMPLE = `fun main(): Unit =
    let x = 42 * 2 - 10 in
    println-int(x + 20)
`;

    /** The name of the buffer: a browser has no file system, so the editor invents one. */
    const PATH = "/main.mlk";

    let source = $state(EXAMPLE);
    let analysis = $state<Analysis | null>(null);
    let status = $state("loading the wasm driver…");

    let driver: Driver | null = null;

    onMount(async () => {
        try {
            driver = await loadDriver();
            status = "the driver is loaded";
            analyze();
        } catch (error) {
            status = `the driver did not load: ${String(error)}`;
        }
    });

    /**
     * Pushes the buffer into the driver and reads back everything it made of it.
     *
     * The driver compares the text it already holds with what it is handed ([ADR-0008]),
     * so a keystroke that changes nothing it cares about costs a comparison.
     */
    function analyze() {
        if (!driver) return;

        try {
            driver.push(PATH, source);
            analysis = driver.analyze(PATH);
        } catch (error) {
            status = `the driver refused the buffer: ${String(error)}`;
        }
    }

    function onInput(event: Event & { currentTarget: HTMLTextAreaElement }) {
        source = event.currentTarget.value;
        analyze();
    }

    let diagnostics = $derived(analysis?.diagnostics ?? []);
</script>

<svelte:head>
    <title>MLK editor</title>
</svelte:head>

<main>
    <header>
        <h1>MLK editor</h1>
        <p class="status">{status}</p>
    </header>

    <section class="panes">
        <div class="pane">
            <h2>Source</h2>
            <textarea value={source} oninput={onInput} spellcheck="false"
            ></textarea>
        </div>

        <div class="pane">
            <h2>
                Diagnostics
                {#if diagnostics.length > 0}
                    <span class="count">{diagnostics.length}</span>
                {/if}
            </h2>
            {#if !analysis}
                <p class="empty">Waiting for the driver.</p>
            {:else if diagnostics.length === 0}
                <p class="empty">No diagnostics.</p>
            {:else}
                <ul>
                    {#each diagnostics as diagnostic (diagnostic.code + diagnostic.message)}
                        <li class={diagnostic.level}>
                            <span class="code"
                                >{diagnostic.category}:{diagnostic.code}</span
                            >
                            {diagnostic.message}
                            {#each diagnostic.labels as label}
                                <span class="label">
                                    {label.line + 1}:{label.column +
                                        1}{label.message
                                        ? ` — ${label.message}`
                                        : ""}
                                </span>
                            {/each}
                            {#each diagnostic.notes as note}
                                <span class="note">{note}</span>
                            {/each}
                        </li>
                    {/each}
                </ul>
            {/if}
        </div>
    </section>

    <section class="panes">
        <div class="pane">
            <h2>AST</h2>
            <pre>{analysis?.ast ?? "…"}</pre>
        </div>

        <div class="pane">
            <h2>CST</h2>
            <pre>{analysis?.cst ?? "…"}</pre>
        </div>
    </section>
</main>

<style>
    :global(body) {
        margin: 0;
        background: #101418;
        color: #e6e6e6;
        font-family: ui-sans-serif, system-ui, sans-serif;
    }

    main {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        height: 100vh;
        box-sizing: border-box;
        padding: 1rem;
    }

    header {
        display: flex;
        align-items: baseline;
        gap: 1rem;
    }

    h1 {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
        letter-spacing: 0.02em;
        text-transform: uppercase;
    }

    .status {
        margin: 0;
        color: #8ba3b8;
        font-size: 0.85rem;
    }

    .panes {
        display: grid;
        flex: 1;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
        min-height: 0;
    }

    .pane {
        display: flex;
        flex-direction: column;
        min-height: 0;
        border: 1px solid #253038;
        border-radius: 0.4rem;
        overflow: hidden;
    }

    h2 {
        display: flex;
        gap: 0.5rem;
        align-items: center;
        margin: 0;
        padding: 0.4rem 0.6rem;
        background: #172027;
        border-bottom: 1px solid #253038;
        font-size: 0.75rem;
        font-weight: 600;
        letter-spacing: 0.04em;
        text-transform: uppercase;
    }

    .count {
        padding: 0 0.35rem;
        background: #4a2020;
        border-radius: 0.6rem;
        color: #ffb4b4;
        font-size: 0.7rem;
    }

    textarea,
    pre {
        flex: 1;
        margin: 0;
        padding: 0.6rem;
        overflow: auto;
        background: #0c1013;
        border: 0;
        color: inherit;
        font-family: ui-monospace, SFMono-Regular, monospace;
        font-size: 0.8rem;
        line-height: 1.5;
        white-space: pre;
    }

    textarea {
        resize: none;
        outline: none;
    }

    ul {
        margin: 0;
        padding: 0.6rem;
        overflow: auto;
        list-style: none;
        font-size: 0.8rem;
    }

    li {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
        padding: 0.4rem 0;
        border-bottom: 1px solid #1b242b;
    }

    .code {
        color: #ffb4b4;
        font-family: ui-monospace, SFMono-Regular, monospace;
    }

    .label,
    .note {
        color: #8ba3b8;
        font-family: ui-monospace, SFMono-Regular, monospace;
    }

    .empty {
        margin: 0;
        padding: 0.6rem;
        color: #6b7f8f;
        font-size: 0.8rem;
    }
</style>
