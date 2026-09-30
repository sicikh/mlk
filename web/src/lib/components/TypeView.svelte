<script lang="ts">
    import type { Types } from "$lib/driver";
    import { utf16At } from "$lib/offsets";

    interface Props {
        /** What checking the types of the buffer left. */
        types: Types;

        /** The text the types are about: a row shows the code it is a type of. */
        text: string;

        /**
         * Called with the range a row stands for while a pointer is on it, and with `null` once
         * the pointer leaves it: the editor marks that part of the buffer.
         */
        onHover: (range: [number, number] | null) => void;

        /** Called when a row is picked, which is a request to be shown where it is written. */
        onPick: (range: [number, number] | null) => void;
    }

    let { types, text, onHover, onPick }: Props = $props();

    /**
     * The type a pointer is on, so that every row that reads the same is read together.
     *
     * A type is what two places have in common, and two rows that read the same are two places
     * of one type: pointing at one is asking about the other. Rows are compared by what they
     * read as, which is what a person sees, and a mistake is a type of its own.
     */
    let pointed = $state<string | null>(null);

    /** What a row says of a pointer: the type it is, and the code it is the type of. */
    function pointing(range: [number, number] | null, ty: string) {
        return {
            onmouseenter: () => {
                pointed = ty;
                onHover(range);
            },
            onfocus: () => {
                pointed = ty;
                onHover(range);
            },
            onmouseleave: clear,
            onblur: clear,
        };
    }

    /** The pointer is on nothing, and the rows that read together are read as they were. */
    function clear() {
        pointed = null;
        onHover(null);
    }

    /**
     * The code a row stands for, as the row reads it: the source, on one line.
     *
     * A node of a body may cover several lines, and a row is one line of a list: the code is
     * read as the pieces of it, and the whole of it is what a pointer and a pick hand over.
     */
    function codeOf(range: [number, number] | null): string {
        if (!range) return "";

        const from = utf16At(text, range[0]);
        const to = Math.max(from, utf16At(text, range[1]));

        return text
            .slice(from, to)
            .trim()
            .replace(/\s+/g, " ");
    }
</script>

{#if types.surface.length === 0 && types.bodies.length === 0}
    <p class="none">Nothing of the module was checked.</p>
{:else}
    <section class="part" data-tc="surface">
        <h2>Surface</h2>

        {#if types.surface.length === 0}
            <p class="none">No entity of the module declares a type.</p>
        {:else}
            {#each types.surface as entity, index (index)}
                {@const code = codeOf(entity.range)}
                <button
                    class="row"
                    class:same={entity.ty !== "{error}" && pointed === entity.ty}
                    data-tc="entity"
                    title={code === "" ? entity.name : code}
                    {...pointing(entity.range, entity.ty)}
                    onclick={() => onPick(entity.range)}
                >
                    <span class="saying">{entity.name}</span>
                    <span class:error={entity.ty.includes("{error}")} class="ty"
                        >{entity.ty}</span
                    >
                </button>
            {/each}
        {/if}
    </section>

    {#each types.bodies as body, index (index)}
        <section class="part" data-tc="body">
            <button
                class="owner"
                data-tc="owner"
                title={body.owner}
                {...pointing(body.range, "")}
                onclick={() => onPick(body.range)}
            >
                {body.owner}
            </button>

            {#each body.nodes as node, at (at)}
                {@const code = codeOf(node.range)}
                <button
                    class="row"
                    class:same={node.ty !== "{error}" && pointed === node.ty}
                    data-tc="node"
                    data-kind={node.kind}
                    data-error={node.error}
                    title={code === "" ? node.label : code}
                    {...pointing(node.range, node.ty)}
                    onclick={() => onPick(node.range)}
                >
                    <span class="kind">{node.kind}</span>
                    <span class="saying">{code === "" ? node.label : code}</span>
                    <span class:error={node.ty.includes("{error}")} class="ty"
                        >{node.ty}</span
                    >
                </button>
            {/each}
        </section>
    {/each}
{/if}

<style>
    .part {
        padding: 0.35rem 0.75rem 0.5rem;
        border-bottom: 1px solid var(--border);
    }

    h2 {
        margin: 0 0 0.25rem;
        color: var(--muted);
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
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

    /* A row is a place and the type it has: the code is what a person reads in the buffer, and
       the type is what the compiler says of it. The code is cut where the type begins, so the
       types of a part line up however long the code is. */
    .row {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
        width: 100%;
        padding: 0.05rem 0;
        font-family: var(--mono);
        font-size: 12px;
        white-space: nowrap;
    }

    .row:hover,
    .row.same {
        background: var(--raised);
    }

    /* What two rows of one type have in common is said once, by the place the pointer is on. */
    .row.same {
        box-shadow: inset 2px 0 0 var(--accent);
    }

    .kind {
        flex: none;
        width: 2.1rem;
        color: var(--muted);
        font-size: 10px;
    }

    .saying {
        flex: 1;
        overflow: hidden;
        color: var(--text);
        text-overflow: ellipsis;
    }

    .ty {
        flex: none;
        max-width: 55%;
        overflow: hidden;
        color: var(--type);
        text-overflow: ellipsis;
    }

    .error {
        color: var(--error);
    }

    .none {
        margin: 0;
        color: var(--muted);
    }

    /* A phone reads a list with a finger, and a finger wants a row it can land on. */
    @media (max-width: 860px), (max-height: 520px) {
        .row,
        .owner {
            padding: 0.15rem 0;
        }
    }
</style>
