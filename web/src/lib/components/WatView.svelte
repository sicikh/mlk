<script lang="ts">
    import { defaultKeymap } from "@codemirror/commands";
    import {
        codeFolding,
        foldAll,
        foldGutter,
        foldKeymap,
        unfoldAll,
    } from "@codemirror/language";
    import { EditorState } from "@codemirror/state";
    import { EditorView, keymap, lineNumbers } from "@codemirror/view";

    import type { Wat } from "#lib/driver.js";
    import { watLanguageSupport } from "#lib/wat.js";

    interface Props {
        /** The module the back end assembled, as text, with what compiling it reported. */
        wat: Wat;
    }

    let { wat }: Props = $props();

    /** The view, for the commands a person asks for by button rather than by key. */
    let view: EditorView | undefined;

    /** The number of lines of the module, which is what the head of the tab says. */
    const lines = $derived(wat.text.split("\n").length);

    /**
     * Gives the host a view of the module, and shows what the driver read again in it.
     *
     * A module is read, never written: the text comes from the compiler, and what a person does
     * with it is fold it, scroll it, and copy out of it. The view is built once and lives as
     * long as the tab is in front; a module read again --- the buffer was edited --- replaces
     * what it shows, and the folds a person made stay over what of them still stands.
     */
    function watView(node: HTMLDivElement, text: string) {
        const created = new EditorView({
            parent: node,
            state: EditorState.create({
                doc: text,
                extensions: [
                    EditorState.readOnly.of(true),
                    EditorView.editable.of(false),
                    lineNumbers(),
                    codeFolding(),
                    foldGutter({ openText: "▾", closedText: "▸" }),
                    watLanguageSupport(),
                    keymap.of([...defaultKeymap, ...foldKeymap]),
                    theme,
                ],
            }),
        });

        view = created;

        return {
            update(next: string) {
                if (created.state.doc.toString() === next) return;

                created.dispatch({
                    changes: {
                        from: 0,
                        to: created.state.doc.length,
                        insert: next,
                    },
                });
            },

            destroy() {
                view = undefined;
                created.destroy();
            },
        };
    }

    /**
     * The look of the module: the colours and the font of the editor, in a view of its own
     * that scrolls.
     *
     * The view fills the panel rather than growing with the module, so that the line numbers
     * and the fold markers stay where a person reads them while the code moves under them.
     */
    const theme = EditorView.theme(
        {
            "&": {
                height: "100%",
                color: "var(--text)",
                backgroundColor: "transparent",
            },
            ".cm-scroller": {
                fontFamily: "var(--mono)",
                fontSize: "var(--code-size)",
                lineHeight: "1.4",
            },
            ".cm-content": { padding: "0 0 0.35rem" },
            ".cm-gutters": {
                backgroundColor: "transparent",
                border: "none",
                color: "var(--muted)",
            },
            ".cm-lineNumbers .cm-gutterElement": {
                padding: "0 0.5rem 0 0.25rem",
            },
            ".cm-foldGutter .cm-gutterElement": {
                padding: "0 0.2rem",
                cursor: "pointer",
            },
            ".cm-foldGutter .cm-gutterElement:hover": { color: "var(--text)" },
            ".cm-foldPlaceholder": {
                backgroundColor: "var(--raised)",
                border: "none",
                borderRadius: "2px",
                color: "var(--muted)",
                padding: "0 0.3rem",
            },
            ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection":
                {
                    backgroundColor: "var(--raised)",
                },
            "&.cm-focused": { outline: "none" },
            ".cm-activeLine": { backgroundColor: "transparent" },
            ".cm-activeLineGutter": {
                backgroundColor: "transparent",
                color: "var(--text)",
            },
        },
        { dark: true },
    );
</script>

<div class="wat" data-wat>
    <div class="head">
        <span class="lines">{lines} lines</span>
        <button
            data-wat-fold-all
            onclick={() => view && foldAll(view)}
            title="Fold every form of the module">Fold all</button
        >
        <button
            data-wat-unfold-all
            onclick={() => view && unfoldAll(view)}
            title="Unfold every form of the module">Unfold all</button
        >
    </div>

    <!--
        The custom sections are not in the text: they are binary tables, and what a person reads
        of them is their names and sizes. The debug tables are what the debug level decides, so
        they are marked apart from the name section, which every module carries.
    -->
    {#if wat.sections.length > 0}
        <div class="sections" data-wat-sections>
            {#each wat.sections as section (section.name)}
                <span
                    class="section"
                    class:debug={section.name.startsWith(".debug")}
                    data-section={section.name}
                >
                    {section.name} <span class="size">{section.size} B</span>
                </span>
            {/each}
        </div>
    {/if}

    {#each wat.diagnostics as it, index (index)}
        <p class="empty">
            {it.level}[{it.category}::{it.code}]: {it.message}
        </p>
    {/each}

    <div class="host" use:watView={wat.text}></div>
</div>

<style>
    .wat {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        height: 100%;
        min-height: 0;
    }

    .host {
        flex: 1;
        min-height: 0;
    }

    .head {
        display: flex;
        gap: 0.35rem;
        align-items: center;
        padding: 0 0.25rem;
    }

    .lines {
        margin-right: auto;
        color: var(--muted);
        font-family: var(--mono);
        font-size: 11px;
    }

    .head button {
        padding: 0.1rem 0.4rem;
        border: 1px solid var(--border);
        border-radius: var(--radius);
        color: var(--muted);
        font-size: 11px;
    }

    .head button:hover {
        border-color: var(--accent);
        color: var(--text);
    }

    .sections {
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem;
        padding: 0 0.25rem;
    }

    .section {
        padding: 0.05rem 0.35rem;
        border: 1px solid var(--border);
        border-radius: var(--radius);
        color: var(--muted);
        font-family: var(--mono);
        font-size: 10px;
    }

    .section.debug {
        border-color: var(--accent);
        color: var(--text);
    }

    .section .size {
        opacity: 0.7;
    }

    .empty {
        margin: 0;
        padding: 0 0.25rem;
        color: var(--muted);
    }
</style>
