<script lang="ts">
    interface Props {
        /** The buffer being edited. */
        path: string;

        /** Its text, as the driver holds it. */
        text: string;

        /** Called with the whole text after every keystroke. */
        onInput: (text: string) => void;
    }

    let { path, text, onInput }: Props = $props();

    /** A buffer has a name, not a directory: there is no file system under the editor. */
    const name = (path: string) => path.replace(/^\//, "");
</script>

<div class="editor" data-panel="editor">
    <header>
        <span class="tab">{name(path)}</span>
    </header>

    <!--
		The editor proper: a text area today, and the one place a real editor
		— one that marks ranges, folds code and highlights tokens — will stand instead.
	-->
    <textarea
        value={text}
        oninput={(event) => onInput(event.currentTarget.value)}
        spellcheck="false"
        autocapitalize="off"></textarea>
</div>

<style>
    .editor {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
        background: var(--bg);
        border-left: 1px solid var(--border);
        border-right: 1px solid var(--border);
    }

    header {
        display: flex;
        align-items: center;
        padding: 0.4rem 0.75rem;
        background: var(--surface);
        border-bottom: 1px solid var(--border);
    }

    .tab {
        color: var(--text);
        font-family: var(--mono);
        font-size: 12px;
    }

    textarea {
        flex: 1;
        margin: 0;
        padding: 0.75rem;
        background: none;
        border: 0;
        color: inherit;
        font-family: var(--mono);
        font-size: 13px;
        line-height: 1.6;
        white-space: pre;
        resize: none;
        outline: none;
        tab-size: 4;
    }
</style>
