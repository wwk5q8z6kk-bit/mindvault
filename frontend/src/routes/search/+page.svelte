<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { assistAutocomplete } from '$lib/api/assist';
	import { getNeighbors, type GraphNeighbor } from '$lib/api/graph';
	import { getEmbeddingClusters } from '$lib/api/insights';
	import { createSavedSearch, searchByMode, type SearchMode } from '$lib/api/search';
	import type { ProactiveInsight, SearchResultDto } from '$lib/api/types';
	import ContextBoundary from '$lib/components/ContextBoundary.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import SearchResultCard from '$lib/components/SearchResultCard.svelte';
	import {
		emptySearchHint,
		parseSearchMode,
		persistSearchMode,
		readStoredSearchMode,
		toApiSearchType
	} from '$lib/search/mode';
	import { graphDestination, resultDestination } from '$lib/search/results';
	import { recentItems } from '$lib/stores/recent';
	import { pushToast } from '$lib/stores/toast';
	import { ALL_NODE_KINDS, kindBadgeClass, kindLabel } from '$lib/utils/kind-helpers';
	import {
		BookmarkPlus,
		ChevronDown,
		ListFilter,
		Network,
		RefreshCw,
		Search as SearchIcon,
		X
	} from '@lucide/svelte';
	import { onDestroy, onMount, tick } from 'svelte';

	type SortOption = 'relevance' | 'date_desc' | 'date_asc' | 'title';
	type ViewMode = 'results' | 'clusters';

	const searchModeOptions: { value: SearchMode; label: string; title: string }[] = [
		{ value: 'hybrid', label: 'Best match', title: 'Keywords and meaning (recommended)' },
		{ value: 'fulltext', label: 'Keywords', title: 'Exact words and phrases' },
		{ value: 'semantic', label: 'Meaning', title: 'Meaning-based matching' }
	];

	const sortOptions: { value: SortOption; label: string }[] = [
		{ value: 'relevance', label: 'Relevance' },
		{ value: 'date_desc', label: 'Newest first' },
		{ value: 'date_asc', label: 'Oldest first' },
		{ value: 'title', label: 'Title A–Z' }
	];

	let query = '';
	let searchType: SearchMode = readStoredSearchMode();
	let selectedKinds = new Set<string>();
	let tagFilter = '';
	let sortBy: SortOption = 'relevance';
	let results: SearchResultDto[] = [];
	let sortedResults: SearchResultDto[] = [];
	let loading = false;
	let searchError = '';
	let searchRequestId = 0;
	let searchInput: HTMLInputElement | null = null;
	let selectedIndex = -1;
	let showRefine = false;
	let showKindDropdown = false;
	let viewMode: ViewMode = 'results';

	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let suggestions: string[] = [];
	let showSuggestions = false;
	let suggestionsLoading = false;
	let suggestionIndex = -1;
	let autocompleteTimer: ReturnType<typeof setTimeout> | null = null;
	let inputFocused = false;

	let showSavePanel = false;
	let savedSearchName = '';
	let savePending = false;

	let clusters: ProactiveInsight[] = [];
	let clustersLoading = false;
	let clustersError = '';

	let relatedNodes: GraphNeighbor[] = [];
	let relatedLoading = false;
	let relatedError = '';
	let relatedRequestId = 0;
	let neighborTimer: ReturnType<typeof setTimeout> | null = null;

	$: {
		let filtered = [...results];
		if (tagFilter.trim()) {
			const tag = tagFilter.trim().toLowerCase();
			filtered = filtered.filter((result) =>
				result.node.tags.some((item) => item.toLowerCase().includes(tag))
			);
		}

		if (sortBy === 'date_desc') {
			filtered.sort(
				(a, b) =>
					new Date(b.node.temporal.updated_at).getTime() -
					new Date(a.node.temporal.updated_at).getTime()
			);
		} else if (sortBy === 'date_asc') {
			filtered.sort(
				(a, b) =>
					new Date(a.node.temporal.updated_at).getTime() -
					new Date(b.node.temporal.updated_at).getTime()
			);
		} else if (sortBy === 'title') {
			filtered.sort((a, b) => a.node.title.localeCompare(b.node.title));
		}

		sortedResults = filtered;
		if (selectedIndex >= sortedResults.length) {
			selectedIndex = sortedResults.length > 0 ? 0 : -1;
		}
	}

	$: kindFilterLabel =
		selectedKinds.size === 0
			? 'All kinds'
			: selectedKinds.size === 1
				? kindLabel([...selectedKinds][0])
				: `${selectedKinds.size} kinds`;

	$: activeRefinementCount =
		selectedKinds.size +
		(tagFilter.trim() ? 1 : 0) +
		(sortBy !== 'relevance' ? 1 : 0) +
		(searchType !== 'hybrid' ? 1 : 0);

	$: selectedResult =
		selectedIndex >= 0 && selectedIndex < sortedResults.length
			? sortedResults[selectedIndex]
			: null;

	$: {
		if (selectedResult && viewMode === 'results') {
			const nodeId = selectedResult.node.id;
			if (neighborTimer) clearTimeout(neighborTimer);
			neighborTimer = setTimeout(() => void fetchRelatedNodes(nodeId), 350);
		} else {
			relatedRequestId += 1;
			relatedNodes = [];
			relatedLoading = false;
			relatedError = '';
		}
	}

	async function fetchSuggestions(text: string) {
		if (text.trim().length < 2) {
			suggestions = [];
			showSuggestions = false;
			return;
		}

		suggestionsLoading = true;
		try {
			const response = await assistAutocomplete({ text, limit: 5 });
			suggestions = response.completions.slice(0, 5);
			showSuggestions = suggestions.length > 0 && inputFocused;
		} catch {
			suggestions = [];
			showSuggestions = false;
		} finally {
			suggestionsLoading = false;
		}
	}

	function handleSearchInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => void doSearch(), 300);
		if (autocompleteTimer) clearTimeout(autocompleteTimer);
		suggestionIndex = -1;
		autocompleteTimer = setTimeout(() => void fetchSuggestions(query), 200);
	}

	function selectSuggestion(suggestion: string) {
		query = suggestion;
		closeSuggestions();
		void doSearch();
	}

	function closeSuggestions() {
		showSuggestions = false;
		suggestionIndex = -1;
	}

	function handleSearchKeydown(event: KeyboardEvent) {
		if (showSuggestions) {
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
				return;
			}
		}

		if (event.key === 'ArrowDown' && sortedResults.length > 0) {
			event.preventDefault();
			void focusResult(selectedIndex >= 0 ? selectedIndex : 0);
		}
	}

	async function focusResult(index: number) {
		selectedIndex = Math.max(0, Math.min(index, sortedResults.length - 1));
		await tick();
		document.querySelector<HTMLButtonElement>(`[data-result-index="${selectedIndex}"]`)?.focus();
	}

	function moveResultFocus(index: number, direction: number) {
		if (index === 0 && direction < 0) {
			searchInput?.focus();
			return;
		}
		void focusResult(index + direction);
	}

	function handleClickOutside(event: MouseEvent) {
		if (!showKindDropdown) return;
		if (!(event.target as HTMLElement).closest('[data-kind-dropdown]')) showKindDropdown = false;
	}

	async function fetchRelatedNodes(nodeId: string) {
		const requestId = ++relatedRequestId;
		relatedLoading = true;
		relatedError = '';
		try {
			const response = await getNeighbors(nodeId);
			if (requestId === relatedRequestId) relatedNodes = response.neighbors;
		} catch {
			if (requestId === relatedRequestId) {
				relatedNodes = [];
				relatedError = 'Connections could not be loaded.';
			}
		} finally {
			if (requestId === relatedRequestId) relatedLoading = false;
		}
	}

	async function fetchClusters() {
		clustersLoading = true;
		clustersError = '';
		try {
			clusters = await getEmbeddingClusters();
		} catch {
			clusters = [];
			clustersError = 'Knowledge clusters could not be loaded.';
		} finally {
			clustersLoading = false;
		}
	}

	function switchViewMode(mode: ViewMode) {
		viewMode = mode;
		if (mode === 'clusters' && clusters.length === 0) void fetchClusters();
	}

	function navigateToResult(result: SearchResultDto) {
		if (result.node.kind === 'task') recentItems.addTask(result.node.id, result.node.title);
		else recentItems.addNote(result.node.id, result.node.title);
		void goto(resultDestination(result.node));
	}

	function navigateToGraph(nodeId: string) {
		void goto(graphDestination(nodeId));
	}

	function applyTagFilter(tag: string) {
		tagFilter = tag;
		showRefine = true;
	}

	function clearQuery() {
		query = '';
		searchRequestId += 1;
		results = [];
		selectedIndex = -1;
		searchError = '';
		loading = false;
		closeSuggestions();
		searchInput?.focus();
	}

	function clearRefinements() {
		selectedKinds = new Set();
		tagFilter = '';
		sortBy = 'relevance';
		searchType = 'hybrid';
		persistSearchMode(searchType);
		showKindDropdown = false;
		void doSearch();
	}

	function toggleKind(kind: string) {
		const next = new Set(selectedKinds);
		if (next.has(kind)) next.delete(kind);
		else next.add(kind);
		selectedKinds = next;
		void doSearch();
	}

	function clearKindFilter() {
		selectedKinds = new Set();
		void doSearch();
	}

	function setSearchType(mode: SearchMode) {
		searchType = mode;
		persistSearchMode(mode);
		void doSearch();
	}

	function autoSavedSearchName(): string {
		const trimmed = query.trim();
		if (!trimmed) return 'New saved search';
		return trimmed.length > 60 ? `${trimmed.slice(0, 60)}…` : trimmed;
	}

	async function saveCurrentSearch() {
		const trimmed = query.trim();
		if (!trimmed) return;
		savePending = true;
		try {
			await createSavedSearch({
				name: savedSearchName.trim() || autoSavedSearchName(),
				query: trimmed,
				search_type: toApiSearchType(searchType),
				limit: 50,
				kinds: [...selectedKinds]
			});
			showSavePanel = false;
			pushToast('Saved search created', 'success');
		} catch {
			pushToast('Failed to save search', 'danger');
		} finally {
			savePending = false;
		}
	}

	async function doSearch() {
		const trimmed = query.trim();
		if (!trimmed) {
			searchRequestId += 1;
			results = [];
			selectedIndex = -1;
			searchError = '';
			loading = false;
			return;
		}

		const requestId = ++searchRequestId;
		loading = true;
		searchError = '';
		selectedIndex = -1;
		try {
			let response = await searchByMode(trimmed, searchType, 50);
			if (requestId !== searchRequestId) return;
			if (selectedKinds.size > 0) {
				response = response.filter((result) => selectedKinds.has(result.node.kind));
			}
			results = response;
			selectedIndex = results.length > 0 ? 0 : -1;
		} catch {
			if (requestId === searchRequestId) {
				results = [];
				searchError = 'We couldn’t search your vault. Check the connection and try again.';
			}
		} finally {
			if (requestId === searchRequestId) loading = false;
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
	});

	onDestroy(() => {
		if (debounceTimer) clearTimeout(debounceTimer);
		if (autocompleteTimer) clearTimeout(autocompleteTimer);
		if (neighborTimer) clearTimeout(neighborTimer);
	});
