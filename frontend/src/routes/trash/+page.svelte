<script lang="ts">
	import { onMount } from 'svelte';
	import {
		tasksStore,
		loadTasks,
		restoreTaskOptimistic,
		deleteTaskOptimistic
	} from '$lib/stores/tasks';
	import {
		notesStore,
		loadNotes,
		restoreNoteOptimistic,
		deleteNoteOptimistic
	} from '$lib/stores/notes';
	import { pushToast } from '$lib/stores/toast';

	interface TrashedItem {
		id: string;
		title: string;
		type: 'task' | 'note';
		trashed_at: string;
	}

	let showEmptyConfirm = false;

	$: trashedItems = [
		...$tasksStore
			.filter((t) => t.labels?.includes('trashed'))
			.map((t): TrashedItem => ({
				id: t.id,
				title: t.title,
				type: 'task',
				trashed_at: typeof t.metadata?.trashed_at === 'string' ? t.metadata.trashed_at : t.updated_at
			})),
		...$notesStore
			.filter((n) => n.tags?.includes('trashed'))
			.map((n): TrashedItem => ({
				id: n.id,
				title: n.title ?? 'Untitled note',
				type: 'note',
				trashed_at: typeof n.metadata?.trashed_at === 'string' ? (n.metadata.trashed_at as string) : n.updated_at
			}))
	].sort((a, b) => new Date(b.trashed_at).getTime() - new Date(a.trashed_at).getTime());

	onMount(() => {
		loadTasks();
		loadNotes();
	});

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	async function restore(item: TrashedItem) {
		try {
			if (item.type === 'task') {
				await restoreTaskOptimistic(item.id);
			} else {
				await restoreNoteOptimistic(item.id);
			}
			pushToast(`Restored "${item.title}"`, 'success');
		} catch {
			pushToast('Failed to restore item', 'danger');
		}
	}

	async function permanentDelete(item: TrashedItem) {
		try {
			if (item.type === 'task') {
				await deleteTaskOptimistic(item.id);
			} else {
				await deleteNoteOptimistic(item.id);
			}
			pushToast(`Permanently deleted "${item.title}"`, 'success');
		} catch {
			pushToast('Failed to delete item', 'danger');
		}
	}

	async function emptyTrash() {
		let count = 0;
		for (const item of trashedItems) {
			try {
				if (item.type === 'task') {
					await deleteTaskOptimistic(item.id);
				} else {
					await deleteNoteOptimistic(item.id);
				}
				count++;
			} catch {
				// continue
			}
		}
		pushToast(`Permanently deleted ${count} item${count === 1 ? '' : 's'}`, 'success');
		showEmptyConfirm = false;
	}
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Trash</h2>
			<p class="text-xs text-slate-400">
				{trashedItems.length} item{trashedItems.length === 1 ? '' : 's'} in trash
			</p>
		</div>
		{#if trashedItems.length > 0}
			<button
				class="rounded-lg bg-red-500/20 px-3 py-2 text-xs font-medium text-red-200 hover:bg-red-500/30"
				on:click={() => (showEmptyConfirm = true)}
			>
				Empty Trash
			</button>
		{/if}
	</div>

	<div class="mt-6 flex flex-col gap-3">
		{#if trashedItems.length === 0}
			<div class="rounded-xl border border-dashed border-slate-800 bg-slate-900/20 p-8 text-center">
				<p class="text-sm text-slate-400">Trash is empty.</p>
				<p class="mt-1 text-xs text-slate-500">Deleted items will appear here for recovery.</p>
			</div>
		{:else}
			{#each trashedItems as item (item.id)}
				<div class="flex items-center gap-4 rounded-xl border border-slate-900 bg-slate-900/40 px-4 py-3">
					<div class="min-w-0 flex-1">
						<h3 class="truncate text-sm font-medium text-white">{item.title}</h3>
						<div class="mt-1 flex items-center gap-2 text-[11px] text-slate-500">
							<span class="rounded-full bg-slate-800 px-2 py-0.5 text-[9px] uppercase tracking-wide {item.type === 'task' ? 'text-blue-300' : 'text-violet-300'}">
								{item.type}
							</span>
							<span>Trashed {formatDate(item.trashed_at)}</span>
						</div>
					</div>
					<div class="flex gap-2 flex-shrink-0">
						<button
							class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-200 hover:border-emerald-500/50 hover:text-emerald-200"
							on:click={() => restore(item)}
						>
							Restore
						</button>
						<button
							class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-400 hover:border-red-500/50 hover:text-red-300"
							on:click={() => permanentDelete(item)}
						>
							Delete
						</button>
					</div>
				</div>
			{/each}
		{/if}
	</div>
</div>

<!-- Empty trash confirmation -->
{#if showEmptyConfirm}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
		<div class="w-full max-w-sm rounded-xl border border-slate-700 bg-slate-900 p-6 shadow-xl">
			<h3 class="text-sm font-semibold text-white">Empty trash?</h3>
			<p class="mt-2 text-xs text-slate-400">
				This will permanently delete {trashedItems.length} item{trashedItems.length === 1 ? '' : 's'}. This action cannot be undone.
			</p>
			<div class="mt-4 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:border-slate-500"
					on:click={() => (showEmptyConfirm = false)}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-red-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-400"
					on:click={emptyTrash}
				>
					Delete all permanently
				</button>
			</div>
		</div>
	</div>
{/if}
