<script lang="ts">
	import type { TaskRecord } from '$lib/db';
	import type { TaskStatus } from '$lib/api/tasks';
	import AttachmentsPanel from '$lib/components/AttachmentsPanel.svelte';
	import { createEventDispatcher } from 'svelte';

	export let task: TaskRecord | null = null;
	export let dependencyTasks: TaskRecord[] = [];
	export let dependentTasks: TaskRecord[] = [];
	export let blockedReason: string | null = null;

	interface Subtask {
		id: string;
		title: string;
		done: boolean;
	}

	const dispatch = createEventDispatcher<{
		edit: void;
		complete: void;
		statusChange: TaskStatus;
		openTask: string;
		togglePin: void;
		subtasksUpdate: Subtask[];
	}>();

	// Subtask state
	$: subtasks = ((task?.metadata?.subtasks as Subtask[] | undefined) ?? []);
	$: subtaskProgress = subtasks.length > 0
		? Math.round((subtasks.filter((s) => s.done).length / subtasks.length) * 100)
		: 0;
	let newSubtaskTitle = '';

	function addSubtask() {
		if (!newSubtaskTitle.trim()) return;
		const updated: Subtask[] = [
			...subtasks,
			{ id: crypto.randomUUID(), title: newSubtaskTitle.trim(), done: false }
		];
		dispatch('subtasksUpdate', updated);
		newSubtaskTitle = '';
	}

	function toggleSubtask(id: string) {
		const updated = subtasks.map((s) => (s.id === id ? { ...s, done: !s.done } : s));
		dispatch('subtasksUpdate', updated);
	}

	function deleteSubtask(id: string) {
		const updated = subtasks.filter((s) => s.id !== id);
		dispatch('subtasksUpdate', updated);
	}

	function moveSubtask(index: number, direction: 'up' | 'down') {
		const newIndex = direction === 'up' ? index - 1 : index + 1;
		if (newIndex < 0 || newIndex >= subtasks.length) return;
		const updated = [...subtasks];
		[updated[index], updated[newIndex]] = [updated[newIndex], updated[index]];
		dispatch('subtasksUpdate', updated);
	}

	$: isPinned = task?.metadata?.pinned === true;

	const statusOptions: TaskStatus[] = [
		'inbox',
		'planned',
		'in_progress',
		'waiting',
		'review',
		'done'
	];

	const statusColors: Record<string, string> = {
		inbox: 'bg-slate-700 text-slate-300',
		planned: 'bg-violet-500/20 text-violet-300',
		in_progress: 'bg-blue-500/20 text-blue-300',
		waiting: 'bg-amber-500/20 text-amber-300',
		review: 'bg-cyan-500/20 text-cyan-300',
		done: 'bg-emerald-500/20 text-emerald-300'
	};

	const priorityLabels: Record<number, { label: string; color: string }> = {
		1: { label: 'Critical', color: 'text-red-400' },
		2: { label: 'High', color: 'text-orange-400' },
		3: { label: 'Medium', color: 'text-yellow-400' },
		4: { label: 'Low', color: 'text-blue-400' },
		5: { label: 'Minimal', color: 'text-slate-400' }
	};

	$: isDone = task?.status === 'done';
	$: prio = task ? (priorityLabels[task.priority] ?? priorityLabels[3]) : priorityLabels[3];
	$: statusClass = task ? (statusColors[task.status] ?? statusColors['inbox']) : '';
	$: sourceNoteId =
		task && typeof task.metadata?.source_note_id === 'string' ? task.metadata.source_note_id : null;
	$: sourceNoteTitle =
		task && typeof task.metadata?.source_note_title === 'string'
			? task.metadata.source_note_title
			: null;

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function relativeTime(dateStr: string): string {
		const diff = Date.now() - new Date(dateStr).getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 1) return 'just now';
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}
</script>

