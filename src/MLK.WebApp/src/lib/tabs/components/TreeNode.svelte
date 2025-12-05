<script lang="ts">
    export interface Node {
        hidden?: boolean;
        name: string;
        children?: Node[];
        range?: [number, number];
        type?: string;
        [key: string]: any;
    }
    
    

    export let node: Node;
</script>

<li class="node">
    <div class="node-header">
        <span class="node-name"> •ㅤ{node.name}</span>
        {#if node.children && node.children.length > 0}
            {#if !node.hidden}
                <button on:click={() => node.hidden = true}>
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-dasharray="16" stroke-dashoffset="16" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14"><animate fill="freeze" attributeName="stroke-dashoffset" dur="0.4s" values="16;0"/></path></svg>
                </button>
            {:else}
                <button on:click={() => node.hidden = false}>
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M18 12.998h-5v5a1 1 0 0 1-2 0v-5H6a1 1 0 0 1 0-2h5v-5a1 1 0 0 1 2 0v5h5a1 1 0 0 1 0 2"/></svg>
                </button>
            {/if}
        {/if}
        {#if node.range}
            <span class="node-range">[{node.range[0]}, {node.range[1]}]</span>
        {/if}

 
        {#if node.type}
            <span class="node-type">Type: {node.type}</span>
        {/if}

    </div>

    {#if node.children && node.children.length > 0 && !node.hidden}
        <ul>
            {#each node.children as child}
                <svelte:self node={child} />
            {/each}
        </ul>
    {:else if !node.children || node.children.length === 0}
        <div class="leaf-node">(лист)</div>
    {/if}
</li>

<style>
    ul {
        list-style-type: none;
        padding-left: 60px;
        margin: 10px 0;
    }


    
    .node-header {
        margin: 8px 0;
        padding: 4px;
        border-radius: 4px;
        display: flex;
        align-items: center;
        gap: 10px;
    }

    button {
        border-radius: 100px;
        border : 2px solid #000000;
        cursor: pointer;
        margin: 4px 0;
        font-size: 25px;
    }

    .leaf-node {
        margin-left: 20px;
        color: #666;
        font-style: italic;
    }
</style>