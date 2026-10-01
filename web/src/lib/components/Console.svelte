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

        /** What the program said, oldest first: what running it printed, and what happened. */
        program?: Line[];

        /**
         * Which console a host puts in front, when it has something to say in one of them.
         *
         * A person presses Run, and what they asked for is what the program says: the page
         * says so here, and a console that is already in front stays where it is.
         */
        show?: "compiler" | "program";
    }

    let { lines, program = [], show = "compiler" }: Props = $props();

    /** Which of the two consoles a person is reading. */
    let tab = $state<"compiler" | "program">("compiler");

    // A host that asks for one of the consoles is heard after the component is up as well:
    // the effect reads `show`, and a person picking a tab of their own is left where they are.
    $effect(() => {
        tab = show;
    });

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
                    : "Nothing yet: run the program to read what it says."}
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

    /* The console is a panel of its own here, and the switcher above it already draws the line. */
    @media (max-width: 860px), (max-height: 520px) {
        .console {
            border-top: none;
        }

        /* A finger picks a tab the way a pointer does, but it needs more room to land in. */
        .tabs button {
            padding: 0.55rem 0.6rem;
        }
    }
</style>
