<script lang="ts">
	import type { Diagnostic } from '$lib/driver';

	interface Props {
		/** What the parser reported, in the order it reported it. */
		diagnostics: Diagnostic[];
	}

	let { diagnostics }: Props = $props();
</script>

{#if diagnostics.length === 0}
	<p class="empty">No diagnostics.</p>
{:else}
	<ul>
		{#each diagnostics as diagnostic (diagnostic.code + diagnostic.message)}
			<li class={diagnostic.level}>
				<span class="head">
					<span class="code">{diagnostic.category}:{diagnostic.code}</span>
					{diagnostic.message}
				</span>

				{#each diagnostic.labels as label}
					<span class="label">
						{label.line + 1}:{label.column + 1}{label.message ? ` — ${label.message}` : ''}
					</span>
				{/each}

				{#each diagnostic.notes as note}
					<span class="note">{note}</span>
				{/each}
			</li>
		{/each}
	</ul>
{/if}

<style>
	.empty {
		margin: 0;
		padding: 0.75rem;
		color: var(--muted);
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	li.error .code {
		color: var(--error);
	}

	li.warning .code {
		color: var(--warning);
	}

	.head {
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
	}

	.code {
		flex: none;
		color: var(--muted);
		font-family: var(--mono);
		font-size: 11px;
	}

	.label,
	.note {
		color: var(--muted);
		font-family: var(--mono);
		font-size: 12px;
	}

	.note {
		font-family: var(--sans);
	}
</style>
