<script lang="ts">
    import TreeNode from './components/TreeNode.svelte';
    import type { Node } from './components/TreeNode.svelte';

    export let ast: Node | null;
    export let diagnostics: { message: string }[] = [];
</script>

<main>
    {#if ast}
        <ul>
            <TreeNode node={ast} />
        </ul>
    {:else}
        <div>Nothing</div>
    {/if}

    {#if diagnostics.length}
        <h3>Diagnostics</h3>
        <ul>
            {#each diagnostics as d}
                <li>{d.message}</li>
            {/each}
        </ul>
    {/if}
</main>
