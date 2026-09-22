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
     * `/lib/vector.mlk` is a file called `vector.mlk` in a directory called `lib` ([ADR-0007]).
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

        /** Called when a person picks another buffer. */
        onSelect: (path: string) => void;
    }

    let { folder, active, onSelect }: Props = $props();

    /** Whether the directory shows what it holds: a person folds what they are not reading. */
    let open = $state(true);

    /** A buffer has a name, not a directory: only the last part of its path is shown. */
    const name = (path: string) => path.split("/").at(-1) ?? path;
</script>

{#snippet held()}
    {#each folder.folders as child (child.name)}
        <FileTree folder={child} {active} {onSelect} />
    {/each}

    {#each folder.files as file (file)}
        <button
            class="file"
            class:active={file === active}
            onclick={() => onSelect(file)}
            data-file={file}
        >
            {name(file)}
        </button>
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
    .folder,
    .file {
        display: block;
        width: 100%;
        padding: 0.15rem 0.75rem;
        color: var(--muted);
        font-family: var(--mono);
        font-size: 12px;
        text-align: left;
    }

    .folder {
        display: flex;
        gap: 0.35rem;
        align-items: baseline;
    }

    .file {
        padding-left: 1.25rem;
    }

    .folder:hover,
    .file:hover {
        color: var(--text);
    }

    .folder:hover,
    .file:hover,
    .file.active {
        background: var(--raised);
    }

    .file.active {
        color: var(--text);
        box-shadow: inset 2px 0 0 var(--accent);
    }

    .children {
        margin-left: 0.55rem;
        border-left: 1px solid var(--border);
    }

    .caret {
        width: 0.75rem;
    }
</style>
