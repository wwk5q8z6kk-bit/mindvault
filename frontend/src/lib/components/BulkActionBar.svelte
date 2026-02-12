<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let selectedCount = 0;
	export let actions: { label: string; variant?: 'default' | 'danger'; handler: () => void }[] = [];

	const dispatch = createEventDispatcher<{ selectAll: void; clear: void }>();
</script>

{#if selectedCount > 0}
	<div
		class="fixed bottom-6 left-1/2 z-40 flex -translate-x-1/2 items-center gap-3 rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/95 px-5 py-3 shadow-xl backdrop-blur"
	>
		<span class="text-xs font-medium text-[rgb(var(--mv-text))]">{selectedCount} selected</span>
		<div class="h-4 w-px bg-[rgb(var(--mv-panel-strong))]"></div>
		{#each actions as action (action.label)}
			<button
				class={`rounded-md px-2.5 py-1 text-[11px] transition ${
					action.variant === 'danger'
						? 'bg-red-500/20 text-red-200 hover:bg-red-500/30'
						: 'bg-[rgb(var(--mv-accent))]/15 text-[rgb(var(--mv-accent))] hover:bg-[rgb(var(--mv-accent))]/25'
				}`}
				on:click={action.handler}
			>
				{action.label}
			</button>
		{/each}
		<div class="h-4 w-px bg-[rgb(var(--mv-panel-strong))]"></div>
		<button
			class="text-[11px] text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]"
			on:click={() => dispatch('clear')}
		>
			Clear
		</button>
	</div>
{/if}
