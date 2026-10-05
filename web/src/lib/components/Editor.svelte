<script lang="ts">
    import {
        defaultKeymap,
        history,
        historyKeymap,
        indentWithTab,
    } from "@codemirror/commands";
    import { bracketMatching, indentOnInput } from "@codemirror/language";
    import {
        lintGutter,
        lintKeymap,
        setDiagnostics,
        type Diagnostic as LintDiagnostic,
    } from "@codemirror/lint";
    import { EditorState, StateEffect, StateField } from "@codemirror/state";
    import {
        Decoration,
        type DecorationSet,
        EditorView,
        drawSelection,
        highlightActiveLine,
        highlightActiveLineGutter,
        keymap,
        lineNumbers,
    } from "@codemirror/view";
    import { untrack } from "svelte";

    import {
        codeOf,
        mainLabel,
        rangeOf,
        severityOf,
    } from "#lib/diagnostics.js";
    import type { Diagnostic } from "#lib/driver.js";
    import { mlkLanguageSupport } from "#lib/highlight.js";
    import { utf16At } from "#lib/offsets.js";

    interface Props {
        /** The buffers open in the editor, in the order of their tabs. */
        tabs: string[];

        /** The buffer whose tab is in front. */
        path: string;

        /** The text of a buffer, for a tab that has not been read before. */
        text: (path: string) => string;

        /** What the compiler reported about the buffer in front. */
        diagnostics?: Diagnostic[];

        /**
         * The part of the source a pointer is on in a tree of the inspector, counted the
         * way the compiler counts it: the editor marks that part of the buffer.
         */
        hovered?: [number, number] | null;

        /**
         * The part of the source what the pointer is on names, counted the same way: a path
         * of the HIR is marked as the code it was written as, and here as where the name it
         * resolved to comes from.
         */
        resolved?: [number, number] | null;

        /**
         * How many times a person has asked to be taken to a place rather than shown one.
         *
         * A row of a tree that holds nothing stands for a place in the code, and picking one
         * is a request to read it there: the editor scrolls the mark into view for every
         * such pick, while a pointer moving over a tree only marks what it passes.
         */
        reveals?: number;

        /**
         * Whether a person may write in a buffer.
         *
         * The standard library of the language is part of the compiler: the editor shows its
         * files, and takes no text from them.
         */
        writable: (path: string) => boolean;

        /** Called with the whole text of a buffer after every change. */
        onInput: (path: string, text: string) => void;

        /** Called when a person picks another tab. */
        onSelect: (path: string) => void;

        /** Called when a person closes a tab, which is not the same as dropping the buffer. */
        onClose: (path: string) => void;
    }

    let {
        tabs,
        path,
        text,
        diagnostics = [],
        hovered = null,
        resolved = null,
        reveals = 0,
        writable,
        onInput,
        onSelect,
        onClose,
    }: Props = $props();

    /**
     * The text and the undo history of every open tab.
     *
     * A tab keeps both while it is open, so that coming back to it is a step back
     * rather than a reload: the editor holds one view over as many states as there are tabs.
     */
    const states = new Map<string, EditorState>();

    /** The editor, which shows whatever state the tab in front holds. */
    let view = $state<EditorView | undefined>();

    /** The look of the editor: the same dark theme the rest of the page wears. */
    const theme = EditorView.theme(
        {
            "&": {
                height: "100%",
                color: "var(--text)",
                backgroundColor: "var(--bg)",
            },
            ".cm-scroller": {
                fontFamily: "var(--mono)",
                /* The size a phone reads code in is set where the theme is (see `app.css`). */
                fontSize: "var(--code-size)",
                lineHeight: "1.6",
            },
            ".cm-content": { padding: "0.5rem 0", caretColor: "var(--accent)" },
            ".cm-gutters": {
                backgroundColor: "var(--bg)",
                border: "none",
                color: "var(--muted)",
            },
            ".cm-lineNumbers .cm-gutterElement": {
                padding: "0 0.5rem 0 0.75rem",
            },
            ".cm-activeLine": { backgroundColor: "var(--raised)" },
            ".cm-activeLineGutter": {
                backgroundColor: "var(--raised)",
                color: "var(--text)",
            },
            ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
            ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection":
                {
                    backgroundColor: "#2d4f7c",
                },
            "&.cm-focused": { outline: "none" },
            ".cm-matchingBracket": {
                backgroundColor: "#2d4f7c",
                outline: "none",
            },
            ".cm-tooltip": { zIndex: "10" },

            /* The code a row of a tree in the inspector stands for, marked while the pointer
               is on the row. It is the colour the row itself is painted in when it is read. */
            ".cm-hovered": {
                backgroundColor: "var(--raised)",
                borderRadius: "2px",
            },

            /* And the code a row names: what a name of the HIR comes from, marked as the place
               the name is written at rather than as the name itself. */
            ".cm-named": {
                backgroundColor: "var(--raised)",
                borderBottom: "1px dashed var(--accent)",
            },

            /* And what the compiler has to say about the text, marked the way an editor
               marks mistakes. The colours of the code itself are the language's:
               see `#lib/highlight.js`. */
            ".cm-lintRange-error": {
                backgroundImage: "none",
                textDecoration: "underline wavy var(--error)",
            },
            ".cm-lintRange-warning": {
                backgroundImage: "none",
                textDecoration: "underline wavy var(--warning)",
            },
        },
        { dark: true },
    );

    /** Where a pointer is on in a tree of the inspector, as the editor counts it. */
    interface Hover {
        /** The code a row stands for. */
        at: { from: number; to: number } | null;

        /** The code the row names, if it names something. */
        names: { from: number; to: number } | null;
    }

    /** The place in the text a pointer is on in a tree of the inspector. */
    const setHover = StateEffect.define<Hover | null>();

    /** One mark, reused: the code a row of a tree stands for, painted in the editor. */
    const hoveredCode = Decoration.mark({ class: "cm-hovered" });

    /** And one for the code a row names: where the name comes from. */
    const namedCode = Decoration.mark({ class: "cm-named" });

    /** What the editor paints for a pointer: the code a row stands for, and what it names. */
    function painted(at: Hover | null): DecorationSet {
        if (!at) return Decoration.none;

        const marks = [];

        if (at.at) marks.push(hoveredCode.range(at.at.from, at.at.to));
        if (at.names) marks.push(namedCode.range(at.names.from, at.names.to));

        return Decoration.set(marks, true);
    }

    /**
     * What a tree of the inspector points at, marked in the text it stands for.
     *
     * The marks are kept as the text changes, so that typing under the pointer does not move
     * them to somewhere the row never pointed at; the next move of the pointer sets them again.
     */
    const hover = StateField.define<DecorationSet>({
        create: () => Decoration.none,
        update: (marked, change) => {
            for (const effect of change.effects) {
                if (!effect.is(setHover)) continue;

                return painted(effect.value);
            }

            return marked.map(change.changes);
        },
        provide: (field) => EditorView.decorations.from(field),
    });

    /**
     * A range of the source as the editor counts it, or nothing where the range has no width.
     *
     * The compiler counts the source in bytes, and the editor counts the way a string does.
     * A token the parser expected and did not find sits at a place without code, and a mark
     * of no width is not a mark.
     */
    function rangeIn(text: string, at: [number, number] | null) {
        if (!at) return null;

        const from = utf16At(text, at[0]);
        const to = Math.max(from, utf16At(text, at[1]));

        return from < to ? { from, to } : null;
    }

    /** The state of a buffer: what it holds, and how to get back to it. */
    function stateOf(it: string): EditorState {
        let state = states.get(it);

        if (!state) {
            state = EditorState.create({
                doc: text(it),
                extensions: extensions(it),
            });
            states.set(it, state);
        }

        return state;
    }

    /** What every tab has: the same furniture, and a way back to the buffer it belongs to. */
    function extensions(it: string) {
        return [
            // A file of the standard library is the compiler's: a read-only state refuses what
            // a person types, and a view that is not editable does not offer to take it at all.
            EditorState.readOnly.of(!writable(it)),
            EditorView.editable.of(writable(it)),
            lineNumbers(),
            highlightActiveLineGutter(),
            history(),
            drawSelection(),
            highlightActiveLine(),
            indentOnInput(),
            bracketMatching(),
            mlkLanguageSupport(),
            hover,
            lintGutter(),
            keymap.of([
                ...defaultKeymap,
                ...historyKeymap,
                ...lintKeymap,
                indentWithTab,
            ]),
            EditorView.updateListener.of((update) => {
                if (!update.docChanged) return;

                states.set(it, update.state);
                onInput(it, update.state.doc.toString());
            }),
            theme,
        ];
    }

    /** What a diagnostic of the compiler is, as the editor marks one. */
    function lintOf(diagnostic: Diagnostic, text: string): LintDiagnostic {
        const label = mainLabel(diagnostic);
        const range = label ? rangeOf(label, text) : { from: 0, to: 0 };
        const says = label && label.message !== "" ? ` — ${label.message}` : "";

        return {
            ...range,
            severity: severityOf(diagnostic.level),
            message: `${codeOf(diagnostic)} ${diagnostic.message}${says}`,
            source: diagnostic.category,
        };
    }

    /**
     * Shows what the compiler made of the buffer that is in front.
     *
     * The colours are the editor's own ([`#lib/highlight.js`]), so what arrives here is
     * what the compiler has to say about the text: the places it complained about.
     */
    function show() {
        if (!view) return;

        const text = view.state.doc.toString();

        // A mark of a diagnostic is made against the state the text is in, which is this one.
        view.dispatch(
            setDiagnostics(
                view.state,
                diagnostics.map((it) => lintOf(it, text)),
            ),
        );
    }

    /** Another parse arrives with every keystroke: the editor is marked up with it. */
    $effect(() => {
        // Both names are read for their reach: Svelte runs the effect when either changes.
        // oxlint-disable-next-line no-unused-expressions
        diagnostics;
        show();
    });

    /**
     * A pointer on a row of a tree marks the code the row stands for, and what it names.
     *
     * A tree of the inspector and the editor hold the same buffer, so a range of one is a
     * range of the other: the compiler counts the source in bytes, which is what a tree
     * hands over, while the editor counts the way a string does.
     */
    $effect(() => {
        if (!view) return;

        const text = view.state.doc.toString();

        view.dispatch({
            effects: setHover.of({
                at: rangeIn(text, hovered),
                names: rangeIn(text, resolved),
            }),
        });
    });

    /**
     * A pick of a row of a tree is a request to read the place that row stands for, and a place
     * a person asked for is brought to them: the editor scrolls it to the middle of the view,
     * which is also where a phone keyboard leaves the most room to read it.
     */
    $effect(() => {
        // The name is read for its reach: Svelte runs the effect when it changes.
        // oxlint-disable-next-line no-unused-expressions
        reveals;

        if (!view) return;

        // The mark is what a pick leaves behind, and reading it here does not make this effect
        // run on every move of a pointer over a tree.
        const text = view.state.doc.toString();
        const at = untrack(() => rangeIn(text, hovered));

        if (at)
            view.dispatch({
                effects: EditorView.scrollIntoView(at.from, { y: "center" }),
            });
    });

    /**
     * Gives the host an editor, and shows the state of the buffer in front in it.
     *
     * The editor is built once and lives as long as the page: switching tabs swaps
     * what it shows, and nothing is rebuilt.
     */
    function editor(host: HTMLDivElement, first: string) {
        view = new EditorView({ parent: host, state: stateOf(first) });

        // A phone would open its keyboard over half the screen for a person who only asked
        // to read the page: there, the editor is touched before it is written in.
        if (window.matchMedia("(pointer: fine)").matches) view.focus();

        return {
            update(next: string) {
                const state = stateOf(next);

                if (view && view.state !== state) view.setState(state);

                show();
            },

            destroy() {
                view?.destroy();
                view = undefined;
            },
        };
    }

    /** A closed tab is forgotten: what it held is in the buffer, which is still there. */
    $effect(() => {
        // Deleting a key while a map is walked is what the iterator is defined for.
        for (const it of states.keys()) {
            if (!tabs.includes(it)) states.delete(it);
        }
    });

    /** A buffer has a name, not a directory: only the last part of its path is shown. */
    const name = (path: string) => path.split("/").at(-1) ?? path;
