<script lang="ts">
    import type { Mir, MirBlock, MirBody, MirLocal } from "$lib/driver";

    interface Props {
        /** The MIR of the buffer, in one of its two forms: the CFG form or the SSA form. */
        mir: Mir;

        /**
         * Called with the range a line stands for while a pointer is on it, and with `null` once
         * the pointer leaves it: the editor marks that part of the buffer.
         */
        onHover: (range: [number, number] | null) => void;

        /** Called when a line is picked, which is a request to be shown where it is written. */
        onPick: (range: [number, number] | null) => void;
    }

    let { mir, onHover, onPick }: Props = $props();

    /**
     * What a line tells the page about the pointer: the part of the source it stands for.
     *
     * A line of the MIR is a statement or a value the lowering read, and the driver says where
     * it was read from: pointing at the line marks that place in the buffer, and picking it is
     * a request to be shown it.
     */
    function pointing(range: [number, number] | null) {
        return {
            onmouseenter: () => onHover(range),
            onfocus: () => onHover(range),
            onmouseleave: () => onHover(null),
            onblur: () => onHover(null),
        };
    }

    /** The label of a block, by its position: `b0`. */
    function label(at: number): string {
        return `b${at}`;
    }

    /** What a slot is called where it is written: `l0(x)`, or `l0` when it was bound nowhere. */
    function slot(local: MirLocal): string {
        return local.name === null
            ? local.label
            : `${local.label}(${local.name})`;
    }

    /**
     * The blocks a block is entered from and the ones it goes to, as one line reads them.
     *
     * A terminator says where control goes, and nothing says where it comes from: the edges are
     * what a person reads a graph by, and a block of several predecessors is a join.
     */
    function edges(block: MirBlock): string {
        const into = block.predecessors.map(label);
        const out = block.successors.map(label);

        return [
            into.length > 0 ? `← ${into.join(", ")}` : "",
            out.length > 0 ? `→ ${out.join(", ")}` : "",
        ]
            .filter((it) => it !== "")
            .join("  ");
    }
</script>

