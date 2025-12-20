<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import loader from '@monaco-editor/loader';
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';

  export let value = '';
  export let language = 'javascript';
  export let theme: Monaco.editor.BuiltinTheme = 'vs-dark';

  let container: HTMLDivElement;
  let monaco: typeof Monaco;
  let editor: Monaco.editor.IStandaloneCodeEditor;

  onMount(async () => {
    monaco = await loader.init();
    
    editor = monaco.editor.create(container, {
      value,
      language,
      theme,
      automaticLayout: true
    });

    editor.onDidChangeModelContent(() => {
      value = editor.getValue();
      const ev = new CustomEvent('change', { detail: value });
      dispatchEvent(ev);
    });
  });

  onDestroy(() => {
    editor?.dispose();
    monaco?.editor.getModels().forEach(m => m.dispose());
  });
</script>

<div bind:this={container} class="monaco-container"></div>

<style>
  .monaco-container {
    width: 100%;
    height: 100%;
  }
</style>