</script>

<svelte:window on:click={handleClickOutside} />

<div class="grid gap-6 xl:grid-cols-12">
	<section class="space-y-5 xl:col-span-8">
		<div
			class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-5 sm:p-6"
		>
			<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
				<div class="min-w-0">
					<ContextBoundary compact detail="Private retrieval" />
					<h1 class="mt-4 text-xl font-semibold tracking-tight text-[rgb(var(--mv-text))]">
						Find knowledge
					</h1>
					<p class="mt-1 max-w-xl text-xs leading-5 text-[rgb(var(--mv-muted))]">
						Search your Personal Vault by exact words or meaning, then continue in the source or its
						connections.
					</p>
				</div>
				<a
					href="/search/saved"
					class="shrink-0 rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] outline-none transition hover:bg-[rgb(var(--mv-panel-strong))] focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
				>
					Manage saved searches
				</a>
			</div>

			<form
				class="mt-5 flex flex-col gap-3 sm:flex-row"
				on:submit|preventDefault={() => void doSearch()}
			>
				<div class="relative min-w-0 flex-1">
					<SearchIcon
						class="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-[rgb(var(--mv-muted))]/65"
						size={17}
						strokeWidth={1.8}
						aria-hidden="true"
					/>
					<input
						id="vault-search"
						type="search"
						class="h-11 w-full rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] py-2 pl-10 pr-10 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/60 outline-none transition focus:border-sky-500 focus:ring-2 focus:ring-sky-500/15"
						placeholder="Search notes, tasks, people, decisions…"
						aria-label="Search your Personal Vault"
						role="combobox"
						aria-autocomplete="list"
						aria-expanded={showSuggestions}
						aria-controls="search-suggestions"
						aria-activedescendant={suggestionIndex >= 0
							? `search-suggestion-${suggestionIndex}`
							: undefined}
						bind:value={query}
						bind:this={searchInput}
						on:input={handleSearchInput}
						on:keydown={handleSearchKeydown}
						on:focus={() => {
							inputFocused = true;
							showSuggestions = suggestions.length > 0;
						}}
						on:blur={() => {
							inputFocused = false;
							setTimeout(closeSuggestions, 120);
						}}
					/>
					{#if query}
						<button
							type="button"
							class="absolute right-2.5 top-1/2 grid h-7 w-7 -translate-y-1/2 place-items-center rounded-lg text-[rgb(var(--mv-muted))] outline-none transition hover:bg-[rgb(var(--mv-panel))] hover:text-[rgb(var(--mv-text))] focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
							on:click={clearQuery}
							aria-label="Clear search"
						>
							<X size={15} strokeWidth={1.8} aria-hidden="true" />
						</button>
					{/if}

					{#if showSuggestions}
						<div
							id="search-suggestions"
							class="absolute z-30 mt-1.5 w-full rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] p-1.5 shadow-xl"
							role="listbox"
							aria-label="Search suggestions"
						>
							{#if suggestionsLoading}
								<div class="px-3 py-2 text-xs text-[rgb(var(--mv-muted))]/70" role="status">
									Finding suggestions…
								</div>
							{:else}
								{#each suggestions as suggestion, index (suggestion)}
									<button
										id={`search-suggestion-${index}`}
										type="button"
										class={`block w-full rounded-lg px-3 py-2 text-left text-xs outline-none transition ${suggestionIndex === index ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel))]'}`}
										role="option"
										aria-selected={suggestionIndex === index}
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
					type="submit"
					class="h-11 rounded-xl bg-sky-500 px-5 text-xs font-semibold text-slate-950 outline-none transition hover:bg-sky-400 focus-visible:ring-2 focus-visible:ring-sky-300 disabled:cursor-not-allowed disabled:opacity-50"
					disabled={!query.trim() || loading}
				>
					{loading ? 'Searching…' : 'Search'}
				</button>
			</form>

			<div class="mt-3 flex flex-wrap items-center justify-between gap-3">
				<div class="flex flex-wrap items-center gap-2">
					<button
						type="button"
						class={`inline-flex items-center gap-2 rounded-lg border px-3 py-1.5 text-xs outline-none transition focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70 ${showRefine || activeRefinementCount > 0 ? 'border-sky-500/40 bg-sky-500/10 text-sky-200' : 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}`}
						on:click={() => (showRefine = !showRefine)}
						aria-expanded={showRefine}
						aria-controls="search-refinements"
					>
						<ListFilter size={14} strokeWidth={1.8} aria-hidden="true" />
						Refine
						{#if activeRefinementCount > 0}<span
								class="rounded-full bg-sky-400/20 px-1.5 py-0.5 text-[10px]"
								>{activeRefinementCount}</span
							>{/if}
					</button>

					<div
						class="flex rounded-lg border border-[rgb(var(--mv-border))] p-0.5 text-[10px]"
						role="group"
						aria-label="Search view"
					>
						<button
							type="button"
							class={`rounded-md px-3 py-1 transition ${viewMode === 'results' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
							on:click={() => switchViewMode('results')}
							aria-pressed={viewMode === 'results'}>Results</button
						>
						<button
							type="button"
							class={`rounded-md px-3 py-1 transition ${viewMode === 'clusters' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
							on:click={() => switchViewMode('clusters')}
							aria-pressed={viewMode === 'clusters'}>Clusters</button
						>
					</div>
				</div>

				{#if query.trim()}
					<button
						type="button"
						class="inline-flex items-center gap-2 rounded-lg px-2 py-1.5 text-xs text-[rgb(var(--mv-muted))] outline-none transition hover:bg-[rgb(var(--mv-panel-strong))] hover:text-[rgb(var(--mv-text))] focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
						on:click={() => {
							showSavePanel = !showSavePanel;
							if (!savedSearchName) savedSearchName = autoSavedSearchName();
						}}
						aria-expanded={showSavePanel}
						aria-controls="save-search-panel"
					>
						<BookmarkPlus size={14} strokeWidth={1.8} aria-hidden="true" />
						Save this search
					</button>
				{/if}
			</div>

			{#if showRefine}
				<div
					id="search-refinements"
					class="mt-4 rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/35 p-4"
				>
					<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
						<fieldset>
							<legend
								class="mb-1.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-[rgb(var(--mv-muted))]/75"
								>Match by</legend
							>
							<div class="flex rounded-lg border border-[rgb(var(--mv-border))] p-0.5 text-[10px]">
								{#each searchModeOptions as option (option.value)}
									<button
										type="button"
										class={`flex-1 rounded-md px-2 py-1.5 transition ${searchType === option.value ? 'bg-[rgb(var(--mv-panel))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
										aria-pressed={searchType === option.value}
										title={option.title}
										on:click={() => setSearchType(option.value)}
									>
										{option.label}
									</button>
								{/each}
							</div>
						</fieldset>

						<div class="relative" data-kind-dropdown>
							<span
								class="mb-1.5 block text-[10px] font-semibold uppercase tracking-[0.12em] text-[rgb(var(--mv-muted))]/75"
								>Type</span
							>
							<div
								class="flex h-8 items-center rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/35"
							>
								<button
									id="kind-filter-button"
									type="button"
									class="flex min-w-0 flex-1 items-center justify-between gap-2 px-2.5 text-xs text-[rgb(var(--mv-text))]"
									on:click={() => (showKindDropdown = !showKindDropdown)}
									aria-expanded={showKindDropdown}
								>
									<span class="truncate">{kindFilterLabel}</span><ChevronDown
										size={13}
										strokeWidth={1.8}
										aria-hidden="true"
									/>
								</button>
								{#if selectedKinds.size > 0}
									<button
										type="button"
										class="mr-1 rounded p-1 text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]"
										on:click={clearKindFilter}
										aria-label="Clear type filter"
										><X size={12} strokeWidth={1.8} aria-hidden="true" /></button
									>
								{/if}
							</div>
							{#if showKindDropdown}
								<div
									class="absolute left-0 top-full z-20 mt-1 max-h-64 w-52 overflow-y-auto rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] p-2 shadow-xl"
								>
									{#each ALL_NODE_KINDS as kind}
										<label
											class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel))]"
										>
											<input
												type="checkbox"
												class="h-3.5 w-3.5 rounded border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] text-sky-500"
												checked={selectedKinds.has(kind)}
												on:change={() => toggleKind(kind)}
											/>
											<span class={`rounded-full px-1.5 py-0.5 text-[9px] ${kindBadgeClass(kind)}`}
												>{kindLabel(kind)}</span
											>
										</label>
									{/each}
								</div>
							{/if}
						</div>

						<div>
							<label
								class="mb-1.5 block text-[10px] font-semibold uppercase tracking-[0.12em] text-[rgb(var(--mv-muted))]/75"
								for="search-sort">Sort</label
							>
							<select
								id="search-sort"
								class="h-8 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/35 px-2.5 text-xs text-[rgb(var(--mv-text))]"
								bind:value={sortBy}
							>
								{#each sortOptions as option (option.value)}<option value={option.value}
										>{option.label}</option
									>{/each}
							</select>
						</div>

						<div>
							<label
								class="mb-1.5 block text-[10px] font-semibold uppercase tracking-[0.12em] text-[rgb(var(--mv-muted))]/75"
								for="tag-filter">Tag</label
							>
							<input
								id="tag-filter"
								type="text"
								class="h-8 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/35 px-2.5 text-xs text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/55 outline-none focus:border-sky-500"
								placeholder="Any tag"
								bind:value={tagFilter}
							/>
						</div>
					</div>
					<div
						class="mt-3 flex items-center justify-between gap-3 border-t border-[rgb(var(--mv-border))]/70 pt-3"
					>
						<p class="text-[10px] leading-4 text-[rgb(var(--mv-muted))]/65">
							Best match combines exact keywords with related meaning.
						</p>
						{#if activeRefinementCount > 0}<button
								type="button"
								class="shrink-0 text-[10px] font-medium text-sky-300 hover:text-sky-200"
								on:click={clearRefinements}>Reset refinements</button
							>{/if}
					</div>
				</div>
			{/if}

			{#if showSavePanel}
				<div
					id="save-search-panel"
					class="mt-4 grid gap-2 rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/35 p-4 sm:grid-cols-[1fr_auto_auto]"
				>
					<div>
						<label
							for="saved-search-name"
							class="mb-1.5 block text-[10px] font-semibold uppercase tracking-[0.12em] text-[rgb(var(--mv-muted))]/75"
							>Saved search name</label
						>
						<input
							id="saved-search-name"
							class="h-9 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/35 px-3 text-xs text-[rgb(var(--mv-text))] outline-none focus:border-sky-500"
							bind:value={savedSearchName}
						/>
					</div>
					<button
						type="button"
						class="self-end rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-slate-950 disabled:opacity-60"
						disabled={savePending}
						on:click={() => void saveCurrentSearch()}>{savePending ? 'Saving…' : 'Save'}</button
					>
					<button
						type="button"
						class="self-end rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel))]"
						on:click={() => (showSavePanel = false)}>Cancel</button
					>
				</div>
			{/if}
		</div>

		{#if viewMode === 'clusters'}
			<div
				class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/30 p-5"
			>
				<div class="mb-4 flex items-start justify-between gap-3">
					<div>
						<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Knowledge clusters</h2>
						<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]/70">
							Explore groups that already exist across your vault.
						</p>
					</div>
					<button
						type="button"
						class="inline-flex items-center gap-1.5 rounded-lg border border-[rgb(var(--mv-border))] px-2.5 py-1.5 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
						on:click={() => void fetchClusters()}
						disabled={clustersLoading}
					>
						<RefreshCw
							class={clustersLoading ? 'animate-spin' : ''}
							size={12}
							strokeWidth={1.8}
							aria-hidden="true"
						/> Refresh
					</button>
				</div>
				{#if clustersLoading}
					<div class="space-y-3" role="status" aria-label="Loading knowledge clusters">
						{#each Array(3) as _}<div
								class="h-24 animate-pulse rounded-xl bg-[rgb(var(--mv-panel-strong))]/55"
							></div>{/each}
					</div>
				{:else if clustersError}
					<div class="rounded-xl border border-red-500/25 bg-red-500/10 p-4" role="alert">
						<p class="text-xs text-red-200">{clustersError}</p>
						<button
							type="button"
							class="mt-2 text-xs font-medium text-red-100 underline underline-offset-2"
							on:click={() => void fetchClusters()}>Try again</button
						>
					</div>
				{:else if clusters.length === 0}
					<EmptyState
						compact
						icon="generic"
						tone="slate"
						title="No clusters yet"
						description="Clusters will appear as your vault develops more connected knowledge."
					/>
				{:else}
					<div class="space-y-3">
						{#each clusters as cluster (cluster.id)}
							<article
								class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-4"
							>
								<div class="flex items-start gap-3">
									<span
										class="rounded-full bg-sky-500/15 px-2 py-1 text-[10px] font-medium text-sky-300"
										>{Math.round(cluster.importance * 100)}% strength</span
									>
									<div class="min-w-0 flex-1">
										<h3 class="text-sm font-medium text-[rgb(var(--mv-text))]">{cluster.title}</h3>
										<p class="mt-1 text-xs leading-5 text-[rgb(var(--mv-muted))]">
											{cluster.content}
										</p>
									</div>
								</div>
								{#if cluster.related_node_ids.length > 0}
									<div class="mt-3 flex flex-wrap gap-1.5" aria-label="Cluster nodes">
										{#each cluster.related_node_ids.slice(0, 6) as nodeId}<button
												type="button"
												class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:border-sky-500/40 hover:text-sky-200"
												on:click={() => navigateToGraph(nodeId)}
												>Focus {nodeId.slice(0, 8)} in graph</button
											>{/each}
									</div>
								{/if}
							</article>
						{/each}
					</div>
				{/if}
			</div>
		{:else if searchError}
			<div class="rounded-2xl border border-red-500/25 bg-red-500/10 p-5" role="alert">
				<h2 class="text-sm font-semibold text-red-100">Search unavailable</h2>
				<p class="mt-1 text-xs leading-5 text-red-200/80">{searchError}</p>
				<button
					type="button"
					class="mt-3 rounded-lg border border-red-400/30 px-3 py-1.5 text-xs font-medium text-red-100 hover:bg-red-400/10"
					on:click={() => void doSearch()}>Try again</button
				>
			</div>
		{:else if loading}
			<div class="space-y-3" role="status" aria-label="Searching your Personal Vault">
				<p class="text-xs text-[rgb(var(--mv-muted))]">Searching your Personal Vault…</p>
				{#each Array(3) as _}<div
						class="h-40 animate-pulse rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/45"
					></div>{/each}
			</div>
		{:else if sortedResults.length > 0}
			<div>
				<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
					<p class="text-xs text-[rgb(var(--mv-muted))]" aria-live="polite">
						{sortedResults.length} result{sortedResults.length === 1 ? '' : 's'} in your Personal Vault
					</p>
					<p class="text-[10px] text-[rgb(var(--mv-muted))]/60">Use ↑ ↓ to move between results</p>
				</div>
				<div class="space-y-3" role="list" aria-label="Search results">
					{#each sortedResults as result, index (result.node.id)}
						<SearchResultCard
							{result}
							{index}
							selected={index === selectedIndex}
							on:open={() => navigateToResult(result)}
							on:openGraph={() => navigateToGraph(result.node.id)}
							on:select={() => (selectedIndex = index)}
							on:filterTag={(event) => applyTagFilter(event.detail)}
							on:move={(event) => moveResultFocus(index, event.detail)}
						/>
					{/each}
				</div>
			</div>
		{:else if query.trim() && results.length > 0}
			<EmptyState
				compact
				icon="search"
				tone="slate"
				title="No results match these refinements"
				description="Reset the type, tag, sort, or match settings to see the original results."
			>
				<button
					type="button"
					class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40"
					on:click={clearRefinements}>Reset refinements</button
				>
			</EmptyState>
		{:else if query.trim()}
			<EmptyState
				icon="search"
				tone="slate"
				title={`No results for “${query}”`}
				description={emptySearchHint(searchType)}
			>
				{#if activeRefinementCount > 0}<button
						type="button"
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40"
						on:click={clearRefinements}>Reset refinements</button
					>{/if}
			</EmptyState>
		{:else}
			<EmptyState
				icon="search"
				tone="sky"
				title="Search your vault"
				description="Start with a topic, person, decision, task, or phrase you remember."
			>
				<button
					type="button"
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/40 px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40"
					on:click={() => {
						query = 'project ideas';
						void doSearch();
					}}>“project ideas”</button
				>
				<button
					type="button"
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/40 px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40"
					on:click={() => {
						query = 'meeting notes';
						void doSearch();
					}}>“meeting notes”</button
				>
			</EmptyState>
		{/if}
	</section>

	<aside class="space-y-4 xl:col-span-4">
		<section
			class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-5"
			aria-labelledby="journey-title"
		>
			<h2 id="journey-title" class="text-sm font-semibold text-[rgb(var(--mv-text))]">
				From search to knowledge
			</h2>
			<p class="mt-1 text-[11px] leading-5 text-[rgb(var(--mv-muted))]">
				Open a result to continue working, or view its connections to understand the surrounding
				context.
			</p>
			<div class="mt-4 grid gap-2 text-[10px] text-[rgb(var(--mv-muted))]">
				<div
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/25 px-3 py-2"
				>
					<strong class="text-[rgb(var(--mv-text))]">1. Find</strong><span class="ml-1"
						>by words or meaning</span
					>
				</div>
				<div
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/25 px-3 py-2"
				>
					<strong class="text-[rgb(var(--mv-text))]">2. Verify</strong><span class="ml-1"
						>type, source, and recency</span
					>
				</div>
				<div
					class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/25 px-3 py-2"
				>
					<strong class="text-[rgb(var(--mv-text))]">3. Continue</strong><span class="ml-1"
						>in the item or graph</span
					>
				</div>
			</div>
		</section>

		<section
			class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-5"
			aria-labelledby="connections-title"
		>
			<div class="flex items-center gap-2">
				<Network class="text-sky-300" size={15} strokeWidth={1.8} aria-hidden="true" />
				<h2 id="connections-title" class="text-sm font-semibold text-[rgb(var(--mv-text))]">
					Connections
				</h2>
			</div>
			{#if selectedResult}
				<p class="mt-1 truncate text-[11px] text-[rgb(var(--mv-muted))]">
					From “{selectedResult.node.title || 'Untitled'}”
				</p>
				<button
					type="button"
					class="mt-3 inline-flex items-center gap-1.5 rounded-lg border border-sky-500/30 bg-sky-500/10 px-2.5 py-1.5 text-[10px] font-medium text-sky-200 hover:bg-sky-500/15"
					on:click={() => navigateToGraph(selectedResult.node.id)}
					><Network size={12} strokeWidth={1.8} aria-hidden="true" />View in graph</button
				>
			{:else}
				<p class="mt-1 text-[11px] leading-4 text-[rgb(var(--mv-muted))]">
					Focus a result to explore its existing relationships.
				</p>
			{/if}

			<div class="mt-3 space-y-2" aria-live="polite">
				{#if viewMode !== 'results'}
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">Available in results view.</p>
				{:else if relatedLoading}
					<div class="space-y-2" role="status" aria-label="Loading connections">
						<div class="h-14 animate-pulse rounded-lg bg-[rgb(var(--mv-panel-strong))]/50"></div>
						<div class="h-14 animate-pulse rounded-lg bg-[rgb(var(--mv-panel-strong))]/50"></div>
					</div>
				{:else if relatedError}
					<div class="rounded-lg border border-amber-500/25 bg-amber-500/10 p-3">
						<p class="text-xs text-amber-100">{relatedError}</p>
						{#if selectedResult}<button
								type="button"
								class="mt-2 text-[10px] font-medium text-amber-100 underline underline-offset-2"
								on:click={() => void fetchRelatedNodes(selectedResult.node.id)}>Try again</button
							>{/if}
					</div>
				{:else if selectedResult && relatedNodes.length === 0}
					<p
						class="rounded-lg border border-dashed border-[rgb(var(--mv-border))] p-3 text-xs leading-5 text-[rgb(var(--mv-muted))]/60"
					>
						No saved connections for this result yet.
					</p>
				{:else if !selectedResult}
					<p
						class="rounded-lg border border-dashed border-[rgb(var(--mv-border))] p-3 text-xs leading-5 text-[rgb(var(--mv-muted))]/60"
					>
						Search and focus a result to see its connections.
					</p>
				{:else}
					{#each relatedNodes.slice(0, 8) as neighbor (neighbor.node.id + neighbor.relationship_kind)}
						<a
							href={resultDestination(neighbor.node)}
							class="block rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2.5 text-xs outline-none transition hover:border-sky-500/30 focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
						>
							<div class="flex items-center gap-2">
								<span
									class={`rounded-full px-1.5 py-0.5 text-[9px] ${kindBadgeClass(neighbor.node.kind)}`}
									>{kindLabel(neighbor.node.kind)}</span
								><span class="truncate text-[rgb(var(--mv-text))]"
									>{neighbor.node.title || 'Untitled'}</span
								>
							</div>
							<p class="mt-1 text-[10px] text-[rgb(var(--mv-muted))]/70">
								{neighbor.direction} · {neighbor.relationship_kind}
							</p>
						</a>
					{/each}
				{/if}
			</div>
		</section>
	</aside>
</div>
