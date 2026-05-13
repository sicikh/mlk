<script lang="ts">
    import ASTTab from '$lib/tabs/ASTTab.svelte'
    import OtherTab from '$lib/tabs/OtherTab.svelte'
    import { writable } from 'svelte/store';
    import MonacoEditor from '$lib/monaco-editor.svelte';
    import { buildAstFromSource } from '$lib/language/generated/AstApi.js';

    let code = "id 42";

    const selectedResult = writable('AST');
    export interface Node {
        hidden?: boolean;
        name: string;
        children?: Node[];
        range?: [number, number];
        type?: string;
        [key: string]: any;
    }

    interface DiagnosticDto {
        message: string;
    }

    interface AstResponseDto {
        diagnostics: DiagnosticDto[];
        tree?: Node | null;
    }

    let ast: Node | null = null;
    let diagnostics: DiagnosticDto[] = [];

    function run() {
        let res : AstResponseDto = JSON.parse(JSON.stringify(buildAstFromSource(code)));
        ast = res.tree ?? null;
        diagnostics = res.diagnostics;
    }

    let timeout: number | undefined;

    $: if (code !== undefined) {
        clearTimeout(timeout);
        timeout = setTimeout(() => {
            void run();
        }, 300);
    }
</script>

<div class="main">
    <div class="header">
        <button>mlk</button>
    </div>

    <div class="toolbar">
        <button class="runbutton" on:click={run}>Run</button>
        <div class="center-group">
            Results
            <select bind:value={$selectedResult} class="select-result">
                <option value="AST">AST</option>
                <option value="smth">smth</option>
            </select>
        </div>
        
        <div class="right-group">
            <button><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M12 22c5.523 0 10-4.477 10-10c0-.463-.694-.54-.933-.143a6.5 6.5 0 1 1-8.924-8.924C12.54 2.693 12.463 2 12 2C6.477 2 2 6.477 2 12s4.477 10 10 10"/></svg></button>
            <button><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10s-4.477 10-10 10m-1-11v6h2v-6zm0-4v2h2V7z"/></svg></button>
            <button><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="m8.686 4l2.607-2.607a1 1 0 0 1 1.414 0L15.314 4H19a1 1 0 0 1 1 1v3.686l2.607 2.607a1 1 0 0 1 0 1.414L20 15.314V19a1 1 0 0 1-1 1h-3.686l-2.607 2.607a1 1 0 0 1-1.414 0L8.686 20H5a1 1 0 0 1-1-1v-3.686l-2.607-2.607a1 1 0 0 1 0-1.414L4 8.686V5a1 1 0 0 1 1-1zM12 15a3 3 0 1 0 0-6a3 3 0 0 0 0 6"/></svg></button>
        </div>
    </div>

</div>

<div class="input">
    <div class="editor-wrapper">
        <MonacoEditor bind:value={code} />
    </div>
</div>

<div class="console-output">
    console
</div>

<div class="output">
    <div class="ast-tab-container">
        {#if $selectedResult === 'AST'}
            {#if ast}
                <ASTTab {ast} />
            {:else}
                <div>Nothing</div>
            {/if}
        {:else}
            <OtherTab />
        {/if}
    </div>
</div>



<style>
    @import url('https://fonts.googleapis.com/css2?family=Reem+Kufi+Fun:wght@400..700&display=swap');
    
    :global(html, body) {
        background-color: #e0e0e0;
        margin: 0;
        padding: 0;
        height: 100%;
        font-family: "Reem Kufi Fun", sans-serif;
    }

    .right-group {
        display: flex;
    }

    .right-group button {
        padding: 5px 5px;
        border: 4px solid #000000;
        margin: 0;
        border-radius: 0;
        cursor: pointer;
    }
    
    .right-group button:hover {
        background-color: #aaaaaa;
    }

    .right-group button:not(:last-child) {
        border-right: none;
    }


    
    .select-result {
        background-color: #e0e0e0;
    }

    .select-result:hover {
        background-color: #d1d1d1;
    }
    
    .toolbar select {
        border: 3px solid #000000;
        padding: 0px 60px;
        font-size: 25px;
        border-radius: 0;
    }
    
    .runbutton:hover {
        background-color: #59d851;
    }


    .main {
        position: relative;
        z-index: 1;
        background-color: #000000;
    }

    .runbutton {
        border: 3px solid #000000;
        background-color: #78fd71;
        padding: 1px 40px;
        border-radius: 5px;
        cursor: pointer;
        font-size: 23px;
    }

    .input {
        position: absolute;
        border-top: 2px solid #000000;
        border-right: 2px solid #000000;
        border-bottom: 2px solid #000000;
        height: 70%;
        top: 60px;
        left: 0px;
        width: 50%;
        z-index: 0; 
        display: flex;
        flex-direction: column;
    }
    
    .console-output {
        position: absolute;
        border-top: 2px solid #000000;
        border-right: 2px solid #000000;
        border-bottom: 2px solid #000000;
        height: 70%;
        top:725px;
        left: 0px;
        width: 50%;
        z-index: 0;
        display: flex;
        flex-direction: column;
        padding: 20px;
    }

    .editor-wrapper {
        flex: 1;            
    }

    .output {
        position: absolute;
        border-top: 2px solid #000000;
        border-left: 2px solid #000000;
        height: calc(100% - 60px); 
        top: 60px; 
        right: 0px;
        width: 50%;
        z-index: 0; 
    }

    .header {
        text-align: left;
        position: absolute;
        left: 0;
        top: 0;
        border-bottom: 2.5px solid #000000;
        border-right: 2px solid #000000;
        width: 50%;
        height: 60px;
        box-sizing: border-box;
        padding: 10px 5px;
        display: flex;
        align-items: center;
        font-size: 50px;
        z-index: 2;
        color: #fd72c3;

        -webkit-text-stroke: 6px black;
        paint-order: stroke fill;
        text-stroke: 6px black;
    }
    
    .header button {
        cursor: pointer;
    }
    
    .header button:hover {
        color: #cf64a2;
    }

    .toolbar {
        text-align: center;
        position: absolute;
        right: 0;
        top: 0;
        border-bottom: 2.5px solid #000000;
        border-left: 2px solid #000000;
        width: 50%;
        height: 60px;
        box-sizing: border-box;
        padding: 10px 20px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        z-index: 2;
    }

    .center-group {
        display: flex;
        align-items: center;
        gap: 10px;
        position: absolute;
        left: 50%;
        transform: translateX(-50%);
        font-size: 40px;
    }

    .toolbar button {
        margin: 0;
    }

    .toolbar select {
        margin: 0;
        cursor: pointer;
    }

    .ast-tab-container {
        display: flex;
        justify-content: left;
        margin: 30px;
    }
</style>
