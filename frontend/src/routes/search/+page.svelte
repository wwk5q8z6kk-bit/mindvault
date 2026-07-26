<script lang="ts">
	import {
		createSavedSearch,
		deleteSavedSearch,
		listSavedSearches,
		runSavedSearch,
		searchByMode,
		updateSavedSearch,
		type SavedSearch,
		type SearchMode
	} from '$lib/api/search';
	import {
		emptySearchHint,
		parseSearchMode,
		persistSearchMode,
		readStoredSearchMode,
		toApiSearchType
	} from '$lib/search/mode';
	import type { SearchResultDto, ProactiveInsight } from '$lib/api/types';
	import { assistAutocomplete } from '$lib/api/assist';
	import { getEmbeddingClusters } from '$lib/api/insights';
	import { getNeighbors, type GraphNeighbor } from '$lib/api/graph';
	import { pushToast } from '$lib/stores/toast';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { kindLabel, kindBadgeClass, ALL_NODE_KINDS } from '$lib/utils/kind-helpers';
	import { recentItems } from '$lib/stores/recent';
	import { createVirtualizer } from '@tanstack/svelte-virtual';
	import EmptyState from '$lib/components/EmptyState.svelte';

	type SortOption = 'relevance' | 'date_desc' | 'date_asc' | 'title';
	type ViewMode = 'results' | 'clusters';

	let query = '';
	let searchType: SearchMode = readStoredSearchMode();
	let selectedKinds: Set<string> = new Set();
	let showKindDropdown = false;
	let tagFilter: string = '';
	let sortBy: SortOption = 'relevance';
	let results: SearchResultDto[] = [];
	let sortedResults: SearchResultDto[] = [];
	let loading = false;
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let searchInput: HTMLInputElement | null = null;
	let savedSearches: SavedSearch[] = [];
	let selectedSavedSearchId: string | null = null;
	let savedSearchName = '';
	let savePending = false;
	let savedSearchLoading = false;
	let selectedIndex = -1;
	let resultsContainer: HTMLDivElement | null = null;
	let searchListParentRef: HTMLDivElement | null = null;

	// Autocomplete state
	let suggestions: string[] = [];
	let showSuggestions = false;
	let suggestionsLoading = false;
	let suggestionIndex = -1;
	let autocompleteTimer: ReturnType<typeof setTimeout> | null = null;
	let inputFocused = false;

	// Cluster view state
	let viewMode: ViewMode = 'results';
	let clusters: ProactiveInsight[] = [];
	let clustersLoading = false;

	// Related nodes state
	let relatedNodes: GraphNeighbor[] = [];
	let relatedLoading = false;
	let neighborTimer: ReturnType<typeof setTimeout> | null = null;

	const searchVirtualizer = createVirtualizer({
		get count() {
			return sortedResults.length;
		},
		getScrollElement: () => searchListParentRef,
		estimateSize: () => 80,
		overscan: 5
	});

	const sortOptions: { value: SortOption; label: string }[] = [
		{ value: 'relevance', label: 'Relevance' },
		{ value: 'date_desc', label: 'Newest first' },
		{ value: 'date_asc', label: 'Oldest first' },
		{ value: 'title', label: 'Title A-Z' }
	];

	// Sort and filter results reactively
	$: {
		let filtered = [...results];

		// Apply tag filter
		if (tagFilter.trim()) {
			const tag = tagFilter.trim().toLowerCase();
			filtered = filtered.filter((r) =>
				r.node.tags.some((t) => t.toLowerCase().includes(tag))
			);
		}

		// Apply sorting
		if (sortBy === 'date_desc') {
			filtered.sort((a, b) => new Date(b.node.temporal.updated_at).getTime() - new Date(a.node.temporal.updated_at).getTime());
		} else if (sortBy === 'date_asc') {
			filtered.sort((a, b) => new Date(a.node.temporal.updated_at).getTime() - new Date(b.node.temporal.updated_at).getTime());
		} else if (sortBy === 'title') {
			filtered.sort((a, b) => a.node.title.localeCompare(b.node.title));
		}
		// relevance = default order from API

		sortedResults = filtered;
		// Reset selection when results change
		if (selectedIndex >= sortedResults.length) {
			selectedIndex = sortedResults.length > 0 ? 0 : -1;
		}
	}

	// Fetch autocomplete suggestions (debounced)
	async function fetchSuggestions(text: string) {
		if (text.length < 2) {
			suggestions = [];
			showSuggestions = false;
			return;
		}
		suggestionsLoading = true;
		try {
			const resp = await assistAutocomplete({ text, limit: 5 });
			suggestions = resp.completions.slice(0, 5);
			showSuggestions = suggestions.length > 0 && inputFocused;
		} catch {
			suggestions = [];
			showSuggestions = false;
		} finally {
			suggestionsLoading = false;
		}
	}

	function onAutocompleteInput() {
		if (autocompleteTimer) clearTimeout(autocompleteTimer);
		suggestionIndex = -1;
		autocompleteTimer = setTimeout(() => {
			void fetchSuggestions(query);
		}, 200);
	}

	function selectSuggestion(suggestion: string) {
		query = suggestion;
		suggestions = [];
		showSuggestions = false;
		suggestionIndex = -1;
		void doSearch();
	}

	function closeSuggestions() {
		showSuggestions = false;
		suggestionIndex = -1;
	}

	// Fetch related nodes when selected result changes (debounced)
	$: {
		if (selectedIndex >= 0 && selectedIndex < sortedResults.length && viewMode === 'results') {
			const nodeId = sortedResults[selectedIndex].node.id;
			if (neighborTimer) clearTimeout(neighborTimer);
			neighborTimer = setTimeout(() => {
				void fetchRelatedNodes(nodeId);
			}, 500);
		} else {
			relatedNodes = [];
			relatedLoading = false;
		}
	}

	async function fetchRelatedNodes(nodeId: string) {
		relatedLoading = true;
		try {
			const resp = await getNeighbors(nodeId);
			relatedNodes = resp.neighbors;
		} catch {
			relatedNodes = [];
		} finally {
			relatedLoading = false;
		}
	}

	// Fetch clusters when switching to cluster view
	async function fetchClusters() {
		clustersLoading = true;
		try {
			clusters = await getEmbeddingClusters();
		} catch {
			pushToast('Failed to load clusters', 'danger');
			clusters = [];
		} finally {
			clustersLoading = false;
		}
	}

	function switchViewMode(mode: ViewMode) {
		viewMode = mode;
		if (mode === 'clusters' && clusters.length === 0) {
			void fetchClusters();
		}
	}

	function navigateToNodeById(nodeId: string, title: string) {
		recentItems.addNote(nodeId, title);
		goto(`/notes?note=${nodeId}`);
	}

	function kindBadge(kind: string): { label: string; color: string } {
		return { label: kindLabel(kind), color: kindBadgeClass(kind) };
	}

	function handleKeydown(event: KeyboardEvent) {
		if (showSuggestions && inputFocused) {
			if (event.key === 'ArrowDown') {
				event.preventDefault();
				suggestionIndex = Math.min(suggestionIndex + 1, suggestions.length - 1);
				return;
			}
			if (event.key === 'ArrowUp') {
				event.preventDefault();
				suggestionIndex = Math.max(suggestionIndex - 1, -1);
				return;
			}
			if (event.key === 'Enter' && suggestionIndex >= 0 && suggestions[suggestionIndex]) {
				event.preventDefault();
				selectSuggestion(suggestions[suggestionIndex]);
				return;
			}
			if (event.key === 'Escape') {
				closeSuggestions();
			}
		}

		// Close dropdown on Escape
		if (event.key === 'Escape' && showKindDropdown) {
			showKindDropdown = false;
			return;
		}

		if (viewMode !== 'results' || sortedResults.length === 0) return;

		if (event.key === 'ArrowDown') {
			event.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, sortedResults.length - 1);
			scrollToSelected();
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
			scrollToSelected();
		} else if (event.key === 'Enter' && selectedIndex >= 0) {
			event.preventDefault();
			navigateToResult(sortedResults[selectedIndex]);
		}
	}

	function handleClickOutside(event: MouseEvent) {
		if (showKindDropdown) {
			const target = event.target as HTMLElement;
			if (!target.closest('[data-kind-dropdown]')) {
				showKindDropdown = false;
			}
		}
	}

	function scrollToSelected() {
		if (!resultsContainer || selectedIndex < 0) return;
		const items = resultsContainer.querySelectorAll('[data-result-item]');
		if (items[selectedIndex]) {
			items[selectedIndex].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
		}
	}

	function navigateToResult(result: SearchResultDto) {
		const node = result.node;

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
		} else {
			recentItems.addNote(node.id, node.title);
			goto(`/notes?note=${node.id}`);
		}
	}

	function onQueryInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			void doSearch();
		}, 300);
	}

	function handleSearchInput() {
		onQueryInput();
		onAutocompleteInput();
	}

	function parseKindsFilter(): string[] {
		return selectedKinds.size === 0 ? [] : [...selectedKinds];
	}

	function toggleKind(kind: string) {
		const newSet = new Set(selectedKinds);
		if (newSet.has(kind)) {
			newSet.delete(kind);
		} else {
			newSet.add(kind);
		}
		selectedKinds = newSet;
		void doSearch();
	}

	function clearKindFilter() {
		selectedKinds = new Set();
		void doSearch();
	}

	$: kindFilterLabel = selectedKinds.size === 0
		? 'All kinds'
		: selectedKinds.size === 1
			? kindLabel([...selectedKinds][0])
			: `${selectedKinds.size} kinds`;

	function applySavedSearch(search: SavedSearch) {
		query = search.query;
		searchType = parseSearchMode(search.search_type) ?? 'fulltext';
		persistSearchMode(searchType);
		selectedKinds = new Set(search.kinds);
		selectedSavedSearchId = search.id;
		savedSearchName = search.name;
	}

	function setSearchType(mode: SearchMode) {
		searchType = mode;
		persistSearchMode(mode);
		void doSearch();
	}

	async function refreshSavedSearches() {
		savedSearchLoading = true;
		try {
			savedSearches = await listSavedSearches(200, 0);
			if (selectedSavedSearchId && !savedSearches.some((item) => item.id === selectedSavedSearchId)) {
				selectedSavedSearchId = null;
			}
		} catch {
			pushToast('Failed to load saved searches', 'warning');
		} finally {
			savedSearchLoading = false;
		}
	}

	function autoSavedSearchName(): string {
		const trimmed = query.trim();
		if (!trimmed) return 'New saved search';
		return trimmed.length > 60 ? `${trimmed.slice(0, 60)}…` : trimmed;
	}

	async function saveCurrentSearch() {
		const trimmed = query.trim();
		if (!trimmed) {
			pushToast('Enter a query before saving', 'warning');
			return;
		}
		const payload = {
			name: savedSearchName.trim() || autoSavedSearchName(),
			query: trimmed,
			search_type: toApiSearchType(searchType),
			limit: 50,
			kinds: parseKindsFilter()
		};
		savePending = true;
		try {
			if (selectedSavedSearchId) {
				const updated = await updateSavedSearch(selectedSavedSearchId, payload);
				selectedSavedSearchId = updated.id;
				savedSearchName = updated.name;
				pushToast('Saved search updated', 'success');
			} else {
				const created = await createSavedSearch(payload);
				selectedSavedSearchId = created.id;
				savedSearchName = created.name;
				pushToast('Saved search created', 'success');
			}
			await refreshSavedSearches();
		} catch {
			pushToast('Failed to save search', 'danger');
		} finally {
			savePending = false;
		}
	}

	async function runSavedSearchById(item: SavedSearch) {
		loading = true;
		try {
			const response = await runSavedSearch(item.id);
			results = response.results;
			applySavedSearch(response.saved_search);
		} catch {
			pushToast('Failed to run saved search', 'danger');
		} finally {
			loading = false;
		}
	}

	async function removeSavedSearch(item: SavedSearch) {
		if (!confirm(`Delete saved search "${item.name}"?`)) return;
		try {
			await deleteSavedSearch(item.id);
			pushToast('Saved search deleted', 'success');
			if (selectedSavedSearchId === item.id) {
				selectedSavedSearchId = null;
				savedSearchName = '';
			}
			await refreshSavedSearches();
		} catch {
			pushToast('Failed to delete saved search', 'danger');
		}
	}

	async function doSearch() {
		const q = query.trim();
		if (!q) {
			results = [];
			selectedIndex = -1;
			relatedNodes = [];
			return;
		}
		loading = true;
		selectedIndex = -1;
		try {
			let raw = await searchByMode(q, searchType, 50);
			// Filter by selected kinds if any are selected
			if (selectedKinds.size > 0) {
				raw = raw.filter((r) => selectedKinds.has(r.node.kind));
			}
			results = raw;
			// Auto-select first result for keyboard nav
			if (results.length > 0) {
				selectedIndex = 0;
			}
		} catch {
			pushToast('Search failed', 'danger');
			results = [];
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		const urlMode = parseSearchMode($page.url.searchParams.get('mode'));
		if (urlMode) {
			searchType = urlMode;
			persistSearchMode(urlMode);
		}
		const urlQuery = $page.url.searchParams.get('q');
		if (urlQuery) {
			query = urlQuery;
			void doSearch();
		}
		void refreshSavedSearches();
	});