</script>

<div class="editor" data-panel="editor">
    <nav class="tabs">
        {#each tabs as tab (tab)}
            <div class="tab" class:active={tab === path} data-open={tab}>
                <button class="pick" onclick={() => onSelect(tab)} title={tab}
                    >{name(tab)}</button
                >
                {#if !writable(tab)}
                    <!-- The compiler's files are read rather than written, and a tab says at
                         a glance which of the open buffers is one of them. -->
                    <span
                        class="locked"
                        title="The standard library is read-only">r/o</span
                    >
                {/if}
                <button
                    class="close"
                    onclick={() => onClose(tab)}
                    aria-label="Close {name(tab)}">×</button
                >
            </div>
        {/each}
    </nav>

    <div class="host" use:editor={path}></div>
</div>

<style>
    .editor {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
        /* A line of code is as wide as it is; the panel it is read in is the width it was
           given, and the editor scrolls what does not fit. */
        min-width: 0;
        background: var(--bg);
        border-left: 1px solid var(--border);
        border-right: 1px solid var(--border);
    }

    .tabs {
        display: flex;
        min-height: 0;
        overflow-x: auto;
        background: var(--surface);
        border-bottom: 1px solid var(--border);
    }

    .tab {
        display: flex;
        align-items: center;
        border-right: 1px solid var(--border);
    }

    .tab.active {
        background: var(--bg);
        box-shadow: inset 0 2px 0 var(--accent);
    }

    .pick {
        padding: 0.4rem 0.25rem 0.4rem 0.7rem;
        color: var(--muted);
        font-family: var(--mono);
        font-size: 12px;
        white-space: nowrap;
    }

    .tab.active .pick,
    .pick:hover {
        color: var(--text);
    }

    .close {
        padding: 0 0.5rem 0 0.25rem;
        color: var(--muted);
        font-size: 13px;
        line-height: 1;
    }

    .close:hover {
        color: var(--error);
    }

    /* What a tab of a file that is not a person's to write says for itself. */
    .locked {
        color: var(--muted);
        font-size: 10px;
        letter-spacing: 0.03em;
    }

    .host {
        flex: 1;
        min-height: 0;
    }

    /* A finger picks a tab the way a pointer does, but it needs more room to land in. */
    @media (max-width: 860px), (max-height: 520px) {
        /* The editor is the screen here and not a panel among others: it has no seam to show. */
        .editor {
            border: none;
        }

        /* The switcher above is underlined along its bottom, and a mark along the top of a tab
           sat right under it, reading as one thick line: the tab that is open is marked the
           way the switcher is, along its own bottom. */
        .tab.active {
            box-shadow: inset 0 -2px 0 var(--accent);
        }

        .pick {
            padding: 0.55rem 0.3rem 0.55rem 0.7rem;
        }

        .close {
            padding: 0 0.6rem;
            font-size: 15px;
        }
    }
</style>
