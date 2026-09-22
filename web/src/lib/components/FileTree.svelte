<script lang="ts" module>
    /** A directory of the editor's buffers, worked out from the paths of the buffers. */
    export interface Folder {
        /** The name of the directory, empty for the one everything sits in. */
        name: string;

        /** The directories under it, in the order they appeared. */
        folders: Folder[];

        /** The paths of the buffers directly under it. */
        files: string[];
    }

    interface Node {
        folders: Map<string, Node>;
        files: string[];
    }

    /**
     * The directories a list of paths makes.
     *
     * A browser has no directories to read, so the paths of the buffers are the whole truth:
     * `/lib/vector.mlk` is a file called `vector.mlk` in a directory called `lib`,
     * and a directory exists for as long as a buffer does ([ADR-0007]).
     *
     * [ADR-0007]: https://github.com/sicikh/mlk/blob/main/docs/adr/0007-vfs-file-state.md
     */
    export function foldersOf(paths: string[]): Folder {
        const root: Node = { folders: new Map(), files: [] };

        for (const path of paths) {
            const parts = path.split("/").filter(Boolean);
            const file = parts.pop();

            if (file === undefined) continue;

            let node = root;

            for (const part of parts) {
                let next = node.folders.get(part);

                if (!next)
                    node.folders.set(
                        part,
                        (next = { folders: new Map(), files: [] }),
                    );

                node = next;
            }

            node.files.push(path);
        }

        return of(root, "");
    }

    /** Turns what the paths built into the shape a component walks. */
    function of(node: Node, path: string): Folder {
        return {
            name: path.split("/").filter(Boolean).at(-1) ?? "",
            folders: [...node.folders].map(([name, child]) =>
                of(child, `${path}/${name}`),
            ),
            files: node.files,
        };
    }
</script>

<script lang="ts">
    import FileTree from "./FileTree.svelte";

    interface Props {
        /** The directory to show, with everything under it. */
        folder: Folder;

        /** The buffer the editor is editing. */
        active: string;

        /** The buffer a person is being asked about, if any. */
        confirming: string | null;

        /** Called when a person picks another buffer. */
        onSelect: (path: string) => void;

        /** Called with the buffer to ask about, or with `null` when the asking is over. */
        onConfirm: (path: string | null) => void;

        /** Called with the buffer a person said yes to. */
        onRemove: (path: string) => void;
    }

    let { folder, active, confirming, onSelect, onConfirm, onRemove }: Props =
        $props();

    /** Whether the directory shows what it holds: a person folds what they are not reading. */
    let open = $state(true);

    /** A buffer has a name, not a directory: only the last part of its path is shown. */
    const name = (path: string) => path.split("/").at(-1) ?? path;
</script>

{#snippet held()}
    {#each folder.folders as child (child.name)}
        <FileTree
            folder={child}
            {active}
            {confirming}
            {onSelect}
            {onConfirm}
            {onRemove}
        />
    {/each}

    {#each folder.files as file (file)}
        {#if confirming === file}
            <div class="file asking" data-file={file}>
                <span class="question">Delete {name(file)}?</span>
                <span class="choice">
                    <button class="yes" onclick={() => onRemove(file)}
                        >Delete</button
                    >
                    <button class="no" onclick={() => onConfirm(null)}
                        >Keep</button
                    >
                </span>
            </div>
        {:else}
            <div class="file" class:active={file === active} data-file={file}>
                <button class="pick" onclick={() => onSelect(file)}
                    >{name(file)}</button
                >
                <button
                    class="drop"
                    onclick={() => onConfirm(file)}
                    title="Delete this buffer"
                    aria-label="Delete {name(file)}">×</button
                >
            </div>
        {/if}
    {/each}
{/snippet}

{#if folder.name === ""}
    {@render held()}
{:else}
    <button
        class="folder"
        onclick={() => (open = !open)}
        aria-expanded={open}
        data-folder={folder.name}
    >
        <span class="caret">{open ? "▾" : "▸"}</span>
        <span class="name">{folder.name}</span>
    </button>

    {#if open}
        <div class="children">
            {@render held()}
        </div>
    {/if}
{/if}

<style>
    .folder {
        display: flex;
        gap: 0.35rem;
        align-items: baseline;
        width: 100%;
        padding: 0.15rem 0.75rem;
        color: var(--muted);
        font-family: var(--mono);
        font-size: 12px;
        text-align: left;
    }

    .folder:hover {
        background: var(--raised);
        color: var(--text);
    }

    .file {
        display: flex;
        align-items: center;
    }

    .file:hover,
    .file.active {
        background: var(--raised);
    }

    .file.active {
        box-shadow: inset 2px 0 0 var(--accent);
    }

    .pick {
        flex: 1;
        min-width: 0;
        padding: 0.15rem 0.35rem 0.15rem 1.25rem;
        overflow: hidden;
        color: var(--muted);
        font-family: var(--mono);
        font-size: 12px;
        text-align: left;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .file:hover .pick,
    .file.active .pick {
        color: var(--text);
    }

    /* Dropping a buffer is rare and wants a second thought: it is out of the way until asked for. */
    .drop {
        padding: 0 0.4rem;
        color: var(--muted);
        font-size: 13px;
        line-height: 1;
        opacity: 0;
    }

    .file:hover .drop,
    .file:focus-within .drop,
    .file.active .drop {
        opacity: 1;
    }

    .drop:hover {
        color: var(--error);
    }

    .asking {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.2rem;
        padding: 0.25rem 0.35rem 0.25rem 1.25rem;
        font-family: var(--mono);
        font-size: 12px;
    }

    .choice {
        display: flex;
        gap: 0.35rem;
    }

    .question {
        max-width: 100%;
        overflow: hidden;
        color: var(--text);
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .yes,
    .no {
        padding: 0.05rem 0.35rem;
        border: 1px solid var(--border);
        border-radius: var(--radius);
        font-size: 11px;
    }

    .yes {
        color: var(--error);
    }

    .yes:hover {
        border-color: var(--error);
    }

    .no {
        color: var(--muted);
    }

    .no:hover {
        color: var(--text);
    }

    .children {
        margin-left: 0.55rem;
        border-left: 1px solid var(--border);
    }

    .caret {
        width: 0.75rem;
    }
</style>
