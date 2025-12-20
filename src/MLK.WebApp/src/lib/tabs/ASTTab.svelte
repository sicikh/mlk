
<script lang="ts">
    import TreeNode from './components/TreeNode.svelte';
    import type { Node } from './components/TreeNode.svelte';

    export let ast: Node | null;

</script>

<main>
    {#if ast}
        <ul>
            <TreeNode node={ast} />
        </ul>
    {:else}
        <div>Nothing</div>
    {/if}
</main>