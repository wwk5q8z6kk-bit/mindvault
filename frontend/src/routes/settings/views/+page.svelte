<script lang="ts">
	import { onMount } from 'svelte';
	import {
		listSavedViews,
		createSavedView,
		updateSavedView,
		deleteSavedView,
		type SavedView,
		type SavedViewType,
		type CreateSavedViewPayload
	} from '$lib/api/saved-views';
	import { pushToast } from '$lib/stores/toast';
	import { setActiveSavedView } from '$lib/stores/saved-views';
	import { goto } from '$app/navigation';

	let views: SavedView[] = [];
	let loading = true;
	let showForm = false;
	let editingView: SavedView | null = null;

	// Form state
	let formName = '';
	let formViewType: SavedViewType = 'list';
	let formQuery = '';
	let formGroupBy = '';
	let formSortField = '';
	let formSortDirection: 'asc' | 'desc' = 'desc';
	let formSaving = false;

	// Filter status options
	let formFilterStatuses: string[] = [];
	const statusOptions = ['inbox', 'planned', 'in_progress', 'waiting', 'review', 'done'];

	onMount(async () => {
		await loadViews();
	});

	async function loadViews() {
		loading = true;
		try {
			views = await listSavedViews({ limit: 100 });
		} catch {
			pushToast('Failed to load saved views', 'danger');
		} finally {
			loading = false;
		}
	}

	function openCreate() {
		editingView = null;
		formName = '';
		formViewType = 'list';
		formQuery = '';
		formGroupBy = '';
		formSortField = '';
		formSortDirection = 'desc';
		formFilterStatuses = [];
		showForm = true;
	}

	function openEdit(view: SavedView) {
		editingView = view;
		formName = view.name;
		formViewType = view.view_type;
		formQuery = view.query ?? '';
		formGroupBy = view.group_by ?? '';
		formSortField = view.sort?.field ?? '';
		formSortDirection = view.sort?.direction ?? 'desc';
		formFilterStatuses = (view.filters?.statuses as string[]) ?? [];
		showForm = true;
	}

	function closeForm() {
		showForm = false;
		editingView = null;
	}

	async function saveView() {
		if (!formName.trim()) {
			pushToast('Name is required', 'warning');
			return;
		}

		formSaving = true;

		try {
			const filters: Record<string, unknown> = {};
			if (formFilterStatuses.length > 0) {
				filters.statuses = formFilterStatuses;
			}

			const payload: CreateSavedViewPayload = {
				name: formName.trim(),
				view_type: formViewType,
				query: formQuery.trim() || undefined,
				group_by: formGroupBy.trim() || undefined,
				filters,
				sort: formSortField.trim()
					? { field: formSortField.trim(), direction: formSortDirection }
					: undefined
			};

			if (editingView) {
				await updateSavedView(editingView.id, payload);
				pushToast('View updated', 'success');
			} else {
				await createSavedView(payload);
				pushToast('View created', 'success');
			}

			closeForm();
			await loadViews();
		} catch {
			pushToast('Failed to save view', 'danger');
		} finally {
			formSaving = false;
		}
	}

	async function handleDelete(view: SavedView) {
		if (!confirm(`Delete view "${view.name}"?`)) return;

		try {
			await deleteSavedView(view.id);
			pushToast('View deleted', 'success');
			await loadViews();
		} catch {
			pushToast('Failed to delete view', 'danger');
		}
	}

	function handleApply(view: SavedView) {
		setActiveSavedView(view);
		if (view.view_type === 'kanban') {
			goto('/kanban');
		} else if (view.view_type === 'calendar') {
			goto('/calendar');
		} else {
			goto('/tasks');
		}
	}

	function toggleStatus(status: string) {
		if (formFilterStatuses.includes(status)) {
			formFilterStatuses = formFilterStatuses.filter((s) => s !== status);
		} else {
			formFilterStatuses = [...formFilterStatuses, status];
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	const viewTypeIcons: Record<SavedViewType, string> = {
		list: 'M4 6h16M4 10h16M4 14h16M4 18h16',
		kanban: 'M9 3h6v18H9M3 3h6v12H3m12 0h6v6h-6V15',
		calendar: 'M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z'
	};
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-lg font-semibold text-white">Saved Views</h1>
			<p class="text-xs text-slate-400">Create and manage custom task views</p>
		</div>
		<div class="flex gap-2">
			<a
				href="/settings"
				class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
			>
				Back to Settings
			</a>
			<button
				class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={openCreate}
			>
				New View
			</button>
		</div>
	</div>

	{#if showForm}
		<div class="rounded-2xl border border-slate-800 bg-slate-900/60 p-6">
			<h2 class="mb-4 text-sm font-semibold text-white">
				{editingView ? 'Edit View' : 'Create View'}
			</h2>

			<div class="grid gap-4 md:grid-cols-2">
				<div>
					<label for="view-name" class="mb-1 block text-xs text-slate-400">Name</label>
					<input
						id="view-name"
						type="text"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500 outline-none focus:border-sky-500"
						placeholder="My view..."
						bind:value={formName}
					/>
				</div>

				<fieldset>
					<legend class="mb-1 block text-xs text-slate-400">View Type</legend>
					<div class="flex rounded-lg border border-slate-700 text-xs">
						{#each (['list', 'kanban', 'calendar'] as const) as vt}
							<button
								type="button"
								class={`flex-1 px-3 py-2 transition ${formViewType === vt ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
								on:click={() => (formViewType = vt)}
							>
								{vt.charAt(0).toUpperCase() + vt.slice(1)}
							</button>
						{/each}
					</div>
				</fieldset>

				<div>
					<label for="view-query" class="mb-1 block text-xs text-slate-400">Search Query (optional)</label>
					<input
						id="view-query"
						type="text"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500 outline-none focus:border-sky-500"
						placeholder="Filter text..."
						bind:value={formQuery}
					/>
				</div>

				<div>
					<label for="view-group" class="mb-1 block text-xs text-slate-400">Group By (optional)</label>
					<select
						id="view-group"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white outline-none focus:border-sky-500"
						bind:value={formGroupBy}
					>
						<option value="">No grouping</option>
						<option value="status">Status</option>
						<option value="priority">Priority</option>
						<option value="due_date">Due Date</option>
						<option value="labels">Labels</option>
					</select>
				</div>

				<fieldset class="md:col-span-2">
					<legend class="mb-1 block text-xs text-slate-400">Filter by Status (optional)</legend>
					<div class="flex flex-wrap gap-2">
						{#each statusOptions as status}
							<button
								type="button"
								class={`rounded-lg border px-2 py-1 text-xs transition ${
									formFilterStatuses.includes(status)
										? 'border-sky-500 bg-sky-500/20 text-sky-300'
										: 'border-slate-700 text-slate-400 hover:border-slate-600'
								}`}
								on:click={() => toggleStatus(status)}
							>
								{status.replace('_', ' ')}
							</button>
						{/each}
					</div>
				</fieldset>

				<div>
					<label for="view-sort-field" class="mb-1 block text-xs text-slate-400">Sort By (optional)</label>
					<select
						id="view-sort-field"
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white outline-none focus:border-sky-500"
						bind:value={formSortField}
					>
						<option value="">Default</option>
						<option value="due_at">Due Date</option>
						<option value="priority">Priority</option>
						<option value="created_at">Created</option>
						<option value="updated_at">Updated</option>
						<option value="title">Title</option>
					</select>
				</div>

				<fieldset>
					<legend class="mb-1 block text-xs text-slate-400">Sort Direction</legend>
					<div class="flex rounded-lg border border-slate-700 text-xs">
						<button
							type="button"
							class={`flex-1 px-3 py-2 transition ${formSortDirection === 'asc' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
							on:click={() => (formSortDirection = 'asc')}
						>
							Ascending
						</button>
						<button
							type="button"
							class={`flex-1 px-3 py-2 transition ${formSortDirection === 'desc' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
							on:click={() => (formSortDirection = 'desc')}
						>
							Descending
						</button>
					</div>
				</fieldset>
			</div>

			<div class="mt-4 flex gap-2">
				<button
					class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					disabled={formSaving}
					on:click={saveView}
				>
					{formSaving ? 'Saving...' : editingView ? 'Update' : 'Create'}
				</button>
				<button
					class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 hover:bg-slate-800"
					on:click={closeForm}
				>
					Cancel
				</button>
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-sm text-slate-400">Loading saved views...</div>
		</div>
	{:else if views.length === 0}
		<div class="rounded-2xl border border-dashed border-slate-800 bg-slate-900/20 p-12 text-center">
			<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-sky-500/20 text-sky-300">
				<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
				</svg>
			</div>
			<h3 class="text-sm font-medium text-white">No saved views yet</h3>
			<p class="mt-1 text-xs text-slate-500">Create custom views for quick access to filtered tasks</p>
			<button
				class="mt-4 rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={openCreate}
			>
				Create Your First View
			</button>
		</div>
	{:else}
		<div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
			{#each views as view (view.id)}
				<div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4 transition hover:border-slate-700">
					<div class="flex items-start justify-between gap-2">
						<div class="flex items-center gap-2">
							<div class="flex h-8 w-8 items-center justify-center rounded-lg bg-slate-800">
								<svg class="h-4 w-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={viewTypeIcons[view.view_type]} />
								</svg>
							</div>
							<div>
								<h3 class="text-sm font-medium text-white">{view.name}</h3>
								<span class="text-[10px] text-slate-500">{view.view_type}</span>
							</div>
						</div>
					</div>

					{#if view.query}
						<p class="mt-2 text-xs text-slate-400">Query: "{view.query}"</p>
					{/if}

					{#if view.filters?.statuses && Array.isArray(view.filters.statuses)}
						<div class="mt-2 flex flex-wrap gap-1">
							{#each view.filters.statuses as status}
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-500">
									{status}
								</span>
							{/each}
						</div>
					{/if}

					{#if view.sort}
						<p class="mt-1 text-[10px] text-slate-500">
							Sort: {view.sort.field} ({view.sort.direction})
						</p>
					{/if}

					<div class="mt-3 flex items-center justify-between border-t border-slate-800 pt-3">
						<span class="text-[10px] text-slate-600">
							Updated {formatDate(view.updated_at)}
						</span>
						<div class="flex gap-1">
							<button
								class="rounded-lg bg-sky-500/20 px-2 py-1 text-[10px] text-sky-300 hover:bg-sky-500/30"
								on:click={() => handleApply(view)}
							>
								Apply
							</button>
							<button
								class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
								on:click={() => openEdit(view)}
							>
								Edit
							</button>
							<button
								class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10"
								on:click={() => handleDelete(view)}
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
