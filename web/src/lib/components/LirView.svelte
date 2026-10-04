<script lang="ts">
    import type {
        Lir,
        LirBlock,
        LirBody,
        LirLocal,
        LirStructureLine,
        LirValue,
    } from "$lib/driver";

    interface Props {
        /** The LIR of the buffer: the target's instructions in SSA form. */
        lir: Lir;

        /**
         * Called with the range a line stands for while a pointer is on it, and with `null` once
         * the pointer leaves it: the editor marks that part of the buffer.
         */
        onHover: (range: [number, number] | null) => void;

        /** Called when a line is picked, which is a request to be shown where it is written. */
        onPick: (range: [number, number] | null) => void;
    }

    let { lir, onHover, onPick }: Props = $props();

    /**
     * The frames a person folded, by the body and the line they stand at: a reading of a deep
     * structure is a person's choice, and the choice outlives a buffer read again.
     *
     * A body is named by its place in the tree --- `2`, and `2.0` for the first lambda of the
     * third body --- because a lambda has no name of its own and two of them may be written in
     * bodies far apart.
     */
    let folded = $state<string[]>([]);

    /**
     * What a line tells the page about the pointer: the part of the source it stands for.
     *
     * A line of the LIR is an instruction of the target, and the driver says what it was lowered
     * from: pointing at the line marks that place in the buffer, and picking it is a request to
     * be shown it.
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

    /** What one value of a block is: `v1(local 2): (ref i31)`, the local included. */
    function value_text(value: LirValue): string {
        const local = value.local === null ? "" : `(local ${value.local})`;

        return `${value.label}${local}: ${value.ty}`;
    }

    /** What one local holds: `1: (ref i31) = v1`, `3: i32 (pc)`. */
    function local(local: LirLocal): string {
        const values =
            local.values.length > 0 ? ` = ${local.values.join(", ")}` : "";
        const kind = local.kind === "value" ? "" : ` (${local.kind})`;

        return `${local.index}: ${local.ty}${kind}${values}`;
    }

    /**
     * The blocks a block is entered from and the ones it goes to, as one line reads them.
     */
    function edges(block: LirBlock): string {
        const into = block.predecessors.map(label);
        const out = block.successors.map(label);

        return [
            into.length > 0 ? `← ${into.join(", ")}` : "",
            out.length > 0 ? `→ ${out.join(", ")}` : "",
        ]
            .filter((it) => it !== "")
            .join("  ");
    }

    /** Whether a node of the structure holds other nodes and can be folded. */
    function holds(kind: string): boolean {
        return (
            kind === "block" ||
            kind === "loop" ||
            kind === "if" ||
            kind === "else"
        );
    }

    /** When a frame of the structure was folded: the body and the line it stands at. */
    function key(body: string, at: number): string {
        return `${body}:${at}`;
    }

    /** Folds a frame, or unfolds one a person opened again. */
    function toggle(body: string, at: number) {
        const id = key(body, at);

        folded = folded.includes(id)
            ? folded.filter((it) => it !== id)
            : [...folded, id];
    }

    /**
     * Which lines of a structure a person folded away, by the line they stand at.
     *
     * A frame holds every line deeper than it, up to the next line at its own depth; an `if`
     * holds its else arm as well, because the arm stands at the depth of the `if` itself.
     */
    function hidden_lines(lines: LirStructureLine[], body: string): boolean[] {
        const hidden = lines.map(() => false);

        for (const id of folded) {
            if (!id.startsWith(`${body}:`)) continue;

            const at = Number(id.slice(id.indexOf(":") + 1));
            if (at >= lines.length) continue;

            const depth = lines[at].depth;
            let end = at + 1;

            while (end < lines.length && lines[end].depth > depth) end++;

            if (
                lines[at].kind === "if" &&
                end < lines.length &&
                lines[end].kind === "else"
            ) {
                end++;
                while (end < lines.length && lines[end].depth > depth) end++;
            }

            for (let line = at + 1; line < end; line++) hidden[line] = true;
        }

        return hidden;
    }
</script>

