<script lang="ts">
	import { resolve } from '$app/paths';
	import type { BriefingTask } from '$lib/api/briefing';
	import { completeTask } from '$lib/api/tasks';

	export let tasks: BriefingTask[] = [];
	export let title: string = 'Due Today';
	export let loading: boolean = false;

	function priorityColor(priority: number): string {
		if (priority <= 1) return 'bg-red-400';
		if (priority === 2) return 'bg-orange-400';
		if (priority === 3) return 'bg-yellow-400';
		return 'bg-slate-500';
	}

	function formatTime(dateStr: string | null): string {
		if (!dateStr) return '';
		const d = new Date(dateStr);
		return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	async function handleComplete(task: BriefingTask) {
		try {
			await completeTask(task.id);
			tasks = tasks.filter((t) => t.id !== task.id);
		} catch (e) {
			console.error('Failed to complete task', e);
		}
	}
</script>

<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
	<div class="flex items-center justify-between">
		<h3 class="text-sm font-semibold text-white">{title}</h3>
		<a href={resolve('/tasks')} class="text-[11px] text-sky-400 hover:text-sky-300">View all</a>
	</div>
	<div class="mt-3 space-y-2">
		{#if loading}
			<p class="text-xs text-slate-500">Loading...</p>
		{:else if tasks.length === 0}
			<p class="py-4 text-center text-xs text-slate-500">No tasks. You're all clear!</p>
		{:else}
			{#each tasks as task (task.id)}
				<div
					class="flex items-center justify-between rounded-lg border border-slate-800/60 px-3 py-2.5 transition hover:border-slate-700"
				>
					<div class="flex items-center gap-2.5 min-w-0">
						<button
							on:click={() => handleComplete(task)}
							class="h-4 w-4 flex-shrink-0 rounded border border-slate-600 hover:border-emerald-400 hover:bg-emerald-500/20 transition"
							title="Mark complete"
						></button>
						<span class="h-2 w-2 flex-shrink-0 rounded-full {priorityColor(task.priority)}"></span>
						<span class="truncate text-xs font-medium text-white">{task.title}</span>
					</div>
					<div class="flex items-center gap-2 flex-shrink-0">
						{#if task.due_at}
							<span class="text-[10px] text-slate-500">{formatTime(task.due_at)}</span>
						{/if}
						<span
							class="rounded-full px-2 py-0.5 text-[9px] uppercase tracking-wide {task.status === 'in_progress'
								? 'bg-blue-500/20 text-blue-300'
								: task.status === 'planned'
									? 'bg-violet-500/20 text-violet-300'
									: 'bg-slate-800 text-slate-400'}"
						>
							{task.status.replace('_', ' ')}
						</span>
					</div>
				</div>
			{/each}
		{/if}
	</div>
</div>
