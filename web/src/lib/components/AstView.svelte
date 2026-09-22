<script lang="ts" module>
    /** What a value of the typed tree turns out to be, as far as a host can tell. */
    type Shape =
        | "node"
        | "list"
        | "token"
        | "missing"
        | "optional"
        | "value"
        | "unknown";

    /** A token of the syntax tree, as the tree itself serializes it ([ADR-0002]). */
    interface Token {
        kind: string;
        text_range: [number, number];
        text?: string;
    }

    /** A typed node or a list of them, which says what it is and holds the rest. */
    interface Named {
        kind: string;
    }

    /** One line of what a folded value holds: a field of a node, or an element of a list. */
    interface Row {
        /** What makes the row unique among its siblings. */
        key: string;

        /** What the value is called here, when it has a name of its own. */
        name?: string;

        /** The value itself. */
        value: unknown;
    }

    const isObject = (value: unknown): value is Record<string, unknown> =>
        typeof value === "object" && value !== null && !Array.isArray(value);

    /**
     * What a value of the typed tree is.
     *
     * A node says what it is and holds its fields under `fields`; a list says what it is
     * and holds its elements under `items`; a token is the one the syntax tree serializes:
     * its kind, its range and its text. A field that has to be there but is not is an `Err`,
     * and a field that may be missing is `null` ([ADR-0002]).
     *
     * [ADR-0002]: https://github.com/sicikh/mlk/blob/main/docs/adr/0002-lossless-syntax-tree.md
     */
    const shapeOf = (value: unknown): Shape => {
        if (value === null) return "optional";
        if (!isObject(value)) return "value";
        if ("Err" in value) return "missing";
        if (typeof value.text === "string" && Array.isArray(value.text_range))
            return "token";
        if (typeof value.kind === "string")
            return isObject(value.fields) ? "node" : "list";

        return "unknown";
    };

    /** What a folded value opens into, in the order the compiler holds it. */
    const rowsOf = (value: unknown, shape: Shape): Row[] => {
        if (shape === "node") {
            const fields =
                isObject(value) && isObject(value.fields) ? value.fields : {};

            return Object.entries(fields).map(([name, field]) => ({
                key: name,
                name,
                value: field,
            }));
        }

        const items = Array.isArray(value)
            ? value
            : isObject(value) && Array.isArray(value.items)
              ? value.items
              : [];

        // An element of a list names itself: an index in front of it would only be noise.
        return items.map((item, index) => ({ key: `${index}`, value: item }));
    };
</script>

<script lang="ts">
    import AstView from "./AstView.svelte";

    interface Props {
        /** A value of the typed tree, however deep it goes. */
        value: unknown;

        /** What the value is called where it sits: the field it is, or nothing under a list. */
        name?: string;
    }

    let { value, name }: Props = $props();

    /** Whether the value shows what it holds: a person folds what they are not reading. */
    let open = $state(true);

    /** A field that is there reads as it is; one that sits under `Ok` is only wrapped ([ADR-0002]). */
    const unwrapped = $derived(
        isObject(value) && "Ok" in value && Object.keys(value).length === 1
            ? value.Ok
            : value,
    );

    const shape = $derived(shapeOf(unwrapped));
    const rows = $derived(rowsOf(unwrapped, shape));

    const token = $derived(shape === "token" ? (unwrapped as Token) : null);
    const named = $derived(
        shape === "node" || shape === "list" ? (unwrapped as Named) : null,
    );

    const opening = $derived(shape === "list" ? "[" : "{");
    const closing = $derived(shape === "list" ? "]" : "}");

    const text = $derived(
        typeof unwrapped === "string"
            ? JSON.stringify(unwrapped)
            : String(unwrapped),
    );
</script>

{#snippet Field({ name }: { name?: string })}
    {#if name !== undefined}
        <span class="field">{name}<span class="punct">:</span></span>
    {/if}
{/snippet}

{#if token}
    <div class="row">
        <span class="spacer"></span>
        {@render Field({ name })}
        <span class="kind">{token.kind}</span>
        <span class="range">@{token.text_range[0]}..{token.text_range[1]}</span>
        <span class="text">{JSON.stringify(token.text ?? "")}</span>
    </div>
{:else if shape === "missing" || shape === "optional"}
    <div class="row">
        <span class="spacer"></span>
        {@render Field({ name })}
        <span class="missing"
            >{shape === "optional" ? "missing (optional)" : "missing"}</span
        >
    </div>
{:else if named && rows.length > 0}
    <button class="row" onclick={() => (open = !open)} aria-expanded={open}>
        <span class="caret">{open ? "▾" : "▸"}</span>
        {@render Field({ name })}
        <span class="kind">{named.kind}</span>
        <span class="bracket">{open ? opening : `${opening} … ${closing}`}</span
        >
    </button>

    {#if open}
        <div class="children">
            {#each rows as row (row.key)}
                <AstView value={row.value} name={row.name} />
            {/each}
        </div>
    {/if}
{:else if named}
    <div class="row">
        <span class="spacer"></span>
        {@render Field({ name })}
        <span class="kind">{named.kind}</span>
        <span class="bracket">{opening}{closing}</span>
    </div>
{:else}
    <div class="row">
        <span class="spacer"></span>
        {@render Field({ name })}
        <span class="value">{text}</span>
    </div>
{/if}

<style>
    .row {
        display: flex;
        gap: 0.35rem;
        align-items: baseline;
        width: 100%;
        font-family: var(--mono);
        font-size: 12px;
        text-align: left;
        white-space: nowrap;
    }

    .row:hover {
        background: var(--raised);
    }

    .children {
        margin-left: 0.55rem;
        padding-left: 0.75rem;
        border-left: 1px solid var(--border);
    }

    .caret,
    .spacer {
        width: 0.75rem;
        color: var(--muted);
    }

    .field {
        color: #b4a1e3;
    }

    .punct,
    .bracket {
        color: var(--muted);
    }

    .kind {
        color: var(--accent);
    }

    .range {
        color: var(--muted);
        font-size: 11px;
    }

    .text {
        color: var(--ok);
    }

    .value {
        color: var(--text);
    }

    .missing {
        color: var(--warning);
    }
</style>
