<script lang="ts">
	import type { TaskRecord } from '$lib/db';
	import { createEventDispatcher } from 'svelte';

	export let task: TaskRecord;
	export let active = false;

	const dispatch = createEventDispatcher<{ select: string }>();

	const priorityColors: Record<number, string> = {
		1: 'bg-red-500/20 text-red-300',
		2: 'bg-orange-500/20 text-orange-300',
		3: 'bg-sky-500/20 text-sky-300',
		4: 'bg-slate-500/20 text-[rgb(var(--mv-muted))]',
		5: 'bg-slate-700/20 text-[rgb(var(--mv-muted))]'
	};

	$: isOverdue =
		task.due_at && task.status !== 'done' && new Date(task.due_at) < new Date();

	function formatDue(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diff = d.getTime() - now.getTime();
		const days = Math.ceil(diff / 86400000);
		if (days === 0) return 'Today';
		if (days === 1) return 'Tomorrow';
		if (days === -1) return 'Yesterday';
		if (days < -1) return `${Math.abs(days)}d overdue`;
		if (days <= 7) return `${days}d`;
		return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	function onDragStart(event: DragEvent) {
		if (!event.dataTransfer) return;
		event.dataTransfer.effectAllowed = 'move';
		event.dataTransfer.setData('text/plain', task.id);
	}
</script>

<button
	class={`group w-full cursor-grab rounded-xl border px-3 py-2.5 text-left transition active:cursor-grabbing ${
		active
			? 'border-sky-500/60 bg-sky-500/10'
			: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/60 hover:border-[rgb(var(--mv-border))]'
	}`}
	draggable="true"
	on:dragstart={onDragStart}
	on:click={() => dispatch('select', task.id)}
	aria-selected={active}
>
	<div class="flex items-start justify-between gap-2">
		<h4 class="text-xs font-semibold leading-snug text-[rgb(var(--mv-text))]">{task.title}</h4>
		<span
			class={`shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold ${priorityColors[task.priority] ?? priorityColors[3]}`}
		>
			P{task.priority}
		</span>
	</div>

	{#if task.labels.length > 0}
		<div class="mt-1.5 flex flex-wrap gap-1">
			{#each task.labels.slice(0, 3) as label}
				<span class="rounded-full bg-[rgb(var(--mv-panel-strong))] px-1.5 py-0.5 text-[9px] text-[rgb(var(--mv-muted))]">
					{label}
				</span>
			{/each}
			{#if task.labels.length > 3}
				<span class="text-[9px] text-[rgb(var(--mv-muted))]/60">+{task.labels.length - 3}</span>
			{/if}
		</div>
	{/if}

	<div class="mt-2 flex items-center gap-2 text-[10px]">
		{#if task.due_at}
			<span class={isOverdue ? 'font-medium text-red-400' : 'text-[rgb(var(--mv-muted))]/60'}>
				{formatDue(task.due_at)}
			</span>
		{/if}
		{#if task.estimate_min}
			<span class="text-slate-600">{task.estimate_min}m</span>
		{/if}
		{#if task.pending}
			<span class="rounded-full bg-amber-500/20 px-1.5 py-0.5 text-amber-200">sync</span>
		{/if}
	</div>
</button>
