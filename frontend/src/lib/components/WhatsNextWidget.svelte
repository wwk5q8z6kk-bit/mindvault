<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { prioritizeTasks, type PrioritizedTaskItem } from '$lib/api/ai';
	import { completeTaskOptimistic, updateTaskOptimistic } from '$lib/stores/tasks';
	import { pushToast } from '$lib/stores/toast';
	import { activeNamespace } from '$lib/stores/namespace';
	import { get } from 'svelte/store';

	let loading = true;
	let error: string | null = null;
	let nextTask: PrioritizedTaskItem | null = null;

	const priorityColors: Record<number, string> = {
		1: 'text-red-400',
		2: 'text-orange-400',
		3: 'text-amber-400',
		4: 'text-blue-400',
		5: 'text-slate-400'
	};

	onMount(async () => {
		await loadNextTask();
	});

	async function loadNextTask() {
		loading = true;
		error = null;
		try {
			const namespace = get(activeNamespace);
			const result = await prioritizeTasks({
				namespace: namespace ?? undefined,
				limit: 1,
				include_done: false,
				statuses: ['inbox', 'planned', 'in_progress']
			});
			nextTask = result.items[0] ?? null;
		} catch (e) {
			error = 'Could not determine next task';
			console.error(e);
		} finally {
			loading = false;
		}
	}

	async function startTask() {
		if (!nextTask) return;
		try {
			await updateTaskOptimistic(nextTask.task.id, { status: 'in_progress' });
			pushToast('Task started', 'success');
			goto(`/focus?task=${nextTask.task.id}`);
		} catch {
			pushToast('Failed to start task', 'danger');
		}
	}

	async function completeTask() {
		if (!nextTask) return;
		try {
			await completeTaskOptimistic(nextTask.task.id);
			pushToast('Task completed', 'success');
			await loadNextTask();
		} catch {
			pushToast('Failed to complete task', 'danger');
		}
	}

	function formatDue(dueAt: string | null): string | null {
		if (!dueAt) return null;
		const due = new Date(dueAt);
		const now = new Date();
		const diff = due.getTime() - now.getTime();
		const days = Math.floor(diff / (1000 * 60 * 60 * 24));

		if (days < 0) return 'Overdue';
		if (days === 0) return 'Today';
		if (days === 1) return 'Tomorrow';
		if (days < 7) return `${days} days`;
		return due.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}
</script>

<div class="rounded-xl border border-emerald-500/20 bg-gradient-to-br from-emerald-500/5 to-slate-900/40 p-5">
	<div class="flex items-center justify-between">
		<h3 class="text-sm font-semibold text-white">What's Next</h3>
		<button
			class="text-[10px] text-slate-500 hover:text-slate-300"
			on:click={loadNextTask}
			disabled={loading}
		>
			Refresh
		</button>
	</div>

	{#if loading}
		<div class="mt-4 flex items-center gap-3">
			<div class="h-8 w-8 animate-spin rounded-full border-2 border-emerald-500 border-t-transparent"></div>
			<span class="text-xs text-slate-400">Finding your top priority...</span>
		</div>
	{:else if error}
		<div class="mt-4">
			<p class="text-xs text-red-300">{error}</p>
			<button
				class="mt-2 text-[10px] text-sky-400 hover:text-sky-300"
				on:click={loadNextTask}
			>
				Try again
			</button>
		</div>
	{:else if !nextTask}
		<div class="mt-4 rounded-lg border border-dashed border-slate-700 p-4 text-center">
			<p class="text-xs text-slate-400">No tasks to prioritize</p>
			<a href="/tasks" class="mt-2 inline-block text-[10px] text-sky-400 hover:text-sky-300">
				Add a task
			</a>
		</div>
	{:else}
		<div class="mt-4">
			<div class="flex items-start gap-3">
				<div class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-sm font-bold text-emerald-300">
					1
				</div>
				<div class="min-w-0 flex-1">
					<h4 class="text-sm font-medium text-white">{nextTask.task.title}</h4>
					<div class="mt-1 flex flex-wrap items-center gap-2 text-[10px]">
						<span class={priorityColors[nextTask.task.priority]}>P{nextTask.task.priority}</span>
						{#if nextTask.task.due_at}
							{@const dueText = formatDue(nextTask.task.due_at)}
							<span class={dueText === 'Overdue' ? 'text-red-400' : 'text-slate-400'}>
								{dueText}
							</span>
						{/if}
						{#if nextTask.task.estimate_min}
							<span class="text-violet-300">{nextTask.task.estimate_min}m</span>
						{/if}
					</div>
					{#if nextTask.reason}
						<p class="mt-2 text-[11px] text-slate-500 italic">"{nextTask.reason}"</p>
					{/if}
				</div>
			</div>

			<div class="mt-4 flex gap-2">
				<button
					class="flex-1 rounded-lg bg-emerald-500 py-2 text-xs font-semibold text-white hover:bg-emerald-400"
					on:click={startTask}
				>
					Start Focus
				</button>
				<button
					class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 hover:border-slate-500"
					on:click={completeTask}
				>
					Done
				</button>
				<a
					href={`/tasks?task=${nextTask.task.id}`}
					class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 hover:border-slate-500"
				>
					View
				</a>
			</div>
		</div>
	{/if}

	<a href="/plan" class="mt-4 inline-flex items-center gap-1 text-[10px] text-sky-400 hover:text-sky-300">
		<span>See full daily plan</span>
		<span>&rarr;</span>
	</a>
</div>
