<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { AssistLinkSuggestion } from '$lib/api/assist';

	/** Suggestions to display */
	export let suggestions: AssistLinkSuggestion[] = [];
	/** Currently active (highlighted) index */
	export let activeIndex = 0;
	/** X position relative to editor */
	export let x = 0;
	/** Y position relative to editor */
	export let y = 0;
	/** Whether the menu is loading */
	export let loading = false;
	/** The current query */
	export let query = '';

	const dispatch = createEventDispatcher<{
		select: AssistLinkSuggestion;
		close: void;
		createNew: { title: string };
	}>();

	function handleSelect(suggestion: AssistLinkSuggestion) {
		dispatch('select', suggestion);
	}

	function handleCreateNew() {
		dispatch('createNew', { title: query });
	}

	function handleMouseEnter(index: number) {
		activeIndex = index;
	}

	function getDisplayTitle(suggestion: AssistLinkSuggestion): string {
		if (suggestion.heading) {
			return `${suggestion.title} > ${suggestion.heading}`;
		}
		return suggestion.title;
	}

	function truncatePreview(text: string | undefined, maxLen = 80): string {
		if (!text) return '';
		if (text.length <= maxLen) return text;
		return text.slice(0, maxLen).trim() + '...';
	}
</script>

<div
	class="absolute z-50 w-72 max-h-64 overflow-y-auto rounded-xl border border-slate-700 bg-slate-900 py-1 shadow-2xl"
	style="left: {x}px; top: {y}px;"
	role="listbox"
	aria-label="Mention suggestions"
>
	<div class="px-3 py-1 text-[10px] uppercase tracking-wide text-slate-500 flex items-center gap-2">
		<span>Link to note</span>
		{#if loading}
			<span class="animate-pulse">...</span>
		{/if}
	</div>

	{#if suggestions.length > 0}
		{#each suggestions as suggestion, idx (suggestion.node_id || suggestion.title)}
			<button
				class={`flex w-full flex-col gap-0.5 px-3 py-2 text-left transition ${
					idx === activeIndex
						? 'bg-purple-500/10 text-white'
						: 'text-slate-300 hover:bg-slate-800'
				}`}
				role="option"
				aria-selected={idx === activeIndex}
				on:mousedown|preventDefault={() => handleSelect(suggestion)}
				on:mouseenter={() => handleMouseEnter(idx)}
			>
				<span class="flex items-center gap-2">
					<span class="text-purple-400">[[</span>
					<span class="text-xs font-medium">{getDisplayTitle(suggestion)}</span>
					<span class="text-purple-400">]]</span>
				</span>
				{#if suggestion.preview}
					<span class="text-[10px] text-slate-500 pl-4">
						{truncatePreview(suggestion.preview)}
					</span>
				{/if}
				{#if suggestion.reason}
					<span class="text-[10px] text-slate-600 pl-4 italic">
						{suggestion.reason}
					</span>
				{/if}
			</button>
		{/each}
	{:else if !loading && query.length > 0}
		<div class="px-3 py-2 text-xs text-slate-500">
			No matching notes found
		</div>
	{/if}

	{#if query.length > 0}
		<div class="border-t border-slate-800 mt-1 pt-1">
			<button
				class={`flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition ${
					activeIndex === suggestions.length
						? 'bg-sky-500/10 text-white'
						: 'text-slate-400 hover:bg-slate-800'
				}`}
				role="option"
				aria-selected={activeIndex === suggestions.length}
				on:mousedown|preventDefault={handleCreateNew}
				on:mouseenter={() => (activeIndex = suggestions.length)}
			>
				<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
				</svg>
				<span>Create new note: <span class="font-medium text-white">"{query}"</span></span>
			</button>
		</div>
	{/if}
</div>
