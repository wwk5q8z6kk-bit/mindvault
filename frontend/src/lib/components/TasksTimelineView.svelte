<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { tasksStore, loadTasks } from '$lib/stores/tasks';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { selectedTaskId, selectedNoteId } from '$lib/stores/ui';
	import type { TaskRecord } from '$lib/db';
	import type { Note } from '$lib/api/notes';

	interface TimelineEntry {
		id: string;
		type: 'task_created' | 'task_completed' | 'task_updated' | 'note_created' | 'note_updated';
		title: string;
		timestamp: string;
		objectId: string;
		objectType: 'task' | 'note';
		metadata?: Record<string, unknown>;
	}

	let entries: TimelineEntry[] = [];
	let filter: 'all' | 'tasks' | 'notes' = 'all';
	let dateRange: 'all' | 'today' | 'week' | 'month' = 'all';
	let eventTypes: Set<string> = new Set(['task_created', 'task_completed', 'task_updated', 'note_created', 'note_updated']);
	let searchQuery = '';
	let showFilters = false;

	onMount(async () => {
		await Promise.all([loadTasks(), loadNotes()]);
		buildTimeline();
	});

	$: {
		$tasksStore;
		$notesStore;
		buildTimeline();
	}

	function buildTimeline() {
		const items: TimelineEntry[] = [];

		for (const task of $tasksStore) {
			items.push({
				id: `task-created-${task.id}`,
				type: 'task_created',
				title: `Created task: ${task.title}`,
				timestamp: task.created_at,
				objectId: task.id,
				objectType: 'task'
			});
			if (task.completed_at) {
				items.push({
					id: `task-completed-${task.id}`,
					type: 'task_completed',
					title: `Completed: ${task.title}`,
					timestamp: task.completed_at,
					objectId: task.id,
					objectType: 'task'
				});
			}
			if (task.updated_at !== task.created_at) {
				items.push({
					id: `task-updated-${task.id}`,
					type: 'task_updated',
					title: `Updated task: ${task.title}`,
					timestamp: task.updated_at,
					objectId: task.id,
					objectType: 'task'
				});
			}
		}

		for (const note of $notesStore) {
			items.push({
				id: `note-created-${note.id}`,
				type: 'note_created',
				title: `Created note: ${note.title ?? 'Untitled'}`,
				timestamp: note.created_at,
				objectId: note.id,
				objectType: 'note'
			});
			if (note.updated_at !== note.created_at) {
				items.push({
					id: `note-updated-${note.id}`,
					type: 'note_updated',
					title: `Updated note: ${note.title ?? 'Untitled'}`,
					timestamp: note.updated_at,
					objectId: note.id,
					objectType: 'note'
				});
			}
		}

		items.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
		entries = items;
	}

	$: filteredEntries = (() => {
		let result = entries;

		if (filter !== 'all') {
			result = result.filter((e) => e.objectType === (filter === 'tasks' ? 'task' : 'note'));
		}

		if (eventTypes.size < 5) {
			result = result.filter((e) => eventTypes.has(e.type));
		}

		if (dateRange !== 'all') {
			const now = new Date();
			const cutoff = new Date();
			if (dateRange === 'today') {
				cutoff.setHours(0, 0, 0, 0);
			} else if (dateRange === 'week') {
				cutoff.setDate(now.getDate() - 7);
			} else if (dateRange === 'month') {
				cutoff.setDate(now.getDate() - 30);
			}
			result = result.filter((e) => new Date(e.timestamp) >= cutoff);
		}

		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			result = result.filter((e) => e.title.toLowerCase().includes(q));
		}

		return result;
	})();

	function toggleEventType(type: string) {
		const newSet = new Set(eventTypes);
		if (newSet.has(type)) {
			if (newSet.size > 1) {
				newSet.delete(type);
			}
		} else {
			newSet.add(type);
		}
		eventTypes = newSet;
	}

	function resetFilters() {
		filter = 'all';
		dateRange = 'all';
		eventTypes = new Set(['task_created', 'task_completed', 'task_updated', 'note_created', 'note_updated']);
		searchQuery = '';
	}

	$: hasActiveFilters = filter !== 'all' || dateRange !== 'all' || eventTypes.size < 5 || searchQuery.trim() !== '';

	function formatTimestamp(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diffMs = now.getTime() - d.getTime();
		const diffMin = Math.floor(diffMs / 60000);
		if (diffMin < 1) return 'Just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;
		const diffDays = Math.floor(diffHr / 24);
		if (diffDays < 7) return `${diffDays}d ago`;
		return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}

	const typeColors: Record<string, string> = {
		task_created: 'bg-sky-500',
		task_completed: 'bg-emerald-500',
		task_updated: 'bg-amber-500',
		note_created: 'bg-violet-500',
		note_updated: 'bg-purple-400'
	};

	const typeLabels: Record<string, string> = {
		task_created: 'New Task',
		task_completed: 'Completed',
		task_updated: 'Updated',
		note_created: 'New Note',
		note_updated: 'Edited'
	};

	async function handleClick(entry: TimelineEntry) {
		if (entry.objectType === 'task') {
			selectedTaskId.set(entry.objectId);
			await goto(`/tasks?task=${entry.objectId}`);
		} else {
			selectedNoteId.set(entry.objectId);
			await goto(`/notes?note=${entry.objectId}`);
		}
	}

	function groupByDate(items: TimelineEntry[]): Map<string, TimelineEntry[]> {
		const groups = new Map<string, TimelineEntry[]>();
		for (const item of items) {
			const d = new Date(item.timestamp);
			const key = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
			if (!groups.has(key)) groups.set(key, []);
			groups.get(key)!.push(item);
		}
		return groups;
	}

	$: groupedEntries = groupByDate(filteredEntries);
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<p class="text-xs text-[rgb(var(--mv-muted))]">{filteredEntries.length} activity events</p>
		<div class="flex flex-wrap items-center gap-2">
			<input
				class="w-40 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-1.5 text-xs text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/60 focus:border-[rgb(var(--mv-accent))] focus:outline-none"
				placeholder="Search..."
				bind:value={searchQuery}
			/>

			<div class="flex rounded-lg border border-[rgb(var(--mv-border))] text-[10px]">
				<button
					class={`px-2 py-1 transition ${dateRange === 'all' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (dateRange = 'all')}
				>
					All time
				</button>
				<button
					class={`px-2 py-1 transition ${dateRange === 'today' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (dateRange = 'today')}
				>
					Today
				</button>
				<button
					class={`px-2 py-1 transition ${dateRange === 'week' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (dateRange = 'week')}
				>
					7 days
				</button>
				<button
					class={`px-2 py-1 transition ${dateRange === 'month' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (dateRange = 'month')}
				>
					30 days
				</button>
			</div>

			<div class="flex rounded-lg border border-[rgb(var(--mv-border))] text-[10px]">
				<button
					class={`px-3 py-1 transition ${filter === 'all' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (filter = 'all')}
				>
					All
				</button>
				<button
					class={`px-3 py-1 transition ${filter === 'tasks' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (filter = 'tasks')}
				>
					Tasks
				</button>
				<button
					class={`px-3 py-1 transition ${filter === 'notes' ? 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-text))]' : 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'}`}
					on:click={() => (filter = 'notes')}
				>
					Notes
				</button>
			</div>

			<button
				class={`rounded-lg border px-2 py-1 text-[10px] transition ${
					showFilters || hasActiveFilters
						? 'border-[rgb(var(--mv-accent))]/50 bg-[rgb(var(--mv-accent))]/20 text-[rgb(var(--mv-accent))]'
						: 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'
				}`}
				on:click={() => (showFilters = !showFilters)}
			>
				{showFilters ? 'Less' : 'More'}
			</button>

			{#if hasActiveFilters}
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]"
					on:click={resetFilters}
				>
					Reset
				</button>
			{/if}
		</div>
	</div>

	{#if showFilters}
		<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/50 p-3">
			<p class="mb-2 text-[10px] font-semibold uppercase tracking-wide text-[rgb(var(--mv-muted))]/60">Event Types</p>
			<div class="flex flex-wrap gap-2">
				{#each Object.entries(typeLabels) as [type, label]}
					<button
						class={`flex items-center gap-1.5 rounded-lg border px-2 py-1 text-[10px] transition ${
							eventTypes.has(type)
								? 'border-[rgb(var(--mv-accent))]/50 bg-[rgb(var(--mv-accent))]/10 text-[rgb(var(--mv-accent))]'
								: 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))]/60 hover:text-[rgb(var(--mv-muted))]'
						}`}
						on:click={() => toggleEventType(type)}
					>
						<span class={`h-2 w-2 rounded-full ${typeColors[type]}`}></span>
						{label}
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if filteredEntries.length === 0}
		<div class="rounded-xl border border-dashed border-[rgb(var(--mv-border))] p-12 text-center text-sm text-[rgb(var(--mv-muted))]">
			No activity yet. Create tasks or notes to see timeline events.
		</div>
	{:else}
		{#each [...groupedEntries] as [dateLabel, items] (dateLabel)}
			<div>
				<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-[rgb(var(--mv-muted))]/60">{dateLabel}</h3>
				<div class="relative ml-3 border-l border-[rgb(var(--mv-border))]">
					{#each items as entry (entry.id)}
						<button
							class="group flex w-full items-start gap-3 py-2 pl-4 text-left transition hover:bg-[rgb(var(--mv-panel-strong))]/20 rounded-r-lg"
							on:click={() => void handleClick(entry)}
						>
							<div class={`mt-1 h-2.5 w-2.5 -ml-[21px] shrink-0 rounded-full ${typeColors[entry.type] ?? 'bg-slate-500'}`}></div>
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2">
									<span class={`rounded px-1.5 py-0.5 text-[9px] font-medium ${typeColors[entry.type] ?? 'bg-slate-500'}/20 text-[rgb(var(--mv-muted))]`}>
										{typeLabels[entry.type] ?? entry.type}
									</span>
									<span class="text-[10px] text-[rgb(var(--mv-muted))]/60">{formatTimestamp(entry.timestamp)}</span>
								</div>
								<p class="mt-0.5 truncate text-xs text-[rgb(var(--mv-muted))] group-hover:text-[rgb(var(--mv-text))]">{entry.title}</p>
							</div>
						</button>
					{/each}
				</div>
			</div>
		{/each}
	{/if}
</div>
