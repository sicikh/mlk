<script lang="ts" module>
	/** One line of what the editor has to say for itself. */
	export interface Line {
		/** How to read it: what happened, what it is worth knowing, or what went wrong. */
		level: 'info' | 'note' | 'error';

		/** The line itself. */
		text: string;
	}
</script>

<script lang="ts">
	interface Props {
		/** What the editor has said so far, oldest first. */
		lines: Line[];
	}

	let { lines }: Props = $props();

	let view: HTMLDivElement;

	/** A console shows the newest line: it scrolls itself to the bottom. */
	$effect(() => {
		if (lines.length > 0) view.scrollTop = view.scrollHeight;
	});
</script>

<div class="console" data-panel="console" bind:this={view}>
	{#each lines as line, index (index)}
		<div class="line {line.level}">{line.text}</div>
	{/each}
</div>

<style>
	.console {
		height: 100%;
		padding: 0.4rem 0.75rem;
		overflow: auto;
		background: var(--surface);
		border-top: 1px solid var(--border);
		font-family: var(--mono);
		font-size: 12px;
	}

	.line {
		white-space: pre-wrap;
		color: var(--muted);
	}

	.line.info {
		color: var(--text);
	}

	.line.error {
		color: var(--error);
	}
</style>
