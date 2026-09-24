<script lang="ts">
	import { isToken, type SyntaxNode } from '$lib/driver';
	import TreeView from './TreeView.svelte';

	interface Props {
		/** The node to show, with everything under it. */
		node: SyntaxNode;
	}

	let { node }: Props = $props();

	/** Whether the node shows what it holds: a person folds what they are not reading. */
	let open = $state(true);

	const token = $derived(isToken(node));
	const children = $derived(node.children ?? []);

	/** Whether the parser is saying that something here is wrong. */
	const broken = (kind: string) => kind === 'ERROR_TOKEN' || kind.startsWith('BOGUS');

	const range = $derived(`${node.text_range[0]}..${node.text_range[1]}`);
</script>

<div class="node" data-kind={node.kind} class:token>
	{#if token}
		<span class="row">
			<span class="spacer"></span>
			<span class="kind" class:broken={broken(node.kind)}>{node.kind}</span>
			<span class="text">{JSON.stringify(node.text ?? '')}</span>
			<span class="range">{range}</span>
		</span>
	{:else}
		<button class="row" onclick={() => (open = !open)} aria-expanded={open}>
			<span class="caret">{open ? '▾' : '▸'}</span>
			<span class="kind" class:broken={broken(node.kind)}>{node.kind}</span>
			<span class="count">{children.length}</span>
			<span class="range">{range}</span>
		</button>

		{#if open && children.length > 0}
			<div class="children">
				{#each children as child, index (index)}
					<TreeView node={child} />
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style>
	.node {
		font-family: var(--mono);
		font-size: 12px;
		white-space: nowrap;
	}

	.children {
		margin-left: 0.55rem;
		padding-left: 0.75rem;
		border-left: 1px solid var(--border);
	}

	.row {
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
		width: 100%;
		text-align: left;
	}

	.caret {
		width: 0.75rem;
		color: var(--muted);
	}

	.spacer {
		width: 0.75rem;
	}

	.node:hover > .row {
		background: var(--raised);
	}

	.kind {
		color: var(--accent);
	}

	.kind.broken {
		color: var(--error);
	}

	.token .kind {
		color: var(--muted);
	}

	.text {
		color: var(--ok);
	}

	.count,
	.range {
		color: var(--muted);
		font-size: 11px;
	}

	.count::before {
		content: '· ';
	}
</style>
