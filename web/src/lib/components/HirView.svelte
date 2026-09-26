<script lang="ts">
    import type { HirNode } from "$lib/driver";
    import HirView from "./HirView.svelte";

    interface Props {
        /** The line to show, with everything under it. */
        node: HirNode;

        /**
         * Called with the ranges a line stands for while a pointer is on it, and with `null`
         * once the pointer leaves it: the editor marks them in the text.
         *
         * What a line stands for is the code it is written as, and — when it is a path that
         * named something — the code the name comes from.
         */
        onHover: (
            range: [number, number] | null,
            resolves?: [number, number] | null,
        ) => void;
    }

    let { node, onHover }: Props = $props();

    /** Whether the line shows what it holds: a person folds what they are not reading. */
    let open = $state(true);

    const children = $derived(node.children);

    /**
     * What a line tells the page about the pointer: the part of the source it stands for, and
     * the part of the source what it names is written at.
     *
     * A line says what a node of the HIR is, and the code says what it was written as, so
     * a pointer on one is a question about the other: the editor marks that part of the
     * buffer while the pointer is on the line. A line that is about nothing a module wrote —
     * a section, a field of a declaration — says as much, and nothing is marked.
     */
    const pointing = (
        at: [number, number] | null,
        names: [number, number] | null,
    ) => ({
        onmouseenter: () => onHover(at, names),
        onmouseleave: () => onHover(null),
    });
</script>

<div class="line" data-kind={node.kind}>
    {#if children.length > 0}
        <button
            class="row"
            {...pointing(node.range, node.resolves)}
            onclick={() => (open = !open)}
            aria-expanded={open}
        >
            <span class="caret">{open ? "▾" : "▸"}</span>
            <span class="text">{node.text}</span>
            <span class="count">{children.length}</span>
        </button>

        {#if open}
            <div class="children">
                {#each children as child, index (index)}
                    <HirView node={child} {onHover} />
                {/each}
            </div>
        {/if}
    {:else}
        <div class="row" {...pointing(node.range, node.resolves)}>
            <span class="spacer"></span>
            <span class="text">{node.text}</span>
        </div>
    {/if}
</div>

<style>
    .line {
        font-family: var(--mono);
        font-size: 12px;
        white-space: nowrap;
    }

    .children {
        margin-left: 0.55rem;
        padding-left: 0.75rem;
        border-left: 1px solid var(--border);
    }

    .row {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
        width: 100%;
        text-align: left;
    }

    .row:hover {
        background: var(--raised);
    }

    .caret,
    .spacer {
        width: 0.75rem;
        color: var(--muted);
    }

    .count {
        color: var(--muted);
        font-size: 11px;
    }

    .count::before {
        content: "· ";
    }

    /* What a line says, painted by what it is: the module and its items are what a reader
       looks for, a section is a word about the lines under it, and a field is what an item
       is rather than the item itself. */
    .line[data-kind="module"] .text {
        color: var(--accent);
        font-weight: 600;
    }

    .line[data-kind="section"] .text {
        color: var(--muted);
        letter-spacing: 0.06em;
    }

    .line[data-kind="item"] .text {
        color: var(--accent);
    }

    .line[data-kind="field"] .text {
        color: var(--muted);
    }

    .line[data-kind="path"] .text {
        color: var(--ok);
    }
</style>
