<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { goto } from '$app/navigation';
	import { searchFts, searchHybrid, type SearchResultDto } from '$lib/api/search';
	import { kindLabel, kindBadgeClass } from '$lib/utils/kind-helpers';
	import { recentItems, recentNotes, recentTasks, recentSearches, type RecentItem } from '$lib/stores/recent';

	let open = false;
	let query = '';
	let results: SearchResultDto[] = [];
	let loading = false;
	let selectedIndex = 0;
	let inputEl: HTMLInputElement | null = null;
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let searchMode: 'fulltext' | 'hybrid' = 'fulltext';

	// Recent items for empty state
	$: showRecent = !query.trim() && !loading;
	$: recentCombined = [...$recentNotes, ...$recentTasks].sort((a, b) => b.timestamp - a.timestamp).slice(0, 6);

	function handleGlobalKeydown(event: KeyboardEvent) {
		// Cmd+/ or Ctrl+/ to open quick search
		if ((event.metaKey || event.ctrlKey) && event.key === '/') {
			event.preventDefault();
			toggleOpen();
			return;
		}

		if (!open) return;

		if (event.key === 'Escape') {
			event.preventDefault();
			close();
			return;
		}

		const maxIndex = showRecent ? recentCombined.length - 1 : results.length - 1;
		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				selectedIndex = Math.min(selectedIndex + 1, maxIndex);
				break;
			case 'ArrowUp':
				event.preventDefault();
				selectedIndex = Math.max(selectedIndex - 1, 0);
				break;
			case 'Enter':
				event.preventDefault();
				if (showRecent && recentCombined[selectedIndex]) {
					navigateToRecent(recentCombined[selectedIndex]);
				} else if (results[selectedIndex]) {
					navigateToResult(results[selectedIndex]);
				}
				break;
			case 'Tab':
				event.preventDefault();
				searchMode = searchMode === 'fulltext' ? 'hybrid' : 'fulltext';
				void doSearch();
				break;
		}
	}

	async function toggleOpen() {
		open = !open;
		if (open) {
			query = '';
			results = [];
			selectedIndex = 0;
			await tick();
			inputEl?.focus();
		}
	}

	function close() {
		open = false;
		query = '';
		results = [];
	}

	function onInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			void doSearch();
		}, 150);
	}

	async function doSearch() {
		const q = query.trim();
		if (!q) {
			results = [];
			return;
		}

		loading = true;
		try {
			if (searchMode === 'hybrid') {
				results = await searchHybrid(q, 10);
			} else {
				results = await searchFts(q, 10);
			}
			selectedIndex = 0;
			// Track search query
			if (results.length > 0) {
				recentItems.addSearch(q);
			}
		} catch {
			results = [];
		} finally {
			loading = false;
		}
	}

	function navigateToResult(result: SearchResultDto) {
		const node = result.node;
		close();

		// Track in recent items
		if (node.kind === 'task') {
			recentItems.addTask(node.id, node.title);
			goto(`/tasks?task=${node.id}`);
		} else if (node.kind === 'fact') {
			recentItems.addNote(node.id, node.title);
			if (node.tags.some((t) => t.startsWith('day:'))) {
				goto(`/daily`);
			} else {
				goto(`/notes?note=${node.id}`);
			}
		} else if (node.kind === 'bookmark' || node.kind === 'reference') {
			goto(`/bookmarks?id=${node.id}`);
		} else if (node.kind === 'event') {
			goto(`/tasks?view=calendar`);
		} else {
			recentItems.addNote(node.id, node.title);
			goto(`/notes?note=${node.id}`);
		}
	}

	function navigateToRecent(item: RecentItem) {
		close();
		if (item.type === 'task') {
			goto(`/tasks?task=${item.id}`);
		} else if (item.type === 'note') {
			goto(`/notes?note=${item.id}`);
		} else if (item.type === 'search') {
			goto(`/search?q=${encodeURIComponent(item.title)}`);
		}
	}

	function kindBadge(kind: string): { label: string; cls: string } {
		return { label: kindLabel(kind), cls: kindBadgeClass(kind) };
	}
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
		role="presentation"
		transition:fade={{ duration: 100 }}
	>
		<div
			class="absolute inset-0 bg-black/50 backdrop-blur-sm"
			on:click={close}
			on:keydown={(e) => e.key === 'Escape' && close()}
			role="button"
			tabindex="-1"
			aria-label="Close quick search"
		></div>

		<div
			class="relative z-10 w-full max-w-xl rounded-2xl border border-slate-700 bg-slate-900/95 shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-label="Quick search"
			transition:scale={{ duration: 120, start: 0.97 }}
		>
			<div class="flex items-center gap-2 border-b border-slate-800 px-4 py-3">
				<svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
				</svg>
				<input
					bind:this={inputEl}
					class="flex-1 bg-transparent text-sm text-white placeholder-slate-500 outline-none"
					placeholder="Search notes, tasks, and more..."
					bind:value={query}
					on:input={onInput}
				/>
				<div class="flex items-center gap-2">
					<button
						class="rounded px-2 py-0.5 text-[10px] transition {searchMode === 'fulltext'
							? 'bg-slate-700 text-white'
							: 'text-slate-500 hover:text-white'}"
						on:click|stopPropagation={() => { searchMode = 'fulltext'; void doSearch(); }}
					>
						Text
					</button>
					<button
						class="rounded px-2 py-0.5 text-[10px] transition {searchMode === 'hybrid'
							? 'bg-slate-700 text-white'
							: 'text-slate-500 hover:text-white'}"
						on:click|stopPropagation={() => { searchMode = 'hybrid'; void doSearch(); }}
					>
						Semantic
					</button>
				</div>
				{#if loading}
					<div class="h-4 w-4 animate-spin rounded-full border-2 border-slate-600 border-t-sky-500"></div>
				{/if}
			</div>

			<div class="max-h-[50vh] overflow-y-auto p-2">
				{#if results.length === 0 && query.trim() && !loading}
					<div class="py-8 text-center text-sm text-slate-500">
						No results for "{query}"
					</div>
				{:else if showRecent && recentCombined.length > 0}
					<div class="pb-2">
						<p class="px-3 py-2 text-[10px] uppercase tracking-wider text-slate-600">Recent</p>
						{#each recentCombined as item, index (item.id)}
							<button
								class="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition {index === selectedIndex
									? 'bg-sky-500/10 text-white'
									: 'text-slate-300 hover:bg-slate-800/60'}"
								on:click={() => navigateToRecent(item)}
								on:mouseenter={() => { selectedIndex = index; }}
							>
								<span class={`shrink-0 rounded-full px-2 py-0.5 text-[9px] font-medium ${item.type === 'task' ? 'bg-violet-500/20 text-violet-300' : 'bg-sky-500/20 text-sky-300'}`}>
									{item.type === 'task' ? 'Task' : 'Note'}
								</span>
								<span class="truncate text-sm">{item.title || 'Untitled'}</span>
							</button>
						{/each}
					</div>
					{#if $recentSearches.length > 0}
						<div class="border-t border-slate-800 pt-2">
							<p class="px-3 py-2 text-[10px] uppercase tracking-wider text-slate-600">Recent Searches</p>
							{#each $recentSearches.slice(0, 3) as search}
								<button
									class="flex w-full items-center gap-2 rounded-xl px-3 py-2 text-left text-xs text-slate-400 transition hover:bg-slate-800/60 hover:text-white"
									on:click={() => { query = search.title; void doSearch(); }}
								>
									<svg class="h-3 w-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
									</svg>
									<span class="truncate">{search.title}</span>
								</button>
							{/each}
						</div>
					{/if}
				{:else if results.length === 0 && !query.trim()}
					<div class="py-6 text-center">
						<p class="text-sm text-slate-400">Start typing to search</p>
						<p class="mt-2 text-[10px] text-slate-600">
							<kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5">Tab</kbd> to toggle semantic search
						</p>
					</div>
				{:else}
					{#each results as result, index (result.node.id)}
						{@const badge = kindBadge(result.node.kind)}
						<button
							class="flex w-full items-start gap-3 rounded-xl px-3 py-2.5 text-left transition {index === selectedIndex
								? 'bg-sky-500/10 text-white'
								: 'text-slate-300 hover:bg-slate-800/60'}"
							on:click={() => navigateToResult(result)}
							on:mouseenter={() => { selectedIndex = index; }}
						>
							<span class={`mt-0.5 shrink-0 rounded-full px-2 py-0.5 text-[9px] font-medium ${badge.cls}`}>
								{badge.label}
							</span>
							<div class="min-w-0 flex-1">
								<div class="truncate text-sm font-medium">{result.node.title || 'Untitled'}</div>
								{#if result.node.content}
									<p class="mt-0.5 line-clamp-1 text-[11px] text-slate-500">
										{result.node.content.slice(0, 100)}
									</p>
								{/if}
							</div>
							{#if result.score != null}
								<span class="shrink-0 text-[10px] text-slate-600">
									{result.score.toFixed(2)}
								</span>
							{/if}
						</button>
					{/each}
				{/if}
			</div>

			<div class="flex items-center justify-between border-t border-slate-800 px-4 py-2 text-[10px] text-slate-600">
				<div>
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5">↑↓</kbd> navigate
					<kbd class="ml-2 rounded border border-slate-700 bg-slate-800 px-1 py-0.5">Enter</kbd> open
					<kbd class="ml-2 rounded border border-slate-700 bg-slate-800 px-1 py-0.5">Esc</kbd> close
				</div>
				<div>
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5">Cmd+/</kbd> toggle
				</div>
			</div>
		</div>
	</div>
{/if}
