<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import type { SavedView, SavedViewType } from '$lib/api/saved-views';
	import {
		activeSavedViewId,
		loadSavedViews,
		savedViewsLoading,
		savedViewsStore,
		setActiveSavedView
	} from '$lib/stores/saved-views';

	export let currentView: SavedViewType = 'list';

	const dispatch = createEventDispatcher<{ apply: SavedView | null }>();

	let search = '';
	let open = false;

	onMount(() => {
		loadSavedViews().catch(() => {
			// handled elsewhere
		});
	});

	$: filtered = $savedViewsStore.filter((view) => {
		if (!search.trim()) return true;
		const q = search.trim().toLowerCase();
		return view.name.toLowerCase().includes(q);
	});

	$: ordered = filtered
		.slice()
		.sort((a, b) => {
			const aMatch = a.view_type === currentView ? 0 : 1;
			const bMatch = b.view_type === currentView ? 0 : 1;
			if (aMatch != bMatch) return aMatch - bMatch;
			return a.name.localeCompare(b.name);
		});

	function handleSelect(view: SavedView) {
		setActiveSavedView(view);
		dispatch('apply', view);
		open = false;
	}

	function clearSelection() {
		setActiveSavedView(null);
		dispatch('apply', null);
	}
</script>

<div class="relative">
	<div class="flex items-center justify-between">
		<div>
			<p class="text-[11px] uppercase tracking-wide text-slate-500">Saved Views</p>
			<p class="text-[10px] text-slate-500">Apply across list / kanban / calendar</p>
		</div>
		<button
			class="rounded-md border border-slate-800 px-2 py-1 text-[11px] text-slate-300 hover:bg-slate-800"
			on:click={() => (open = !open)}
		>
			{open ? 'Close' : 'Browse'}
		</button>
	</div>

	{#if open}
		<div class="absolute z-20 mt-2 w-full rounded-xl border border-slate-800 bg-slate-950 p-3 shadow-lg">
			<input
				class="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
				placeholder="Search saved views"
				bind:value={search}
			/>

			<div class="mt-3 flex flex-col gap-2">
				{#if $savedViewsLoading}
					<p class="text-xs text-slate-500">Loading saved views…</p>
				{:else if filtered.length === 0}
					<p class="text-xs text-slate-500">No saved views yet.</p>
				{:else}
					{#each ordered as view (view.id)}
						<button
							class={`rounded-lg border px-3 py-2 text-left text-xs transition ${
								$activeSavedViewId === view.id
									? 'border-sky-500 bg-sky-500/10 text-sky-200'
									: 'border-slate-800 bg-slate-900/40 text-slate-200 hover:border-slate-700'
							}`}
							on:click={() => handleSelect(view)}
						>
							<div class="flex items-center justify-between gap-2">
								<span class="font-semibold">{view.name}</span>
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">
									{view.view_type}
								</span>
							</div>
							{#if view.query}
								<p class="mt-1 line-clamp-1 text-[10px] text-slate-500">{view.query}</p>
							{/if}
						</button>
					{/each}
				{/if}
			</div>

			<div class="mt-3 flex items-center justify-between">
				<p class="text-[10px] text-slate-500">Active: {$activeSavedViewId ?? 'none'}</p>
				<button
					class="text-[10px] text-slate-400 hover:text-slate-200"
					on:click={clearSelection}
				>
					Clear
				</button>
			</div>
		</div>
	{/if}
</div>