</script>

<svelte:window on:keydown={handleKeydown} on:click={handleClickOutside} />

<div class="grid gap-6 lg:grid-cols-12">
	<section class="space-y-6 lg:col-span-8">
		<div class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-6">
			<div class="mb-4 flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Search</h2>
					<p class="text-xs text-[rgb(var(--mv-muted))]">Use <kbd class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1 text-[10px]">↑</kbd><kbd class="ml-0.5 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1 text-[10px]">↓</kbd> to navigate, <kbd class="ml-0.5 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1 text-[10px]">Enter</kbd> to open.</p>
				</div>
				<a
					href="/search/saved"
					class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
				>
					Manage Saved Searches
				</a>
			</div>

			<div class="flex gap-3">
				<div class="relative flex-1">
					<input
						type="text"
						class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-4 py-2 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/60 outline-none focus:border-sky-500"
						placeholder="Search..."
						bind:value={query}
						bind:this={searchInput}
						on:input={handleSearchInput}
						on:focus={() => {
							inputFocused = true;
							showSuggestions = suggestions.length > 0;
						}}
						on:blur={() => {
							inputFocused = false;
							setTimeout(closeSuggestions, 120);
						}}
					/>
					{#if showSuggestions}
						<div class="absolute z-20 mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] p-1 shadow-xl">
							{#if suggestionsLoading}
								<div class="px-2 py-1.5 text-xs text-[rgb(var(--mv-muted))]/70">Loading suggestions...</div>
							{:else}
								{#each suggestions as suggestion, idx (suggestion)}
									<button
										class={`block w-full rounded-lg px-2 py-1.5 text-left text-xs transition ${
											suggestionIndex === idx
												? 'bg-sky-500/20 text-sky-200'
												: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel))]'
										}`}
										on:mousedown|preventDefault
										on:click={() => selectSuggestion(suggestion)}
									>
										{suggestion}
									</button>
								{/each}
							{/if}
						</div>
					{/if}
				</div>
				<button
					class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-[rgb(var(--mv-text))] hover:bg-sky-400"
					on:click={doSearch}
				>
					Search
				</button>
			</div>

			<div class="mt-3 flex flex-wrap items-center gap-3">
				<div class="flex rounded-lg border border-[rgb(var(--mv-border))] text-[10px]" role="group" aria-label="Search mode">
					<button
						class={`px-3 py-1 transition ${searchType === 'hybrid' ? 'bg-[rgb(var(--mv-panel-strong))]/80 text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
						aria-pressed={searchType === 'hybrid'}
						title="Combine keyword and semantic matching (recommended)"
						on:click={() => setSearchType('hybrid')}
					>
						Hybrid
					</button>
					<button
						class={`px-3 py-1 transition ${searchType === 'fulltext' ? 'bg-[rgb(var(--mv-panel-strong))]/80 text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
						aria-pressed={searchType === 'fulltext'}
						title="Exact keyword / BM25 matching"
						on:click={() => setSearchType('fulltext')}
					>
						Fulltext
					</button>
					<button
						class={`px-3 py-1 transition ${searchType === 'semantic' ? 'bg-[rgb(var(--mv-panel-strong))]/80 text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
						aria-pressed={searchType === 'semantic'}
						title="Vector / meaning-based matching only"
						on:click={() => setSearchType('semantic')}
					>
						Semantic
					</button>
				</div>

				<div class="flex rounded-lg border border-[rgb(var(--mv-border))] text-[10px]">
					<button
						class={`px-3 py-1 transition ${viewMode === 'results' ? 'bg-[rgb(var(--mv-panel-strong))]/80 text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
						on:click={() => switchViewMode('results')}
					>
						Results
					</button>
					<button
						class={`px-3 py-1 transition ${viewMode === 'clusters' ? 'bg-[rgb(var(--mv-panel-strong))]/80 text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
						on:click={() => switchViewMode('clusters')}
					>
						Clusters
					</button>
				</div>

				<div class="relative" data-kind-dropdown>
					<div class="flex items-center gap-1 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] pr-1">
						<button
							class="flex items-center gap-1.5 px-2 py-1 text-xs text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-panel-strong))]/80"
							on:click={() => {
								showKindDropdown = !showKindDropdown;
							}}
							aria-label="Filter by kind"
							aria-expanded={showKindDropdown}
						>
							<span>{kindFilterLabel}</span>
							<svg class="h-3 w-3 text-[rgb(var(--mv-muted))]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
							</svg>
						</button>
						{#if selectedKinds.size > 0}
							<button
								class="rounded-full bg-[rgb(var(--mv-panel-strong))]/60 px-1 text-[9px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/40"
								on:click={clearKindFilter}
								aria-label="Clear kind filter"
							>
								&times;
							</button>
						{/if}
					</div>
					{#if showKindDropdown}
						<div
							class="absolute left-0 top-full z-20 mt-1 max-h-64 w-48 overflow-y-auto rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] p-2 shadow-xl"
							role="listbox"
						>
							{#each ALL_NODE_KINDS as kind}
								<label
									class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/80"
								>
									<input
										type="checkbox"
										class="h-3.5 w-3.5 rounded border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/80 text-sky-500 focus:ring-sky-500 focus:ring-offset-0"
										checked={selectedKinds.has(kind)}
										on:change={() => toggleKind(kind)}
									/>
									<span class={`rounded-full px-1.5 py-0.5 text-[9px] ${kindBadgeClass(kind)}`}>
										{kindLabel(kind)}
									</span>
								</label>
							{/each}
							<div class="mt-2 border-t border-[rgb(var(--mv-border))] pt-2">
								<button
									class="w-full rounded-lg px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/80 hover:text-[rgb(var(--mv-text))]"
									on:click={() => { showKindDropdown = false; }}
								>
									Done
								</button>
							</div>
						</div>
					{/if}
				</div>

				<select
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
					bind:value={sortBy}
					aria-label="Sort by"
				>
					{#each sortOptions as opt (opt.value)}
						<option value={opt.value}>{opt.label}</option>
					{/each}
				</select>

				<input
					type="text"
					class="w-28 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-2 py-1 text-xs text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/60 outline-none focus:border-sky-500"
					placeholder="Filter tag..."
					bind:value={tagFilter}
				/>

				{#if loading}
					<span class="text-[10px] text-amber-300">Searching...</span>
				{/if}
			</div>

			<div class="mt-4 grid gap-2 md:grid-cols-[1fr_auto_auto]">
				<input
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/60 outline-none focus:border-sky-500"
					placeholder="Saved search name"
					bind:value={savedSearchName}
				/>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))] disabled:opacity-60"
					disabled={savePending}
					on:click={saveCurrentSearch}
				>
					{#if savePending}
						Saving...
					{:else if selectedSavedSearchId}
						Update Saved
					{:else}
						Save Search
					{/if}
				</button>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))] disabled:opacity-60"
					disabled={!selectedSavedSearchId}
					on:click={() => {
						selectedSavedSearchId = null;
						savedSearchName = '';
					}}
				>
					Clear
				</button>
			</div>
		</div>

		{#if viewMode === 'clusters'}
			<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/30 p-4">
				<div class="mb-3 flex items-center justify-between">
					<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Knowledge Clusters</h3>
					<button
						class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
						on:click={fetchClusters}
					>
						Refresh
					</button>
				</div>
				{#if clustersLoading}
					<p class="text-xs text-[rgb(var(--mv-muted))]/70">Analyzing cluster structure…</p>
				{:else if clusters.length === 0}
					<p class="text-xs text-[rgb(var(--mv-muted))]/70">No clusters found yet.</p>
				{:else}
					<div class="space-y-2">
						{#each clusters as cluster (cluster.id)}
							<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-3">
								<div class="flex items-start gap-2">
									<div class="rounded-full bg-sky-500/20 px-2 py-0.5 text-[10px] text-sky-300">
										{Math.round(cluster.importance * 100)}%
									</div>
									<div class="min-w-0 flex-1">
										<h4 class="text-sm font-medium text-[rgb(var(--mv-text))]">{cluster.title}</h4>
										<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">{cluster.content}</p>
									</div>
								</div>
								{#if cluster.related_node_ids.length > 0}
									<div class="mt-2 flex flex-wrap gap-1">
										{#each cluster.related_node_ids.slice(0, 6) as nodeId}
											<button
												class="rounded border border-[rgb(var(--mv-border))] px-1.5 py-0.5 text-[10px] text-[rgb(var(--mv-muted))] hover:border-sky-500/40 hover:text-sky-200"
												on:click={() => navigateToNodeById(nodeId, `Node ${nodeId.slice(0, 8)}`)}
											>
												{nodeId.slice(0, 8)}
											</button>
										{/each}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{:else if sortedResults.length > 0}
			<div bind:this={resultsContainer}>
				<p class="mb-3 text-xs text-[rgb(var(--mv-muted))]">
					{sortedResults.length} result{sortedResults.length !== 1 ? 's' : ''}{tagFilter.trim() ? ` (filtered by tag "${tagFilter}")` : ''}
				</p>
				<div bind:this={searchListParentRef} style="max-height: 70vh; overflow-y: auto;">
					<div style="height: {$searchVirtualizer.getTotalSize()}px; width: 100%; position: relative;">
						{#each $searchVirtualizer.getVirtualItems() as row (row.key)}
							{@const result = sortedResults[row.index]}
							{@const idx = row.index}
							{@const badge = kindBadge(result.node.kind)}
							{@const isSelected = idx === selectedIndex}
							<div
								style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({row.start}px);"
							>
								<button
									data-result-item
									class={`w-full rounded-xl border p-4 mb-3 text-left transition ${
										isSelected
											? 'border-sky-500 bg-sky-500/10 ring-1 ring-sky-500/30'
											: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/60 hover:border-[rgb(var(--mv-border))] hover:bg-[rgb(var(--mv-panel))]/80'
									}`}
									on:click={() => navigateToResult(result)}
									on:mouseenter={() => (selectedIndex = idx)}
								>
									<div class="flex items-center gap-2">
										<span class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${badge.color}`}>
											{badge.label}
										</span>
										<h3 class="text-sm font-medium text-[rgb(var(--mv-text))]">{result.node.title}</h3>
										{#if result.score != null}
											<span class="ml-auto rounded-full bg-[rgb(var(--mv-panel-strong))] px-2 py-0.5 text-[10px] text-[rgb(var(--mv-muted))]">
												{result.score.toFixed(2)}
											</span>
										{/if}
									</div>
									{#if result.node.content}
										<p class="mt-1 line-clamp-2 text-xs text-[rgb(var(--mv-muted))]">
											{result.node.content.slice(0, 200)}
										</p>
									{/if}
									{#if result.node.tags.length > 0}
										<div class="mt-2 flex flex-wrap gap-1">
											{#each result.node.tags.slice(0, 5) as tag}
												<span
													class="cursor-pointer rounded bg-[rgb(var(--mv-panel-strong))] px-1.5 py-0.5 text-[10px] text-[rgb(var(--mv-muted))]/60 hover:bg-[rgb(var(--mv-panel-strong))]/80 hover:text-[rgb(var(--mv-muted))]"
													role="button"
													tabindex="0"
													on:click|stopPropagation={() => (tagFilter = tag)}
													on:keydown|stopPropagation={(e) => e.key === 'Enter' && (tagFilter = tag)}
												>
													{tag}
												</span>
											{/each}
										</div>
									{/if}
								</button>
							</div>
						{/each}
					</div>
				</div>
			</div>
		{:else if query.trim() && !loading}
			<EmptyState
				icon="search"
				tone="slate"
				title={`No results for "${query}"`}
				description={emptySearchHint(searchType)}
			/>
		{:else if !query.trim() && !loading}
			<EmptyState
				icon="search"
				tone="sky"
				title="Search your vault"
				description="Hybrid search finds both exact keywords and related meaning across your vault"
			>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/40 px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40"
					on:click={() => { query = 'project ideas'; void doSearch(); }}
				>
					"project ideas"
				</button>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/40 px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40"
					on:click={() => { query = 'meeting notes'; void doSearch(); }}
				>
					"meeting notes"
				</button>
			</EmptyState>
		{/if}
	</section>

	<aside class="lg:col-span-4">
		<div class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-5">
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Saved Searches</h3>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))] disabled:opacity-60"
					disabled={savedSearchLoading}
					on:click={refreshSavedSearches}
				>
					Refresh
				</button>
			</div>
			<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">
				Save frequent queries and re-run them with one click.
			</p>

			<div class="mt-3 space-y-2">
				{#if savedSearchLoading}
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">Loading saved searches…</p>
				{:else if savedSearches.length === 0}
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">No saved searches yet.</p>
				{:else}
					{#each savedSearches as item (item.id)}
						<div
							class={`rounded-lg border p-3 ${
								selectedSavedSearchId === item.id ? 'border-sky-500/50 bg-sky-500/10' : 'border-[rgb(var(--mv-border))]'
							}`}
						>
							<div class="text-xs font-medium text-[rgb(var(--mv-text))]">{item.name}</div>
							<div class="mt-1 line-clamp-2 text-[10px] text-[rgb(var(--mv-muted))]/60">{item.query}</div>
							<div class="mt-2 flex gap-2">
								<button
									class="rounded-lg bg-[rgb(var(--mv-panel-strong))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/80"
									on:click={() => void runSavedSearchById(item)}
								>
									Run
								</button>
								<button
									class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
									on:click={() => applySavedSearch(item)}
								>
									Load
								</button>
								<button
									class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10"
									on:click={() => void removeSavedSearch(item)}
								>
									Delete
								</button>
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</div>

		<div class="mt-4 rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-5">
			<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Related Nodes</h3>
			<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">
				Neighbors of the currently selected result.
			</p>
			<div class="mt-3 space-y-2">
				{#if viewMode !== 'results'}
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">Available in results view.</p>
				{:else if relatedLoading}
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">Loading related nodes…</p>
				{:else if relatedNodes.length === 0}
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">Select a search result to explore neighbors.</p>
				{:else}
					{#each relatedNodes.slice(0, 10) as neighbor (neighbor.node.id + neighbor.relationship_kind)}
						<a
							href={`/notes?note=${neighbor.node.id}`}
							class="block rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2 text-xs transition hover:border-sky-500/30"
						>
							<div class="flex items-center gap-2">
								<span class={`rounded-full px-1.5 py-0.5 text-[9px] ${kindBadgeClass(neighbor.node.kind)}`}>
									{kindLabel(neighbor.node.kind)}
								</span>
								<span class="truncate text-[rgb(var(--mv-text))]">{neighbor.node.title || 'Untitled'}</span>
							</div>
							<div class="mt-1 text-[10px] text-[rgb(var(--mv-muted))]/70">
								{neighbor.direction} via {neighbor.relationship_kind}
							</div>
						</a>
					{/each}
				{/if}
			</div>
		</div>
	</aside>
</div>
