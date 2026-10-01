<script lang="ts">
    import type { Cost } from "$lib/driver";

    interface Props {
        /** What the read of the buffer cost the driver: the counters, and the time it took. */
        cost: Cost;
    }

    let { cost }: Props = $props();

    /**
     * The rows the read touched: a pass and unit that did nothing is not a row.
     *
     * A pass may be several rows --- the check of every body is one --- and what a person reads
     * is which of them was read again.
     */
    const touched = $derived(
        cost.rows.filter(
            (it) => it.hits + it.misses + it.stales + it.dropped > 0,
        ),
    );
</script>

<section class="part">
    <h2>Last read</h2>

    <p class="took" data-cost="took" data-took={cost.took}>
        {cost.took.toFixed(1)} ms
        {#if touched.length > 0}
            <span>· {touched.length} values</span>
        {/if}
    </p>
</section>

{#if touched.length === 0}
    <p class="none">
        Nothing was read: every value the read asked for was already there.
    </p>
{:else}
    <table>
        <thead>
            <tr>
                <th>pass · unit</th>
                <th title="The value was there: the pass did not run.">hit</th>
                <th title="The pass ran: the driver held no value.">miss</th>
                <th
                    title="The pass ran: what was held was built from something else."
                    >stale</th
                >
                <th
                    title="The pass ran and what it read came out the same, so nothing that reads it moved."
                    >kept</th
                >
                <th title="The value went, with the input it was built from."
                    >drop</th
                >
                <th title="What the pass spent running for this unit.">ms</th>
            </tr>
        </thead>

        <tbody>
            {#each touched as it, index (index)}
                <tr data-stats={it.pass} data-unit={it.unit}>
                    <th title="{it.pass} {it.unit}">
                        {#if index === 0 || touched[index - 1].pass !== it.pass}<span
                                class="pass">{it.pass}</span
                            >{/if}<span class="unit">{it.unit}</span>
                    </th>
                    <td data-count="hits" class:zero={it.hits === 0}
                        >{it.hits}</td
                    >
                    <td data-count="misses" class:zero={it.misses === 0}
                        >{it.misses}</td
                    >
                    <td data-count="stales" class:zero={it.stales === 0}
                        >{it.stales}</td
                    >
                    <td
                        data-count="kept"
                        class:zero={it.kept === 0}
                        class:kept={it.kept > 0}>{it.kept}</td
                    >
                    <td data-count="dropped" class:zero={it.dropped === 0}
                        >{it.dropped}</td
                    >
                    <td data-count="took" data-took={it.took}
                        >{it.took.toFixed(1)}</td
                    >
                </tr>
            {/each}
        </tbody>
    </table>
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

    .took {
        margin: 0;
        color: var(--text);
        font-family: var(--mono);
        font-size: 13px;
    }

    .took span {
        color: var(--muted);
        font-size: 11px;
    }

    table {
        width: 100%;
        padding: 0 0.75rem 0.5rem;
        font-family: var(--mono);
        font-size: 12px;
        border-collapse: collapse;
    }

    thead th {
        padding: 0.1rem 0;
        color: var(--muted);
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-align: right;
        text-transform: uppercase;
    }

    tbody th {
        color: var(--text);
        font-weight: 400;
        text-align: left;
    }

    thead th:first-child,
    tbody th {
        text-align: left;
    }

    /* The row is one line: the pass, once for its group of rows, and the unit it is about. */
    tbody th {
        max-width: 11rem;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }

    .pass {
        margin-right: 0.3rem;
        color: var(--muted);
        font-size: 10px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
    }

    td {
        color: var(--text);
        text-align: right;
    }

    /* What did not happen is not what a person is reading the table for: it is kept quiet. */
    td.zero {
        color: var(--muted);
        opacity: 0.6;
    }

    /* What a read kept is the value nothing downstream paid for: it is the point of the table. */
    td.kept {
        color: var(--ok);
    }

    .none {
        margin: 0;
        padding: 0 0.75rem 0.5rem;
        color: var(--muted);
    }
</style>
