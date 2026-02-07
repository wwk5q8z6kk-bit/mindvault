<script lang="ts">
	import { onMount, onDestroy, tick } from 'svelte';
	import {
		onSearchStateChange,
		getSearchState,
		setSearchQuery,
		setReplaceText,
		toggleCaseSensitive,
		toggleRegex,
		nextMatch,
		prevMatch,
		replaceCurrentMatch,
		replaceAllMatches,
		closeSearch
	} from './index';

	let isOpen = false;
	let query = '';
	let replaceText = '';
	let caseSensitive = false;
	let useRegex = false;
	let matchCount = 0;
	let currentIndex = -1;
	let showReplace = false;

	let searchInput: HTMLInputElement;
	let unsubscribe: (() => void) | null = null;

	onMount(() => {
		// Get initial state
		const state = getSearchState();
		syncState(state);

		// Subscribe to state changes
		unsubscribe = onSearchStateChange((state) => {
			syncState(state);
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function syncState(state: ReturnType<typeof getSearchState>) {
		const wasOpen = isOpen;
		isOpen = state.isOpen;
		query = state.query;
		replaceText = state.replaceText;
		caseSensitive = state.caseSensitive;
		useRegex = state.useRegex;
		matchCount = state.matches.length;
		currentIndex = state.currentMatchIndex;
		showReplace = state.showReplace;

		// Focus input when opening
		if (isOpen && !wasOpen) {
			tick().then(() => {
				searchInput?.focus();
				searchInput?.select();
			});
		}
	}

	function handleQueryInput(event: Event) {
		const value = (event.target as HTMLInputElement).value;
		setSearchQuery(value);
	}

	function handleReplaceInput(event: Event) {
		const value = (event.target as HTMLInputElement).value;
		setReplaceText(value);
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			closeSearch();
		} else if (event.key === 'Enter') {
			event.preventDefault();
			if (event.shiftKey) {
				prevMatch();
			} else {
				nextMatch();
			}
		}
	}

	function handleReplaceKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			closeSearch();
		} else if (event.key === 'Enter') {
			event.preventDefault();
			replaceCurrentMatch();
		}
	}

	$: matchDisplay = matchCount > 0
		? `${currentIndex + 1} of ${matchCount}`
		: query
			? 'No results'
			: '';
</script>

{#if isOpen}
	<div
		class="absolute right-4 top-2 z-30 flex flex-col gap-2 rounded-lg border border-slate-700 bg-slate-900 p-3 shadow-xl"
	>
		<!-- Search row -->
		<div class="flex items-center gap-2">
			<div class="relative flex-1">
				<input
					bind:this={searchInput}
					type="text"
					class="w-full rounded border border-slate-700 bg-slate-800 px-3 py-1.5 pr-16 text-xs text-white outline-none placeholder:text-slate-500 focus:border-sky-500"
					placeholder="Find..."
					value={query}
					on:input={handleQueryInput}
					on:keydown={handleKeyDown}
				/>
				{#if matchDisplay}
					<span class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-500">
						{matchDisplay}
					</span>
				{/if}
			</div>

			<!-- Navigation buttons -->
			<button
				class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white disabled:opacity-40"
				disabled={matchCount === 0}
				on:click={() => prevMatch()}
				title="Previous match (Shift+Enter)"
			>
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
				</svg>
			</button>
			<button
				class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white disabled:opacity-40"
				disabled={matchCount === 0}
				on:click={() => nextMatch()}
				title="Next match (Enter)"
			>
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
				</svg>
			</button>

			<!-- Options -->
			<button
				class="rounded px-1.5 py-0.5 text-[10px] font-medium transition-colors"
				class:bg-sky-500={caseSensitive}
				class:text-white={caseSensitive}
				class:text-slate-500={!caseSensitive}
				class:hover:text-slate-300={!caseSensitive}
				on:click={() => toggleCaseSensitive()}
				title="Match case"
			>
				Aa
			</button>
			<button
				class="rounded px-1.5 py-0.5 text-[10px] font-medium transition-colors"
				class:bg-sky-500={useRegex}
				class:text-white={useRegex}
				class:text-slate-500={!useRegex}
				class:hover:text-slate-300={!useRegex}
				on:click={() => toggleRegex()}
				title="Use regular expression"
			>
				.*
			</button>

			<!-- Toggle replace -->
			<button
				class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white"
				class:text-sky-400={showReplace}
				on:click={() => (showReplace = !showReplace)}
				title="Toggle replace"
			>
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
				</svg>
			</button>

			<!-- Close -->
			<button
				class="rounded p-1 text-slate-400 hover:bg-slate-800 hover:text-white"
				on:click={() => closeSearch()}
				title="Close (Escape)"
			>
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</button>
		</div>

		<!-- Replace row -->
		{#if showReplace}
			<div class="flex items-center gap-2">
				<input
					type="text"
					class="flex-1 rounded border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs text-white outline-none placeholder:text-slate-500 focus:border-sky-500"
					placeholder="Replace with..."
					value={replaceText}
					on:input={handleReplaceInput}
					on:keydown={handleReplaceKeyDown}
				/>
				<button
					class="rounded px-2 py-1 text-[10px] text-slate-400 hover:bg-slate-800 hover:text-white disabled:opacity-40"
					disabled={matchCount === 0}
					on:click={() => replaceCurrentMatch()}
					title="Replace current match"
				>
					Replace
				</button>
				<button
					class="rounded px-2 py-1 text-[10px] text-slate-400 hover:bg-slate-800 hover:text-white disabled:opacity-40"
					disabled={matchCount === 0}
					on:click={() => replaceAllMatches()}
					title="Replace all matches"
				>
					All
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	/* Search highlight styles */
	:global(.mv-search-highlight) {
		background-color: rgba(250, 204, 21, 0.3);
		border-radius: 2px;
	}

	:global(.mv-search-highlight-current) {
		background-color: rgba(250, 204, 21, 0.6);
		outline: 2px solid rgb(250, 204, 21);
		outline-offset: -1px;
	}
</style>
