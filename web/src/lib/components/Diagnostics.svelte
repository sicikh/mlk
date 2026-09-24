<script lang="ts">
    import { codeOf, mainLabel } from "$lib/diagnostics";
    import type { Diagnostic, Label } from "$lib/driver";
    import { byteLength, utf16At } from "$lib/offsets";

    interface Props {
        /** What the parser reported, in the order it reported it. */
        diagnostics: Diagnostic[];

        /** The text they are about: a diagnostic points at a place in it, and a snippet is cut out of it. */
        text: string;
    }

    let { diagnostics, text }: Props = $props();

    /** The lines of the buffer, which is what the line a label names points at. */
    const lines = $derived(text.split("\n"));

    /** What one line of a snippet holds: what comes before the mark, the mark, and what follows. */
    function cut(label: Label) {
        const line = lines[label.line] ?? "";
        const from = utf16At(line, label.column);

        // A label may point past the end of its line, or span several: a snippet shows the first.
        const to = utf16At(
            line,
            Math.min(
                byteLength(line),
                label.column + (label.end - label.start),
            ),
        );

        return {
            before: line.slice(0, from),
            mark: line.slice(from, to),
            after: line.slice(to),
        };
    }

    /**
     * What to put in front of a label's message so that it sits under the mark it is about.
     *
     * The marker is padded with what a reader sees, which is not what a string counts:
     * a tab stays a tab, everything else becomes a space.
     */
    const pad = (before: string) => before.replace(/[^\t]/g, " ");

    /** Whether a label points at a place rather than at a piece of text: the end of a file is one. */
    const point = (label: Label) => label.end === label.start;
</script>

{#if diagnostics.length === 0}
    <p class="empty">No diagnostics.</p>
{:else}
    <ul>
        {#each diagnostics as diagnostic (diagnostic.code + diagnostic.message)}
            <li class={diagnostic.level}>
                <p class="head">
                    <span class="code" title={diagnostic.category}
                        >{codeOf(diagnostic)}</span
                    >
                    <span class="message">{diagnostic.message}</span>
                </p>

                {#each diagnostic.labels as label}
                    {@const snippet = cut(label)}
                    <div class="snippet" class:primary={label.primary}>
                        <div class="line">
                            <span class="number">{label.line + 1}</span>
                            <span class="source"
                                >{snippet.before}<mark
                                    class:point={point(label)}
                                    >{snippet.mark}</mark
                                >{snippet.after}</span
                            >
                        </div>

                        {#if label.message !== "" || label.primary}
                            <div class="line">
                                <span class="number"></span>
                                <span class="caret"
                                    >{pad(snippet.before)}<span class="arrow"
                                        >{snippet.mark === ""
                                            ? "^"
                                            : "^".repeat(
                                                  snippet.mark.length,
                                              )}</span
                                    >{label.message !== ""
                                        ? ` ${label.message}`
                                        : ""}</span
                                >
                            </div>
                        {/if}
                    </div>
                {/each}

                {#each diagnostic.notes as note}
                    <p class="note">{note}</p>
                {/each}
            </li>
        {/each}
    </ul>
{/if}

<style>
    .empty {
        margin: 0;
        padding: 0.75rem;
        color: var(--muted);
    }

    ul {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    li {
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
        padding: 0.6rem 0.75rem;
        border-bottom: 1px solid var(--border);
    }

    .head {
        margin: 0;
    }

    .code {
        margin-right: 0.4rem;
        font-family: var(--mono);
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.02em;
    }

    li.error .code {
        color: var(--error);
    }

    li.warning .code {
        color: var(--warning);
    }

    li.note .code {
        color: var(--accent);
    }

    .message {
        color: var(--text);
    }

    /* The place the diagnostic is about, as the compiler sees the text: a line, and a caret under it. */
    .snippet {
        font-family: var(--mono);
        font-size: 12px;
        line-height: 1.45;
    }

    .line {
        display: flex;
        gap: 0.5rem;
        white-space: pre;
    }

    .number {
        flex: none;
        width: 2rem;
        color: var(--muted);
        text-align: right;
    }

    .source {
        color: var(--text);
    }

    .caret {
        color: var(--muted);
    }

    .arrow {
        font-weight: 700;
    }

    .snippet.primary .arrow {
        color: var(--error);
    }

    li.warning .snippet.primary .arrow {
        color: var(--warning);
    }

    li.note .snippet.primary .arrow {
        color: var(--accent);
    }

    mark {
        background: #3a2226;
        border-radius: 2px;
        color: inherit;
    }

    li.warning mark {
        background: #3a3122;
    }

    li.note mark {
        background: #22303a;
    }

    /* A label at the end of a file points at nothing, so it is drawn as a caret. */
    mark.point::before {
        content: "^";
    }

    mark.point {
        background: none;
    }

    .note {
        margin: 0;
        color: var(--muted);
        font-size: 12px;
    }
</style>
