<script lang="ts">
	import type { TaskRecord } from '$lib/db';
	import { createEventDispatcher } from 'svelte';

	export let task: TaskRecord;
	export let active = false;
	export let blocked = false;
	export let blockedReason = '';

	const dispatch = createEventDispatcher<{ select: string; complete: string }>();

	const priorityColors: Record<number, string> = {
		1: 'bg-red-400',
		2: 'bg-orange-400',
		3: 'bg-yellow-400',
		4: 'bg-blue-400',
		5: 'bg-slate-500'
	};

	const priorityLabels: Record<number, string> = {
		1: 'Critical',
		2: 'High',
		3: 'Medium',
		4: 'Low',
		5: 'Minimal'
	};

	const statusColors: Record<string, string> = {
		inbox: 'bg-slate-700 text-slate-300',
		planned: 'bg-violet-500/20 text-violet-300',
		in_progress: 'bg-blue-500/20 text-blue-300',
		waiting: 'bg-amber-500/20 text-amber-300',
		review: 'bg-cyan-500/20 text-cyan-300',
		done: 'bg-emerald-500/20 text-emerald-300'
	};

	$: priorityDot = priorityColors[task.priority] ?? priorityColors[3];
	$: statusClass = statusColors[task.status] ?? statusColors['inbox'];
	$: isDone = task.status === 'done';
	$: isPinned = task.metadata?.pinned === true;
	$: isOverdue = !isDone && task.due_at && new Date(task.due_at) < new Date();
	$: showBlocked = blocked && !isDone;

	// Subtask progress
	type Subtask = { id: string; title: string; done: boolean };
	$: subtasks = (task.metadata?.subtasks as Subtask[] | undefined) ?? [];
	$: hasSubtasks = subtasks.length > 0;
	$: completedSubtasks = subtasks.filter((s) => s.done).length;

	function formatDue(due: string): string {
		const d = new Date(due);
		const now = new Date();
		const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const tomorrow = new Date(today);
		tomorrow.setDate(tomorrow.getDate() + 1);
		const dayAfter = new Date(today);
		dayAfter.setDate(dayAfter.getDate() + 2);

		if (d >= today && d < tomorrow) return 'Today';
		if (d >= tomorrow && d < dayAfter) return 'Tomorrow';

		return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	function handleComplete(e: MouseEvent) {
		e.stopPropagation();
		dispatch('complete', task.id);
	}
</script>

<div
	role="button"
	tabindex="0"
	class={`w-full rounded-xl border px-4 py-3 text-left transition ${
		active
			? 'border-sky-500/60 bg-sky-500/10'
			: 'border-slate-900 bg-slate-900/40 hover:border-slate-700'
	} ${isDone ? 'opacity-60' : ''}`}
	aria-pressed={active}
	on:click={() => dispatch('select', task.id)}
	on:keydown={(event) => {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			dispatch('select', task.id);
		}
	}}
>
	<div class="flex items-center gap-3">
		<!-- Complete checkbox -->
		<button
			class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-md border transition {isDone
				? 'border-emerald-500 bg-emerald-500/20 text-emerald-300'
				: 'border-slate-700 hover:border-slate-500'}"
			on:click={handleComplete}
			role="checkbox"
			aria-checked={isDone}
			aria-label={isDone ? 'Mark incomplete' : 'Mark complete'}
		>
			{#if isDone}
				<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
				</svg>
			{/if}
		</button>

		<!-- Priority dot -->
		<span
			class="h-2 w-2 flex-shrink-0 rounded-full {priorityDot}"
			aria-label="Priority: {priorityLabels[task.priority] ?? 'Medium'}"
			title="Priority: {priorityLabels[task.priority] ?? 'Medium'}"
		></span>

		{#if isPinned}
			<svg class="h-3.5 w-3.5 flex-shrink-0 text-amber-400" fill="currentColor" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
			</svg>
		{/if}

		<!-- Title + description -->
		<div class="min-w-0 flex-1">
			<h3 class="truncate text-sm font-semibold {isDone ? 'text-slate-500 line-through' : 'text-white'}">
				{task.title}
			</h3>
			{#if task.description && !isDone}
				<p class="mt-0.5 line-clamp-1 text-[11px] text-slate-500">{task.description}</p>
			{/if}
		</div>

		<!-- Status badge -->
		<span
			class="flex-shrink-0 rounded-full px-2 py-0.5 text-[9px] uppercase tracking-wide {statusClass}"
			aria-label="Status: {task.status.replace('_', ' ')}"
		>
			{task.status.replace('_', ' ')}
		</span>
	</div>

	<!-- Bottom row: due date, labels, pending -->
	<div class="mt-2 flex items-center gap-2 pl-10">
		{#if showBlocked}
			<span
				class="rounded-full bg-red-500/20 px-2 py-0.5 text-[9px] text-red-200"
				title={blockedReason}
			>
				Blocked
			</span>
		{/if}
		{#if task.due_at}
			<span
				class="text-[11px] {isOverdue ? 'font-medium text-red-400' : 'text-slate-500'}"
			>
				{isOverdue ? 'Overdue: ' : ''}{formatDue(task.due_at)}
			</span>
		{/if}
		{#if task.labels?.length}
			<div class="flex gap-1">
				{#each task.labels.slice(0, 3) as label (label)}
					<span class="rounded-full bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{label}</span>
				{/each}
				{#if task.labels.length > 3}
					<span class="text-[9px] text-slate-600">+{task.labels.length - 3}</span>
				{/if}
			</div>
		{/if}
		{#if task.estimate_min}
			<span class="text-[10px] text-slate-600">{task.estimate_min}m</span>
		{/if}
		{#if hasSubtasks}
			<span class="flex items-center gap-1 text-[10px] {completedSubtasks === subtasks.length ? 'text-emerald-400' : 'text-slate-500'}">
				<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
				</svg>
				{completedSubtasks}/{subtasks.length}
			</span>
		{/if}
		{#if task.pending}
			<span class="ml-auto rounded-full bg-amber-500/20 px-2 py-0.5 text-[9px] text-amber-200">Pending sync</span>
		{/if}
	</div>
</div>
