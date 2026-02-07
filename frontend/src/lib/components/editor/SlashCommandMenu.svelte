<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	/** Menu items to display */
	export let items: SlashItem[] = [];
	/** Currently active (highlighted) index */
	export let activeIndex = 0;
	/** X position relative to editor */
	export let x = 0;
	/** Y position relative to editor */
	export let y = 0;

	interface SlashItem {
		id: string;
		label: string;
		description: string;
		icon?: string;
		group?: string;
	}

	const dispatch = createEventDispatcher<{
		select: SlashItem;
		close: void;
	}>();

	function handleSelect(item: SlashItem) {
		dispatch('select', item);
	}

	function handleMouseEnter(index: number) {
		activeIndex = index;
	}
</script>

<div
	class="absolute z-40 w-64 max-h-64 overflow-y-auto rounded-xl border border-slate-700 bg-slate-900 py-1 shadow-2xl"
	style="left: {x}px; top: {y}px;"
	role="listbox"
	aria-label="Commands"
>
	<div class="px-3 py-1 text-[10px] uppercase tracking-wide text-slate-500">Commands</div>
	{#each items as item, idx (item.id)}
		<button
			class={`flex w-full items-center gap-3 px-3 py-2 text-left text-xs transition ${
				idx === activeIndex
					? 'bg-sky-500/10 text-white'
					: 'text-slate-300 hover:bg-slate-800'
			}`}
			role="option"
			aria-selected={idx === activeIndex}
			on:mousedown|preventDefault={() => handleSelect(item)}
			on:mouseenter={() => handleMouseEnter(idx)}
		>
			{#if item.icon}
				<span class="w-5 text-center text-slate-400">{item.icon}</span>
			{/if}
			<span class="font-medium">{item.label}</span>
			<span class="text-[10px] text-slate-500">{item.description}</span>
		</button>
	{/each}
	{#if items.length === 0}
		<div class="px-3 py-2 text-xs text-slate-500">No matching commands</div>
	{/if}
</div>
