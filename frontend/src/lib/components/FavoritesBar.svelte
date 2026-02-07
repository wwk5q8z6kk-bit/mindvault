<script lang="ts">
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { tasksStore } from '$lib/stores/tasks';
	import { notesStore } from '$lib/stores/notes';
	import type { TaskRecord } from '$lib/db';
	import type { Note } from '$lib/api/notes';

	const COLLAPSED_KEY = 'mindvault_favorites_bar_collapsed';

	let collapsed = false;

	if (browser) {
		collapsed = localStorage.getItem(COLLAPSED_KEY) === 'true';
	}

	function toggleCollapsed() {
		collapsed = !collapsed;
		if (browser) {
			localStorage.setItem(COLLAPSED_KEY, String(collapsed));
		}
	}

	// Get pinned tasks
	$: pinnedTasks = $tasksStore
		.filter((t) => t.metadata?.pinned === true && t.status !== 'done')
		.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
		.slice(0, 8);

	// Get pinned notes
	$: pinnedNotes = $notesStore
		.filter((n) => n.pinned === true)
		.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
		.slice(0, 8);

	$: hasFavorites = pinnedTasks.length > 0 || pinnedNotes.length > 0;
	$: totalCount = pinnedTasks.length + pinnedNotes.length;

	function navigateToTask(task: TaskRecord) {
		goto(`/tasks?task=${task.id}`);
	}

	function navigateToNote(note: Note) {
		goto(`/notes?note=${note.id}`);
	}

	function getPriorityColor(priority: number): string {
		const colors: Record<number, string> = {
			1: 'bg-red-500',
			2: 'bg-orange-500',
			3: 'bg-yellow-500',
			4: 'bg-blue-500',
			5: 'bg-slate-500'
		};
		return colors[priority] ?? colors[3];
	}

	function getStatusClass(status: string): string {
		const classes: Record<string, string> = {
			inbox: 'border-slate-600',
			planned: 'border-violet-500/50',
			in_progress: 'border-blue-500/50',
			waiting: 'border-amber-500/50',
			review: 'border-cyan-500/50'
		};
		return classes[status] ?? classes['inbox'];
	}

	function truncate(text: string, max: number): string {
		if (!text) return '';
		return text.length > max ? text.slice(0, max) + '...' : text;
	}
</script>

{#if hasFavorites}
	<div class="border-b border-slate-800/60 bg-slate-900/30">
		<div class="flex items-center gap-2 px-4 py-1.5">
			<button
				class="flex items-center gap-1.5 text-[10px] text-slate-500 hover:text-slate-300 transition"
				on:click={toggleCollapsed}
				aria-label={collapsed ? 'Expand favorites' : 'Collapse favorites'}
			>
				<svg
					class="h-3 w-3 transition-transform {collapsed ? '' : 'rotate-90'}"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
				</svg>
				<svg class="h-3.5 w-3.5 text-amber-400" fill="currentColor" viewBox="0 0 24 24">
					<path d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
				</svg>
				<span class="font-medium">Favorites</span>
				<span class="rounded-full bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{totalCount}</span>
			</button>

			{#if !collapsed}
				<div class="flex flex-1 items-center gap-2 overflow-x-auto scrollbar-none">
					<!-- Pinned Tasks -->
					{#each pinnedTasks as task (task.id)}
						<button
							class="flex flex-shrink-0 items-center gap-1.5 rounded-lg border bg-slate-800/50 px-2 py-1 transition hover:bg-slate-800 {getStatusClass(task.status)}"
							on:click={() => navigateToTask(task)}
							title={task.title}
						>
							<span class="h-1.5 w-1.5 rounded-full {getPriorityColor(task.priority)}"></span>
							<span class="max-w-[120px] truncate text-[10px] text-slate-200">{truncate(task.title, 20)}</span>
							<span class="rounded bg-slate-700 px-1 py-0.5 text-[8px] text-slate-400">task</span>
						</button>
					{/each}

					<!-- Pinned Notes -->
					{#each pinnedNotes as note (note.id)}
						<button
							class="flex flex-shrink-0 items-center gap-1.5 rounded-lg border border-sky-500/30 bg-slate-800/50 px-2 py-1 transition hover:bg-slate-800"
							on:click={() => navigateToNote(note)}
							title={note.title ?? 'Untitled'}
						>
							<svg class="h-3 w-3 text-sky-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
							</svg>
							<span class="max-w-[120px] truncate text-[10px] text-slate-200">{truncate(note.title ?? 'Untitled', 20)}</span>
							<span class="rounded bg-slate-700 px-1 py-0.5 text-[8px] text-slate-400">note</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.scrollbar-none {
		-ms-overflow-style: none;
		scrollbar-width: none;
	}
	.scrollbar-none::-webkit-scrollbar {
		display: none;
	}
</style>