{#snippet body(view: MirBody)}
    <section class="body">
        <button
            class="owner"
            data-line="owner"
            title={view.owner}
            {...pointing(view.range)}
            onclick={() => onPick(view.range)}
        >
            {view.owner}
            <span class="entry">entry {label(view.entry)}</span>
        </button>

        {#if view.params.length > 0}
            <p class="params" data-line="params">
                <span class="tag">params</span>
                {#each view.params as value, at (at)}
                    <button
                        class="value"
                        data-line="value"
                        title="{value.label}: {value.ty}"
                        {...pointing(value.range)}
                        onclick={() => onPick(value.range)}
                    >
                        {value.label}<span class="ty">: {value.ty}</span>
                    </button>
                {/each}
            </p>
        {/if}

        {#if view.locals.length > 0}
            <p class="params" data-line="locals">
                <span class="tag">locals</span>
                {#each view.locals as local, at (at)}
                    <button
                        class="value"
                        data-line="local"
                        title="{slot(local)}: {local.ty}"
                        {...pointing(local.range)}
                        onclick={() => onPick(local.range)}
                    >
                        {slot(local)}<span class="ty">: {local.ty}</span>
                    </button>
                {/each}
            </p>
        {/if}

        {#each view.blocks as block, at (at)}
            {@const relation = edges(block)}
            <div
                class="block"
                data-block={block.label}
                data-entry={at === view.entry ? "true" : "false"}
            >
                <p class="head">
                    <span class="label"
                        >{block.label}{#if block.params.length > 0}({#each block.params as value, i (i)}<button
                                    class="value"
                                    data-line="value"
                                    title="{value.label}: {value.ty}"
                                    {...pointing(value.range)}
                                    onclick={() => onPick(value.range)}
                                    >{value.label}</button
                                >{#if i < block.params.length - 1}<span
                                        class="comma"
                                        >,
                                    </span>{/if}{/each}){/if}<span class="colon"
                            >:</span
                        ></span
                    >
                    {#if relation !== ""}<span class="edges">{relation}</span
                        >{/if}
                </p>

                {#each block.stmts as stmt, i (i)}
                    <button
                        class="line"
                        data-line="stmt"
                        data-kind={stmt.kind}
                        title={stmt.text}
                        {...pointing(stmt.range)}
                        onclick={() => onPick(stmt.range)}
                    >
                        <span class="saying">{stmt.text}</span>
                    </button>
                {/each}

                <button
                    class="line term"
                    data-line="term"
                    data-kind={block.term.kind}
                    title={block.term.text}
                    {...pointing(block.term.range)}
                    onclick={() => onPick(block.term.range)}
                >
                    <span class="saying">{block.term.text}</span>
                </button>
            </div>
        {/each}

        {#if view.lambdas.length > 0}
            <div class="lambdas" data-lambdas>
                {#each view.lambdas as child (child.owner)}
                    {@render body(child)}
                {/each}
            </div>
        {/if}

        {#if view.localFunctions.length > 0}
            <div class="lambdas" data-locals>
                {#each view.localFunctions as child, i (i)}
                    {@render body(child)}
                {/each}
            </div>
        {/if}
    </section>
{/snippet}

<div class="bodies" data-form={mir.form}>
    {#each mir.bodies as view, at (at)}
        {@render body(view)}
    {/each}
</div>

<style>
    .bodies {
        font-family: var(--mono);
        font-size: 12px;
    }

    .body {
        padding: 0.35rem 0.75rem 0.5rem;
        border-bottom: 1px solid var(--border);
    }

    /* A lambda or a function declared in a `local` is a body written in another one: its section
       stands inside its writer, and the line at its left is what says so. */
    .lambdas {
        margin: 0.35rem 0 0 0.25rem;
        border-left: 1px solid var(--border);
    }

    .lambdas .body {
        padding-left: 0.6rem;
        border-bottom: none;
    }

    .owner {
        display: block;
        width: 100%;
        padding: 0.15rem 0;
        color: var(--accent);
        font-family: var(--mono);
        font-size: 12px;
        font-weight: 600;
        text-align: left;
    }

    .owner:hover {
        background: var(--raised);
    }

    .entry {
        margin-left: 0.5rem;
        color: var(--muted);
        font-size: 10px;
        font-weight: 400;
        letter-spacing: 0.06em;
        text-transform: uppercase;
    }

    .params {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
        margin: 0;
        padding: 0.1rem 0;
    }

    .tag {
        color: var(--muted);
        font-size: 10px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
    }

    .block {
        padding-left: 0.75rem;
    }

    /* A block is headed by its label and the values it is entered through, and the edges
       around it say where a person is in the graph. */
    .head {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
        margin: 0;
        padding: 0.2rem 0 0.1rem;
        white-space: nowrap;
    }

    .label {
        color: var(--accent);
        font-weight: 600;
    }

    .comma,
    .colon {
        color: var(--muted);
        font-weight: 400;
    }

    .edges {
        overflow: hidden;
        color: var(--muted);
        font-size: 10px;
        text-overflow: ellipsis;
    }

    /* A value is what the compiler calls a parameter or a definition: a label, and the type
       the checker gave the expression it stands for. */
    .value {
        padding: 0;
        color: var(--ok);
        font-family: var(--mono);
        font-size: 12px;
    }

    .value:hover {
        background: var(--raised);
    }

    .value .ty {
        color: var(--type);
    }

    /* A statement reads as one line of the block it belongs to; the terminator is the line
       that says where control goes, and it is painted apart. */
    .line {
        display: flex;
        align-items: baseline;
        width: 100%;
        padding-left: 0.75rem;
        text-align: left;
    }

    .line:hover {
        background: var(--raised);
    }

    .saying {
        flex: 1;
        overflow: hidden;
        color: var(--text);
        white-space: nowrap;
        text-overflow: ellipsis;
    }

    .term .saying {
        color: var(--type);
    }

    .term[data-kind="unreachable"] .saying {
        color: var(--muted);
    }

    /* A phone reads a list with a finger, and a finger wants a row it can land on. */
    @media (max-width: 860px), (max-height: 520px) {
        .line,
        .owner {
            padding-top: 0.15rem;
            padding-bottom: 0.15rem;
        }
    }
</style>
