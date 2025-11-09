<script lang="ts">
    import ASTTab from '$lib/tabs/ASTTab.svelte'
</script>

<div class="main">
    <div class="header">
        <h2>MLK</h2>
    </div>

    <div class="toolbar">
        <button>Run</button>
        <div class="center-group">
            Results
            <select>
                <option value="AST">AST</option>
            </select>
        </div>
    </div>

</div>

<div class="ast-tab-container">
    <ASTTab></ASTTab>
</div>

<style>
    .header {
        text-align: left;
        position: absolute;
        left: 0;
        top: 0;
        border-bottom: 2px solid #000000;
        border-right: 1px solid #000000;
        width: 50%;
        height: 60px;
        box-sizing: border-box;
        padding: 10px 20px;
        display: flex;
        align-items: center;
    }

    .toolbar {
        text-align: center;
        position: absolute;
        right: 0;
        top: 0;
        border-bottom: 2px solid #000000;
        border-left: 1px solid #000000;
        width: 50%;
        height: 60px;
        box-sizing: border-box;
        padding: 10px 20px;
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .center-group {
        display: flex;
        align-items: center;
        gap: 10px;
        position: absolute;
        left: 50%;
        transform: translateX(-50%);
    }

    .toolbar button {
        margin: 0;
    }

    .toolbar select {
        margin: 0;
    }
    
    .ast-tab-container {
        display: flex;
        justify-content: center;
        align-items: center;
        margin-top: 150px; 
    }
</style>