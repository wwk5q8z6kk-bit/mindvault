<script context="module" lang="ts">
	export type FilterDef = {
		key: string;
		label: string;
		options: { value: string; label: string }[];
	};
</script>

<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	type FilterDef = {
		key: string;
		label: string;
		options: { value: string; label: string }[];
	};

	export let filters: FilterDef[] = [];
	export let values: Record<string, string> = {};
	export let searchValue = '';
	export let searchPlaceholder = 'Search...';

	const dispatch = createEventDispatcher<{
		search: string;
		filter: { key: string; value: string };
		clear: void;
	}>();

	// Determine if any filters are active
	$: hasActiveFilters =
		searchValue.trim() !== '' || Object.values(values).some((v) => v !== '' && v !== 'all');
</script>

<div class="flex flex-wrap items-center gap-2">
	<input
		class="flex-1 min-w-[160px] rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-2 text-sm text-[rgb(var(--mv-text))] placeholder:text-[rgb(var(--mv-muted))]/60 focus:border-[rgb(var(--mv-accent))] focus:outline-none focus:ring-1 focus:ring-[rgb(var(--mv-accent))]/50 transition-all duration-200"
		placeholder={searchPlaceholder}
		aria-label="Search"
		bind:value={searchValue}
		on:input={() => dispatch('search', searchValue)}
	/>
	{#each filters as filter (filter.key)}
		<select
			class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
			aria-label={filter.label}
			value={values[filter.key] ?? ''}
			on:change={(e) => dispatch('filter', { key: filter.key, value: e.currentTarget.value })}
		>
			{#each filter.options as option (option.value)}
				<option value={option.value}>{option.label}</option>
			{/each}
		</select>
	{/each}
	{#if hasActiveFilters}
		<button
			class="rounded-lg border border-[rgb(var(--mv-border))] px-2.5 py-2 text-xs text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))] hover:border-[rgb(var(--mv-muted))]/40 transition"
			on:click={() => dispatch('clear')}
		>
			Clear
		</button>
	{/if}
</div>
