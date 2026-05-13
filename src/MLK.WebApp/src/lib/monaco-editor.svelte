<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import loader from '@monaco-editor/loader';
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';

  export let value = '';
  export let language = 'javascript';
  export let theme: Monaco.editor.BuiltinTheme = 'hc-light';

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

    /* Фон редактора */
    :global(.monaco-container .monaco-editor),
    :global(.monaco-container .monaco-editor .margin),
    :global(.monaco-container .monaco-editor-background) {
        background-color: #e0e0e0 !important;
    }

    /* Фон minimap (уменьшенное окно справа) */
    :global(.monaco-container .minimap) {
        background-color: #c1c1c1 !important;
    }

    /* Слайдер внутри minimap */
    :global(.monaco-container .minimap .minimap-shadow-visible) {
        background: rgba(0, 0, 0, 0.1) !important;
    }

    /* Слайдер minimap */
    :global(.monaco-container .minimap .minimap-slider) {
        background: rgba(100, 100, 100, 0.2) !important;
    }

    /* Scrollbar - вертикальный */
    :global(.monaco-container .monaco-scrollable-element .scrollbar.vertical) {
        background: #e0e0e0 !important;
    }

    /* Слайдер скроллбара */
    :global(.monaco-container .monaco-scrollable-element .scrollbar .slider) {
        background: rgba(100, 100, 100, 0.4) !important;
    }

    /* Активный слайдер скроллбара */
    :global(.monaco-container .monaco-scrollable-element .scrollbar .slider.active) {
        background: rgba(100, 100, 100, 0.6) !important;
    }

    /* Скроллбар при наведении */
    :global(.monaco-container .monaco-scrollable-element .scrollbar .slider:hover) {
        background: rgba(100, 100, 100, 0.7) !important;
    }

    /* Горизонтальный скроллбар */
    :global(.monaco-container .monaco-scrollable-element .scrollbar.horizontal) {
        background: #e0e0e0 !important;
    }

    :global(.monaco-container .monaco-scrollable-element .scrollbar.horizontal .slider) {
        background: rgba(100, 100, 100, 0.4) !important;
    }

    /* Граница minimap */
    :global(.monaco-container .minimap-shadow-visible) {
        box-shadow: inset -6px 0 6px -3px rgba(0, 0, 0, 0.2) !important;
    }
</style>