<script lang="ts">
    import FileTree, { foldersOf } from "./FileTree.svelte";

    interface Props {
        /** The paths of the buffers the editor holds, in the order they were made. */
        files: string[];

        /** The buffer the editor is editing. */
        active: string;

        /** Called when a person picks another buffer. */
        onSelect: (path: string) => void;

        /** Called when a person asks for another buffer. */
        onAdd: () => void;
    }

    let { files, active, onSelect, onAdd }: Props = $props();

    /** The directories the paths make, which are the whole of the file system there is. */
    const tree = $derived(foldersOf(files));
</script>

<div class="panel" data-panel="files">
    <header>
        <h2>Files</h2>
        <button
            class="add"
            onclick={onAdd}
            title="A new buffer, beside this one">+</button
        >
    </header>

    <div class="tree">
        <FileTree folder={tree} {active} {onSelect} />
    </div>
</div>

<style>
    .panel {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
        background: var(--surface);
    }

    header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.4rem 0.5rem 0.4rem 0.75rem;
        border-bottom: 1px solid var(--border);
    }

    h2 {
        margin: 0;
        color: var(--muted);
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
    }

    .add {
        padding: 0 0.35rem;
        color: var(--muted);
        font-size: 14px;
        line-height: 1;
    }

    .add:hover {
        color: var(--text);
    }

    .tree {
        flex: 1;
        padding: 0.25rem 0;
        overflow: auto;
    }
</style>
