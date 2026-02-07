<script lang="ts">
	import { onMount } from 'svelte';
	import { tasksStore, loadTasks, updateTaskOptimistic, completeTaskOptimistic, snoozeTaskOptimistic } from '$lib/stores/tasks';

	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { pushToast } from '$lib/stores/toast';
	import type { TaskRecord } from '$lib/db';
	import type { Note } from '$lib/api/notes';
	import type { TaskStatus } from '$lib/api/tasks';
	import { updateNote } from '$lib/api/notes';

	type InboxItem =
		| { type: 'task'; data: TaskRecord; created: string }
		| { type: 'note'; data: Note; created: string };

	let items: InboxItem[] = [];
	let loading = true;
	let processedCount = 0;
	let snoozeOpenId: string | null = null;

	$: {
		const taskItems: InboxItem[] = $tasksStore
			.filter((t) => t.status === 'inbox')
			.map((t) => ({ type: 'task' as const, data: t, created: t.created_at }));
		const noteItems: InboxItem[] = $notesStore
			.filter((n) => !n.tags || n.tags.length === 0)
			.map((n) => ({ type: 'note' as const, data: n, created: n.created_at }));
		items = [...taskItems, ...noteItems].sort(
			(a, b) => new Date(b.created).getTime() - new Date(a.created).getTime()
		);
	}

	onMount(async () => {
		await Promise.all([loadTasks(), loadNotes()]);
		loading = false;
	});

	async function moveTask(taskId: string, status: TaskStatus) {
		try {
			await updateTaskOptimistic(taskId, { status });
			processedCount++;
			pushToast(`Moved to ${status.replace('_', ' ')}`, 'success');
		} catch {
			pushToast('Failed to update task', 'danger');
		}
	}

	async function markDone(taskId: string) {
		try {
			const undoId = await completeTaskOptimistic(taskId);
			processedCount++;
			pushToast('Marked done', 'success', 3000, undoId);
		} catch {
			pushToast('Failed to complete task', 'danger');
		}
	}

	async function handleSnooze(taskId: string, option: 'tomorrow' | 'next_week' | 'custom') {
		snoozeOpenId = null;
		try {
			if (option === 'tomorrow') {
				const tomorrow = new Date();
				tomorrow.setDate(tomorrow.getDate() + 1);
				tomorrow.setHours(9, 0, 0, 0);
				await snoozeTaskOptimistic(taskId, tomorrow.toISOString());
				pushToast('Snoozed until tomorrow', 'success');
			} else if (option === 'next_week') {
				const nextWeek = new Date();
				const daysUntilMonday = (8 - nextWeek.getDay()) % 7 || 7;
				nextWeek.setDate(nextWeek.getDate() + daysUntilMonday);
				nextWeek.setHours(9, 0, 0, 0);
				await snoozeTaskOptimistic(taskId, nextWeek.toISOString());
				pushToast('Snoozed until next week', 'success');
			} else {
				const input = prompt('Snooze until (YYYY-MM-DD):');
				if (!input?.trim()) return;
				const date = new Date(input.trim());
				date.setHours(9, 0, 0, 0);
				await snoozeTaskOptimistic(taskId, date.toISOString());
				pushToast(`Snoozed until ${input.trim()}`, 'success');
			}
			processedCount++;
		} catch {
			pushToast('Failed to snooze task', 'danger');
		}
	}

	async function tagNote(noteId: string) {
		const tag = prompt('Enter a tag for this note:');
		if (!tag?.trim()) return;
		try {
			const note = $notesStore.find((n) => n.id === noteId);
			if (!note) return;
			const existingTags = note.tags ?? [];
			await updateNote(noteId, { tags: [...existingTags, tag.trim()] });
			await loadNotes();
			processedCount++;
			pushToast(`Tagged with "${tag.trim()}"`, 'success');
		} catch {
			pushToast('Failed to tag note', 'danger');
		}
	}

	async function archiveNote(noteId: string) {
		try {
			await updateNote(noteId, { tags: ['archived'] });
			await loadNotes();
			processedCount++;
			pushToast('Note archived', 'success');
		} catch {
			pushToast('Failed to archive', 'danger');
		}
	}

	function relativeTime(dateStr: string): string {
		const now = Date.now();
		const date = new Date(dateStr).getTime();
		const diffMin = Math.floor((now - date) / 60000);
		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;
		const diffDay = Math.floor(diffHr / 24);
		return `${diffDay}d ago`;
	}

	const STATUS_ACTIONS: Array<{ status: TaskStatus; label: string; color: string }> = [
		{ status: 'planned', label: 'Plan', color: 'border-blue-500/30 text-blue-300 hover:bg-blue-500/10' },
		{ status: 'in_progress', label: 'Start', color: 'border-amber-500/30 text-amber-300 hover:bg-amber-500/10' }
	];
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Smart Inbox</h2>
			<p class="text-xs text-slate-400">
				{items.length} items to triage
				{#if processedCount > 0}
					&middot; {processedCount} processed this session
				{/if}
			</p>
		</div>
	</div>

	<div class="mt-4 flex flex-col gap-3">
		{#if loading}
			<div class="rounded-xl border border-slate-800 p-6 text-center text-xs text-slate-400">
				Loading inbox...
			</div>
		{:else if items.length === 0}
			<div class="rounded-xl border border-dashed border-slate-800 p-8 text-center">
				<div class="flex justify-center">
					<div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-500/10 text-emerald-300">
						<svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 13l4 4L19 7" />
						</svg>
					</div>
				</div>
				<h3 class="mt-3 text-sm font-semibold text-white">Inbox zero</h3>
				<p class="mt-1 text-xs text-slate-400">
					All items have been triaged. New captures will appear here.
				</p>
			</div>
		{:else}
			{#each items as item (item.type + '-' + item.data.id)}
				<div class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-4 transition hover:border-slate-700">
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<span class="rounded px-1.5 py-0.5 text-[9px] font-medium uppercase {item.type === 'task'
									? 'bg-violet-500/20 text-violet-300'
									: 'bg-sky-500/20 text-sky-300'}">
									{item.type}
								</span>
								<span class="text-[10px] text-slate-500">{relativeTime(item.created)}</span>
							</div>
							<h4 class="mt-1 text-sm font-medium text-white">
								{item.type === 'task' ? item.data.title : item.data.title ?? 'Untitled Note'}
							</h4>
							{#if item.type === 'task' && item.data.description}
								<p class="mt-1 line-clamp-2 text-[11px] text-slate-400">{item.data.description}</p>
							{/if}
							{#if item.type === 'note' && item.data.markdown}
								<p class="mt-1 line-clamp-2 text-[11px] text-slate-400">{item.data.markdown.slice(0, 150)}</p>
							{/if}
						</div>
					</div>

					<div class="mt-3 flex flex-wrap gap-1.5">
						{#if item.type === 'task'}
							{#each STATUS_ACTIONS as action (action.status)}
								<button
									class="rounded-lg border px-2.5 py-1 text-[10px] font-medium transition {action.color}"
									on:click={() => moveTask(item.data.id, action.status)}
								>
									{action.label}
								</button>
							{/each}
							<div class="relative">
								<button
									class="rounded-lg border border-orange-500/30 px-2.5 py-1 text-[10px] font-medium text-orange-300 transition hover:bg-orange-500/10"
									on:click={() => { snoozeOpenId = snoozeOpenId === item.data.id ? null : item.data.id; }}
								>
									Snooze
								</button>
								{#if snoozeOpenId === item.data.id}
									<div class="absolute left-0 top-full z-20 mt-1 w-36 rounded-lg border border-slate-700 bg-slate-800 py-1 shadow-xl">
										<button
											class="w-full px-3 py-1.5 text-left text-[10px] text-slate-300 hover:bg-slate-700"
											on:click={() => handleSnooze(item.data.id, 'tomorrow')}
										>
											Tomorrow
										</button>
										<button
											class="w-full px-3 py-1.5 text-left text-[10px] text-slate-300 hover:bg-slate-700"
											on:click={() => handleSnooze(item.data.id, 'next_week')}
										>
											Next Week
										</button>
										<button
											class="w-full px-3 py-1.5 text-left text-[10px] text-slate-300 hover:bg-slate-700"
											on:click={() => handleSnooze(item.data.id, 'custom')}
										>
											Custom...
										</button>
									</div>
								{/if}
							</div>
							<button
								class="rounded-lg border border-emerald-500/30 px-2.5 py-1 text-[10px] font-medium text-emerald-300 transition hover:bg-emerald-500/10"
								on:click={() => markDone(item.data.id)}
							>
								Done
							</button>
							<a
								href="/tasks?task={item.data.id}"
								class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 transition hover:bg-slate-800"
							>
								Open
							</a>
						{:else}
							<button
								class="rounded-lg border border-sky-500/30 px-2.5 py-1 text-[10px] font-medium text-sky-300 transition hover:bg-sky-500/10"
								on:click={() => tagNote(item.data.id)}
							>
								Tag
							</button>
							<button
								class="rounded-lg border border-slate-600 px-2.5 py-1 text-[10px] text-slate-300 transition hover:bg-slate-800"
								on:click={() => archiveNote(item.data.id)}
							>
								Archive
							</button>
							<a
								href="/notes?note={item.data.id}"
								class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 transition hover:bg-slate-800"
							>
								Open
							</a>
						{/if}
					</div>
				</div>
			{/each}
		{/if}
	</div>
</div>
