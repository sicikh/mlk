<script lang="ts">
    import FileTree, { foldersOf } from "./FileTree.svelte";

    interface Props {
        /** The paths of the buffers the editor holds, in the order they were made. */
        files: string[];

        /** The buffer the editor is editing. */
        active: string;

        /**
         * Whether a person may write at a path.
         *
         * The standard library of the language is part of the compiler: its files are read,
         * and no buffer is made beside them, so the editor asks before it makes one.
         */
        writable: (path: string) => boolean;

        /** Called when a person picks another buffer. */
        onSelect: (path: string) => void;

        /** Called with the path of a buffer to make. */
        onCreate: (path: string) => void;

        /** Called with the path of a buffer to drop. */
        onRemove: (path: string) => void;
    }

    let { files, active, writable, onSelect, onCreate, onRemove }: Props =
        $props();

    /** Whether a person is typing the path of a buffer to make. */
    let making = $state(false);
    let typed = $state("");
    let problem = $state("");

    /** The buffer a person is being asked about: dropping one is not done on a stray click. */
    let dropping = $state<string | null>(null);

    /** The field a path is typed into, which takes the caret as soon as it appears. */
    let field = $state<HTMLInputElement | undefined>();

    $effect(() => {
        if (making) field?.focus();
    });

    /** The directories the paths make, which are the whole of the file system there is. */
    const tree = $derived(foldersOf(files));

    /**
     * A path is a name and the directories it sits in, so its hint says where it will land.
     *
     * A buffer made beside a file of the library would be one of the library's, so a file of the
     * library in front moves the hint to the root the editor's own buffers sit in.
     */
    const hint = $derived(
        `${writable(active) ? active.slice(0, active.lastIndexOf("/") + 1) : "/"}name.mlk`,
    );

    /** What a person typed, as the editor spells paths: one leading slash, no empty parts. */
    function pathOf(typed: string): string {
        const parts = typed.trim().split("/").filter(Boolean);

        return parts.length === 0 ? "" : `/${parts.join("/")}`;
    }

    /** Makes the buffer at the path that was typed, directories and all. */
    function make(event: SubmitEvent) {
        event.preventDefault();

        const path = pathOf(typed);

        if (path === "") return;

        if (files.includes(path)) {
            problem = "a buffer is already there";
            return;
        }

        if (!writable(path)) {
            problem = "the standard library is read-only";
            return;
        }

        onCreate(path);
        making = false;
        typed = "";
        problem = "";
    }

    /** Drops the buffer a person said yes to. */
    function drop(path: string) {
        dropping = null;
        onRemove(path);
    }
</script>

<div class="panel" data-panel="files">
    <header>
        <h2>Files</h2>
        <button
            class="add"
            onclick={() => (making = !making)}
            title="A new buffer, at any path"
            aria-expanded={making}>+</button
        >
    </header>

    {#if making}
        <form onsubmit={make}>
            <input
                bind:this={field}
                bind:value={typed}
                onkeydown={(event) =>
                    event.key === "Escape" && (making = false)}
                placeholder={hint}
                aria-label="The path of the buffer to make"
                spellcheck="false"
                autocomplete="off"
            />
            {#if problem !== ""}
                <p class="problem">{problem}</p>
            {/if}
        </form>
    {/if}

    <div class="tree">
        <FileTree
            folder={tree}
            {active}
            {writable}
            confirming={dropping}
            {onSelect}
            onConfirm={(path) => (dropping = path)}
            onRemove={drop}
        />
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

    form {
        padding: 0.35rem 0.5rem;
        border-bottom: 1px solid var(--border);
    }

    input {
        width: 100%;
        padding: 0.2rem 0.35rem;
        background: var(--bg);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        color: var(--text);
        font-family: var(--mono);
        font-size: 12px;
        outline: none;
    }

    input:focus {
        border-color: var(--accent);
    }

    .problem {
        margin: 0.25rem 0 0;
        color: var(--warning);
        font-size: 11px;
    }

    .tree {
        flex: 1;
        padding: 0.25rem 0;
        overflow: auto;
    }

    /* A finger taps what a pointer clicks, and a field it writes in is read without zooming
       the page: a phone zooms a field whose text it thinks is too small to read. */
    @media (max-width: 860px), (max-height: 520px) {
        header {
            padding: 0.5rem 0.5rem 0.5rem 0.75rem;
        }

        .add {
            padding: 0 0.6rem;
            font-size: 20px;
        }

        input {
            padding: 0.35rem 0.45rem;
            font-size: 16px;
        }
    }
</style>