{#snippet body(view: LirBody, path: string)}
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
            <span class="entry">ret {view.ret}</span>
        </button>

        {#if view.params.length > 0}
            <p class="params" data-line="params">
                <span class="tag">params</span>
                {#each view.params as value_, at (at)}
                    <button
                        class="value"
                        data-line="value"
                        title={value_text(value_)}
                        {...pointing(value_.range)}
                        onclick={() => onPick(value_.range)}
                    >
                        {value_.label}<span class="ty">: {value_.ty}</span>
                    </button>
                {/each}
            </p>
        {/if}

        {#if view.locals.length > 0}
            <p class="params" data-line="locals">
                <span class="tag">locals</span>
                {#each view.locals as held, at (at)}
                    <span
                        class="value local"
                        data-line="local"
                        title={local(held)}>{local(held)}</span
                    >
                {/each}
            </p>
        {/if}

        {#if view.structure !== null}
            {@const hidden = hidden_lines(view.structure.lines, path)}
            <div class="structure" data-structure>
                <p class="params">
                    <span class="tag">structure</span>
                </p>

                {#each view.structure.lines as node, at (at)}
                    {#if !hidden[at]}
                        <div
                            class="frame"
                            style="padding-left: {node.depth * 0.75}rem"
                        >
                            {#if holds(node.kind)}
                                <button
                                    class="fold"
                                    data-fold={node.kind}
                                    title={folded.includes(key(path, at))
                                        ? "Unfold"
                                        : "Fold"}
                                    onclick={() => toggle(path, at)}
                                    >{folded.includes(key(path, at))
                                        ? "▸"
                                        : "▾"}</button
                                >
                            {:else}
                                <span class="fold"></span>
                            {/if}

                            <button
                                class="line"
                                data-line="structure"
                                data-kind={node.kind}
                                data-depth={node.depth}
                                title={node.text}
                                {...pointing(node.range)}
                                onclick={() => onPick(node.range)}
                            >
                                <span class="saying">{node.text}</span>
                            </button>
                        </div>
                    {/if}
                {/each}
            </div>
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
                        >{block.label}{#if block.params.length > 0}({#each block.params as value_, i (i)}<button
                                    class="value"
                                    data-line="value"
                                    title={value_text(value_)}
                                    {...pointing(value_.range)}
                                    onclick={() => onPick(value_.range)}
                                    >{value_text(value_)}</button
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

                {#each block.insts as inst, i (i)}
                    <button
                        class="line"
                        data-line="inst"
                        data-kind={inst.kind}
                        title={inst.text}
                        {...pointing(inst.range)}
                        onclick={() => onPick(inst.range)}
                    >
                        <span class="saying">{inst.text}</span>
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
                {#each view.lambdas as child, i (i)}
                    {@render body(child, `${path}.${i}`)}
                {/each}
            </div>
        {/if}
    </section>
{/snippet}

<div class="bodies" data-lir>
    {#each lir.bodies as view, at (at)}
        {@render body(view, `${at}`)}
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

    /* A lambda is a lifted function written in another one: its section stands inside its
       writer, and the line at its left is what says so. */
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
        flex-wrap: wrap;
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

    /* The structure is the tree of frames encoding writes: a leaf, and the frames around it.
       Its lines are indented by depth, and a frame folds what it holds. */
    .structure {
        padding: 0.1rem 0 0.25rem;
    }

    .frame {
        display: flex;
        align-items: baseline;
        min-height: 1.1rem;
    }

    .fold {
        flex: none;
        width: 1rem;
        color: var(--muted);
        font-family: var(--mono);
        font-size: 10px;
        text-align: left;
    }

    .fold:hover {
        color: var(--text);
    }

    .structure .line {
        padding-left: 0;
    }

    .structure .saying {
        color: var(--muted);
    }

    .line[data-kind="if"] .saying,
    .line[data-kind="br"] .saying {
        color: var(--type);
    }

    .line[data-kind="leaf"] .saying {
        color: var(--accent);
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

    /* A value is what the compiler calls a parameter or a definition: a label, and the type of
       the machine value it is. */
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

    /* A local is where a value lives; the table is a note of the allocation and not source. */
    .local {
        color: var(--muted);
        cursor: default;
    }

    .local:hover {
        background: none;
    }

    /* An instruction reads as one line of the block it belongs to; the terminator is the line
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
