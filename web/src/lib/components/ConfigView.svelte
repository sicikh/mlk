<script lang="ts">
    import type { DebugInfo, OptLevel } from "$lib/driver";

    interface Props {
        /** What debug information a module carries. */
        debug: DebugInfo;
        /** The preset the optional passes optimize under. */
        opt: OptLevel;
        /** What to do when another debug information is asked for. */
        onDebug: (debug: DebugInfo) => void;
        /** What to do when another preset is asked for. */
        onOpt: (opt: OptLevel) => void;
    }

    let { debug, opt, onDebug, onOpt }: Props = $props();
</script>

<!--
    The configuration of the compiler: what the pipeline is asked for, rather than what it made
    of a buffer. It belongs with the views of the compiler --- the tabs beside this one show
    what it produced --- and it grows with the pipeline: the passes will have options of their
    own, the optimization is the preset that picks a group of them at once, and the target of
    the back end will be configured here as well ([ADR-0025]).

    [adr-0025]: ../../../../docs/adr/0025-debug-information-formats.md
-->
<section class="part">
    <h2>Debug information</h2>

    <label>
        <select
            data-debug
            value={debug}
            onchange={(it) => onDebug(it.currentTarget.value as DebugInfo)}
        >
            <option value="none">none</option>
            <option value="source-map">source map</option>
            <option value="dwarf-lines">DWARF lines</option>
            <option value="dwarf-full">DWARF full</option>
        </select>
    </label>

    <p class="hint">
        The format of the debug information of a module: the source map of a browser,
        which carries the text of the sources itself; the tables of DWARF, which a
        native debugger reads; or nothing.
    </p>
</section>

<section class="part">
    <h2>Optimization</h2>

    <label>
        <select
            data-opt
            value={opt}
            onchange={(it) => onOpt(it.currentTarget.value as OptLevel)}
        >
            <option value="none">none</option>
            <option value="full">full</option>
        </select>
    </label>

    <p class="hint">
        The preset the optional passes run under; a pass of its own will have a switch
        beside this one.
    </p>
</section>

<style>
    .part {
        padding: 0.5rem 0.75rem;
        border-bottom: 1px solid var(--border);
    }

    h2 {
        margin: 0 0 0.35rem;
        color: var(--muted);
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
    }

    select {
        width: 100%;
        padding: 0.3rem 0.5rem;
        background: var(--raised);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        color: var(--text);
        font: inherit;
        font-size: 12px;
        cursor: pointer;
    }

    select:hover {
        border-color: #38414f;
    }

    .hint {
        margin: 0.35rem 0 0;
        color: var(--muted);
        font-size: 11px;
    }
</style>
