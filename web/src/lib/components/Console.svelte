<script lang="ts" module>
    /** One line of what the console has to say for itself. */
    export interface Line {
        /** How to read it: what happened, what it is worth knowing, or what went wrong. */
        level: "info" | "note" | "error";

        /** The line itself. */
        text: string;
    }
</script>

<script lang="ts">
    interface Props {
        /** What the compiler said, oldest first. */
        lines: Line[];

        /** What the program said, oldest first: running it has no code generator yet. */
        program?: Line[];
    }

    let { lines, program = [] }: Props = $props();

    /** Which of the two consoles a person is reading. */
    let tab = $state<"compiler" | "program">("compiler");

    let view = $state<HTMLDivElement | undefined>();

    const shown = $derived(tab === "compiler" ? lines : program);

    /** A console shows the newest line: it scrolls itself to the bottom. */
    $effect(() => {
        if (shown.length > 0 && view) view.scrollTop = view.scrollHeight;
    });
</script>

<div class="console" data-panel="console">
    <nav class="tabs">
        <button
            data-console="compiler"
            class:active={tab === "compiler"}
            onclick={() => (tab = "compiler")}
        >
            Compiler
        </button>
        <button
            data-console="program"
            class:active={tab === "program"}
            onclick={() => (tab = "program")}
        >
            Program
        </button>
    </nav>

    <div class="lines" bind:this={view}>
        {#each shown as line, index (index)}
            <div class="line {line.level}">{line.text}</div>
        {:else}
            <p class="empty">
                {tab === "compiler"
                    ? "Nothing yet."
                    : "Nothing yet: the program runs when the compiler can make one."}
            </p>
        {/each}
    </div>
</div>

<style>
    .console {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
        background: var(--surface);
        border-top: 1px solid var(--border);
    }

    .tabs {
        display: flex;
        gap: 0.25rem;
        padding: 0 0.4rem;
        border-bottom: 1px solid var(--border);
    }

    .tabs button {
        padding: 0.25rem 0.45rem;
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

    .lines {
        flex: 1;
        min-height: 0;
        padding: 0.35rem 0.75rem;
        overflow: auto;
        font-family: var(--mono);
        font-size: 12px;
    }

    .line {
        color: var(--muted);
        white-space: pre-wrap;
    }

    .line.info {
        color: var(--text);
    }

    .line.error {
        color: var(--error);
    }

    .empty {
        margin: 0;
        color: var(--muted);
    }
</style>