<div class="flex h-full flex-col rounded-2xl border border-slate-900 bg-slate-900/40 p-6">
	{#if task}
		<div class="flex items-start justify-between gap-4">
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-2">
					<h2 class="text-xl font-semibold text-white {isDone ? 'line-through opacity-60' : ''}">
						{task.title}
					</h2>
					<button
						class="flex h-6 w-6 items-center justify-center rounded-md transition {isPinned
							? 'text-amber-400 hover:text-amber-300'
							: 'text-slate-600 hover:text-slate-400'}"
						on:click={() => dispatch('togglePin')}
						aria-label={isPinned ? 'Unpin task' : 'Pin task'}
						title={isPinned ? 'Unpin task' : 'Pin task'}
					>
						<svg class="h-4 w-4" fill={isPinned ? 'currentColor' : 'none'} stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
						</svg>
					</button>
				</div>
				<div class="mt-2 flex items-center gap-2">
					<span class="rounded-full px-2.5 py-1 text-[10px] uppercase tracking-wide {statusClass}">
						{task.status.replace('_', ' ')}
					</span>
					<span class="text-[10px] {prio.color}">P{task.priority} · {prio.label}</span>
					{#if task.pending}
						<span class="rounded-full bg-amber-500/20 px-2 py-0.5 text-[10px] text-amber-200">
							Pending sync
						</span>
					{/if}
				</div>
			</div>
			<div class="flex gap-2 flex-shrink-0">
				<button
					class="rounded-lg border border-slate-800 px-3 py-1.5 text-xs text-slate-200 hover:border-slate-600"
					on:click={() => dispatch('edit')}
				>
					Edit
				</button>
				<button
					class="rounded-lg px-3 py-1.5 text-xs font-medium {isDone
						? 'border border-slate-800 text-slate-300 hover:border-slate-600'
						: 'bg-emerald-500/20 text-emerald-200 hover:bg-emerald-500/30'}"
					on:click={() => dispatch('complete')}
				>
					{isDone ? 'Reopen' : 'Mark done'}
				</button>
			</div>
		</div>

		<!-- Quick status change -->
		<div class="mt-4 flex items-center gap-1.5">
			<span class="text-[10px] text-slate-500">Status:</span>
			{#each statusOptions as opt (opt)}
				<button
					class="rounded-full px-2 py-0.5 text-[9px] transition {task.status === opt
						? statusColors[opt]
						: 'border border-slate-800 text-slate-500 hover:border-slate-700 hover:text-slate-400'}"
					on:click={() => dispatch('statusChange', opt)}
				>
					{opt.replace('_', ' ')}
				</button>
			{/each}
		</div>

			<div class="mt-6 space-y-5 text-sm text-slate-300">
			<!-- Description -->
			<div>
				<p class="text-[10px] uppercase tracking-wide text-slate-500">Description</p>
				<p class="mt-2 whitespace-pre-wrap text-slate-200">{task.description || '—'}</p>
			</div>

			<!-- Subtasks / Checklist -->
			<div>
				<div class="flex items-center justify-between">
					<p class="text-[10px] uppercase tracking-wide text-slate-500">
						Subtasks {#if subtasks.length > 0}<span class="text-slate-400">({subtasks.filter(s => s.done).length}/{subtasks.length})</span>{/if}
					</p>
					{#if subtasks.length > 0}
						<div class="flex items-center gap-2">
							<div class="h-1.5 w-20 rounded-full bg-slate-800">
								<div
									class="h-1.5 rounded-full transition-all {subtaskProgress === 100 ? 'bg-emerald-500' : 'bg-sky-500'}"
									style="width: {subtaskProgress}%"
								></div>
							</div>
							<span class="text-[10px] text-slate-500">{subtaskProgress}%</span>
						</div>
					{/if}
				</div>
				<div class="mt-3 space-y-1.5">
					{#each subtasks as subtask, index (subtask.id)}
						<div class="group flex items-center gap-2 rounded-lg border border-slate-800/60 bg-slate-900/40 px-3 py-2">
							<button
								class="flex h-4 w-4 flex-shrink-0 items-center justify-center rounded border transition {subtask.done
									? 'border-emerald-500 bg-emerald-500/20 text-emerald-300'
									: 'border-slate-600 hover:border-slate-500'}"
								on:click={() => toggleSubtask(subtask.id)}
								aria-label={subtask.done ? 'Mark incomplete' : 'Mark complete'}
							>
								{#if subtask.done}
									<svg class="h-2.5 w-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
									</svg>
								{/if}
							</button>
							<span class="flex-1 text-xs {subtask.done ? 'text-slate-500 line-through' : 'text-slate-200'}">
								{subtask.title}
							</span>
							<div class="hidden items-center gap-1 group-hover:flex">
								{#if index > 0}
									<button
										class="rounded p-0.5 text-slate-500 hover:bg-slate-800 hover:text-slate-300"
										on:click={() => moveSubtask(index, 'up')}
										aria-label="Move up"
									>
										<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
										</svg>
									</button>
								{/if}
								{#if index < subtasks.length - 1}
									<button
										class="rounded p-0.5 text-slate-500 hover:bg-slate-800 hover:text-slate-300"
										on:click={() => moveSubtask(index, 'down')}
										aria-label="Move down"
									>
										<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
										</svg>
									</button>
								{/if}
								<button
									class="rounded p-0.5 text-slate-500 hover:bg-red-500/20 hover:text-red-300"
									on:click={() => deleteSubtask(subtask.id)}
									aria-label="Delete subtask"
								>
									<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							</div>
						</div>
					{/each}
				</div>
				<form class="mt-2 flex gap-2" on:submit|preventDefault={addSubtask}>
					<input
						type="text"
						bind:value={newSubtaskTitle}
						placeholder="Add a subtask..."
						class="flex-1 rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-1.5 text-xs text-white placeholder-slate-600"
					/>
					<button
						type="submit"
						class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-400 hover:border-slate-600 hover:text-slate-300"
						disabled={!newSubtaskTitle.trim()}
					>
						Add
					</button>
				</form>
			</div>

			<!-- Grid details -->
				<div class="grid gap-4 md:grid-cols-2">
				<div>
					<p class="text-[10px] uppercase tracking-wide text-slate-500">Due</p>
					{#if task.due_at}
						{@const isOverdue = !isDone && new Date(task.due_at) < new Date()}
						<p class="mt-1 text-xs {isOverdue ? 'font-medium text-red-400' : ''}">
							{formatDate(task.due_at)}
							{#if isOverdue}
								<span class="ml-1 text-[10px]">(overdue)</span>
							{/if}
						</p>
					{:else}
						<p class="mt-1 text-xs text-slate-500">—</p>
					{/if}
				</div>
				<div>
					<p class="text-[10px] uppercase tracking-wide text-slate-500">Estimate</p>
					<p class="mt-1 text-xs">
						{task.estimate_min ? `${task.estimate_min} minutes` : '—'}
					</p>
				</div>
				<div>
					<p class="text-[10px] uppercase tracking-wide text-slate-500">Assignee</p>
					<p class="mt-1 text-xs">{task.assignee || '—'}</p>
				</div>
					<div>
						<p class="text-[10px] uppercase tracking-wide text-slate-500">Recurrence</p>
						<p class="mt-1 text-xs">{task.recurrence || '—'}</p>
					</div>
					<div>
						<p class="text-[10px] uppercase tracking-wide text-slate-500">Source Note</p>
						{#if sourceNoteId}
							<a
								class="mt-1 inline-flex rounded-full border border-sky-500/30 px-2 py-0.5 text-[10px] text-sky-200 hover:bg-sky-500/10"
								href={`/notes?note=${encodeURIComponent(sourceNoteId)}`}
							>
								{sourceNoteTitle || sourceNoteId}
							</a>
						{:else if sourceNoteTitle}
							<p class="mt-1 text-xs text-slate-300">{sourceNoteTitle}</p>
						{:else}
							<p class="mt-1 text-xs text-slate-500">—</p>
						{/if}
					</div>
					<div>
						<p class="text-[10px] uppercase tracking-wide text-slate-500">Dependencies</p>
						{#if dependencyTasks.length > 0}
							<div class="mt-1 flex flex-wrap gap-1.5">
								{#each dependencyTasks as dep (dep.id)}
									<button
										class="rounded-full border border-slate-700 px-2 py-0.5 text-[10px] text-slate-300 hover:border-sky-500/50 hover:text-sky-200"
										on:click={() => dispatch('openTask', dep.id)}
									>
										{dep.title}
									</button>
								{/each}
							</div>
						{:else}
							<p class="mt-1 text-xs text-slate-500">None</p>
						{/if}
					</div>
					<div>
						<p class="text-[10px] uppercase tracking-wide text-slate-500">Blocked By</p>
						<p class="mt-1 text-xs {blockedReason ? 'text-red-300' : 'text-slate-500'}">
							{blockedReason || 'No blockers'}
						</p>
					</div>
					<div>
						<p class="text-[10px] uppercase tracking-wide text-slate-500">Dependents</p>
						{#if dependentTasks.length > 0}
							<div class="mt-1 flex flex-wrap gap-1.5">
								{#each dependentTasks as dep (dep.id)}
									<button
										class="rounded-full border border-slate-700 px-2 py-0.5 text-[10px] text-slate-300 hover:border-sky-500/50 hover:text-sky-200"
										on:click={() => dispatch('openTask', dep.id)}
									>
										{dep.title}
									</button>
								{/each}
							</div>
						{:else}
							<p class="mt-1 text-xs text-slate-500">None</p>
						{/if}
					</div>
				</div>

			<!-- Labels -->
			<div>
				<p class="text-[10px] uppercase tracking-wide text-slate-500">Labels</p>
				<div class="mt-2 flex flex-wrap gap-2">
					{#if task.labels?.length}
						{#each task.labels as label (label)}
							<span class="rounded-full bg-slate-800 px-2.5 py-1 text-xs text-slate-200">{label}</span>
						{/each}
					{:else}
						<span class="text-xs text-slate-500">No labels</span>
					{/if}
				</div>
			</div>

			<div>
				<AttachmentsPanel
					nodeId={task.id}
					title="Attachments"
					description="Files linked to this task."
				/>
			</div>

			<!-- Timestamps -->
			<div class="border-t border-slate-800/60 pt-4">
				<div class="flex flex-wrap gap-x-6 gap-y-1 text-[10px] text-slate-500">
					<span>Created {relativeTime(task.created_at)} · {formatDate(task.created_at)}</span>
					<span>Updated {relativeTime(task.updated_at)} · {formatDate(task.updated_at)}</span>
					{#if task.completed_at}
						<span>Completed {relativeTime(task.completed_at)} · {formatDate(task.completed_at)}</span>
					{/if}
				</div>
			</div>
		</div>
	{:else}
		<div
			class="flex flex-1 flex-col items-center justify-center text-center text-sm text-slate-400"
		>
			<div class="rounded-xl bg-slate-800/30 px-6 py-8">
				<p class="font-medium text-slate-300">No task selected</p>
				<p class="mt-1 text-xs text-slate-500">Click a task to see its details.</p>
			</div>
		</div>
	{/if}
</div>
