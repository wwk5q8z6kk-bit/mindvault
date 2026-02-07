<script lang="ts">
	import {
		listSavedSearches,
		createSavedSearch,
		updateSavedSearch,
		deleteSavedSearch,
		runSavedSearch,
		type SavedSearch
	} from '$lib/api/search';
	import { pushToast } from '$lib/stores/toast';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { ALL_NODE_KINDS, kindLabel } from '$lib/utils/kind-helpers';

	let searches: SavedSearch[] = [];
	let loading = true;
	let editingId: string | null = null;

	// Form state
	let showCreateForm = false;
	let formName = '';
	let formQuery = '';
	let formSearchType: 'fulltext' | 'hybrid' = 'fulltext';
	let formKinds: string[] = [];
	let formLimit = 50;
	let formSaving = false;

	const kindOptions = ALL_NODE_KINDS.map((k) => ({ value: k, label: kindLabel(k) }));

	async function loadSearches() {
		loading = true;
		try {
			searches = await listSavedSearches(100, 0);
		} catch {
			pushToast('Failed to load saved searches', 'danger');
		} finally {
			loading = false;
		}
	}

	function startCreate() {
		editingId = null;
		formName = '';
		formQuery = '';
		formSearchType = 'fulltext';
		formKinds = [];
		formLimit = 50;
		showCreateForm = true;
	}

	function startEdit(search: SavedSearch) {
		editingId = search.id;
		formName = search.name;
		formQuery = search.query;
		formSearchType = search.search_type === 'hybrid' ? 'hybrid' : 'fulltext';
		formKinds = [...search.kinds];
		formLimit = search.limit;
		showCreateForm = true;
	}

	function cancelForm() {
		showCreateForm = false;
		editingId = null;
	}

	async function saveForm() {
		if (!formName.trim() || !formQuery.trim()) {
			pushToast('Name and query are required', 'warning');
			return;
		}

		formSaving = true;
		try {
			const payload = {
				name: formName.trim(),
				query: formQuery.trim(),
				search_type: formSearchType,
				kinds: formKinds,
				limit: formLimit
			};

			if (editingId) {
				await updateSavedSearch(editingId, payload);
				pushToast('Saved search updated', 'success');
			} else {
				await createSavedSearch(payload);
				pushToast('Saved search created', 'success');
			}

			cancelForm();
			await loadSearches();
		} catch {
			pushToast('Failed to save', 'danger');
		} finally {
			formSaving = false;
		}
	}

	async function handleDelete(search: SavedSearch) {
		if (!confirm(`Delete "${search.name}"?`)) return;

		try {
			await deleteSavedSearch(search.id);
			pushToast('Deleted', 'success');
			await loadSearches();
		} catch {
			pushToast('Failed to delete', 'danger');
		}
	}

	async function handleRun(search: SavedSearch) {
		try {
			const response = await runSavedSearch(search.id);
			// Navigate to search page with results
			goto(`/search?q=${encodeURIComponent(search.query)}`);
		} catch {
			pushToast('Failed to run search', 'danger');
		}
	}

	function toggleKind(kind: string) {
		if (formKinds.includes(kind)) {
			formKinds = formKinds.filter((k) => k !== kind);
		} else {
			formKinds = [...formKinds, kind];
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	onMount(() => {
		void loadSearches();
	});
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-lg font-semibold text-white">Saved Searches</h1>
			<p class="text-xs text-slate-400">Create and manage reusable search queries</p>
		</div>
		<div class="flex gap-2">
			<a
				href="/search"
				class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
			>
				Back to Search
			</a>
			<button
				class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={startCreate}
			>
				New Saved Search
			</button>
		</div>
	</div>

	{#if showCreateForm}
		<div class="rounded-2xl border border-slate-800 bg-slate-900/60 p-6">
			<h2 class="mb-4 text-sm font-semibold text-white">
				{editingId ? 'Edit Saved Search' : 'Create Saved Search'}
			</h2>

			<div class="grid gap-4 md:grid-cols-2">
				<div>
					<label for="saved-search-name" class="mb-1 block text-xs text-slate-400">Name</label>
					<input
						id="saved-search-name"
						type="text"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500 outline-none focus:border-sky-500"
						placeholder="My search..."
						bind:value={formName}
					/>
				</div>

				<fieldset>
					<legend class="mb-1 block text-xs text-slate-400">Search Type</legend>
					<div class="flex rounded-lg border border-slate-700 text-xs">
						<button
							type="button"
							class={`flex-1 px-3 py-2 transition ${formSearchType === 'fulltext' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
							on:click={() => (formSearchType = 'fulltext')}
						>
							Fulltext
						</button>
						<button
							type="button"
							class={`flex-1 px-3 py-2 transition ${formSearchType === 'hybrid' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
							on:click={() => (formSearchType = 'hybrid')}
						>
							Hybrid (Semantic)
						</button>
					</div>
				</fieldset>

				<div class="md:col-span-2">
					<label for="saved-search-query" class="mb-1 block text-xs text-slate-400">Query</label>
					<input
						id="saved-search-query"
						type="text"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500 outline-none focus:border-sky-500"
						placeholder="Search query..."
						bind:value={formQuery}
					/>
				</div>

				<fieldset class="md:col-span-2">
					<legend class="mb-1 block text-xs text-slate-400">Filter by Kind (optional)</legend>
					<div class="flex flex-wrap gap-2">
						{#each kindOptions as opt (opt.value)}
							<button
								type="button"
								class={`rounded-lg border px-2 py-1 text-xs transition ${
									formKinds.includes(opt.value)
										? 'border-sky-500 bg-sky-500/20 text-sky-300'
										: 'border-slate-700 text-slate-400 hover:border-slate-600'
								}`}
								on:click={() => toggleKind(opt.value)}
							>
								{opt.label}
							</button>
						{/each}
					</div>
				</fieldset>

				<div>
					<label for="saved-search-limit" class="mb-1 block text-xs text-slate-400">Result Limit</label>
					<input
						id="saved-search-limit"
						type="number"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white outline-none focus:border-sky-500"
						min="1"
						max="200"
						bind:value={formLimit}
					/>
				</div>
			</div>

			<div class="mt-4 flex gap-2">
				<button
					class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					disabled={formSaving}
					on:click={saveForm}
				>
					{formSaving ? 'Saving...' : editingId ? 'Update' : 'Create'}
				</button>
				<button
					class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 hover:bg-slate-800"
					on:click={cancelForm}
				>
					Cancel
				</button>
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-sm text-slate-400">Loading saved searches...</div>
		</div>
	{:else if searches.length === 0}
		<div class="rounded-2xl border border-dashed border-slate-800 bg-slate-900/20 p-12 text-center">
			<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-sky-500/20 text-sky-300">
				<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
				</svg>
			</div>
			<h3 class="text-sm font-medium text-white">No saved searches yet</h3>
			<p class="mt-1 text-xs text-slate-500">Save your frequent queries for quick access</p>
			<button
				class="mt-4 rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={startCreate}
			>
				Create Your First Saved Search
			</button>
		</div>
	{:else}
		<div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
			{#each searches as search (search.id)}
				<div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4 transition hover:border-slate-700">
					<div class="flex items-start justify-between gap-2">
						<h3 class="text-sm font-medium text-white">{search.name}</h3>
						<span class={`rounded-full px-2 py-0.5 text-[10px] ${
							search.search_type === 'hybrid'
								? 'bg-purple-500/20 text-purple-300'
								: 'bg-slate-700 text-slate-300'
						}`}>
							{search.search_type === 'hybrid' ? 'Hybrid' : 'Fulltext'}
						</span>
					</div>

					<p class="mt-2 line-clamp-2 text-xs text-slate-400">"{search.query}"</p>

					{#if search.kinds.length > 0}
						<div class="mt-2 flex flex-wrap gap-1">
							{#each search.kinds as kind}
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-500">
									{kindLabel(kind)}
								</span>
							{/each}
						</div>
					{/if}

					<div class="mt-3 flex items-center justify-between border-t border-slate-800 pt-3">
						<span class="text-[10px] text-slate-600">
							Updated {formatDate(search.updated_at)}
						</span>
						<div class="flex gap-1">
							<button
								class="rounded-lg bg-sky-500/20 px-2 py-1 text-[10px] text-sky-300 hover:bg-sky-500/30"
								on:click={() => handleRun(search)}
								title="Run search"
							>
								Run
							</button>
							<button
								class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
								on:click={() => startEdit(search)}
								title="Edit"
							>
								Edit
							</button>
							<button
								class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10"
								on:click={() => handleDelete(search)}
								title="Delete"
							>
								Delete
							</button>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
