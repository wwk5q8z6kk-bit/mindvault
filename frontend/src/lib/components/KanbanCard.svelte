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

	$: isOverdue = task.due_at && task.status !== 'done' && new Date(task.due_at) < new Date();

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
	class={`group w-full cursor-grab rounded-xl border px-3 py-3 text-left transition-all duration-200 active:cursor-grabbing hover:-translate-y-0.5 hover:shadow-lg ${
		active
			? 'border-[rgb(var(--mv-accent))]/60 bg-[rgb(var(--mv-accent))]/5 shadow-[rgb(var(--mv-accent))]/10 ring-1 ring-[rgb(var(--mv-accent))]/20'
			: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/60 hover:border-[rgb(var(--mv-accent))]/30 hover:shadow-[rgb(var(--mv-accent))]/5'
	}`}
	draggable="true"
	on:dragstart={onDragStart}
	on:click={() => dispatch('select', task.id)}
	aria-pressed={active}
>
	<div class="flex items-start justify-between gap-2">
		<h4 class="text-xs font-semibold leading-relaxed text-[rgb(var(--mv-text))] line-clamp-2">
			{task.title}
		</h4>
		<span
			class={`shrink-0 rounded-md px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider ${task.priority === 1 ? 'bg-red-500/10 text-red-400 ring-1 ring-red-500/20' : task.priority === 2 ? 'bg-orange-500/10 text-orange-400 ring-1 ring-orange-500/20' : task.priority === 3 ? 'bg-sky-500/10 text-sky-400 ring-1 ring-sky-500/20' : 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-muted))]'}`}
		>
			P{task.priority}
		</span>
	</div>

	{#if task.labels.length > 0}
		<div class="mt-2 flex flex-wrap gap-1">
			{#each task.labels.slice(0, 3) as label}
				<span
					class="rounded-md bg-[rgb(var(--mv-panel-strong))] px-1.5 py-0.5 text-[9px] font-medium text-[rgb(var(--mv-muted))] border border-[rgb(var(--mv-border))]/50"
				>
					#{label}
				</span>
			{/each}
			{#if task.labels.length > 3}
				<span class="text-[9px] text-[rgb(var(--mv-muted))]/60 px-1">+{task.labels.length - 3}</span
				>
			{/if}
		</div>
	{/if}

	<div class="mt-3 flex items-center justify-between">
		<div class="flex items-center gap-2 text-[10px]">
			{#if task.due_at}
				<span
					class={`flex items-center gap-1 ${isOverdue ? 'font-medium text-red-400' : 'text-[rgb(var(--mv-muted))]/80'}`}
				>
					<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"
						><path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
						></path></svg
					>
					{formatDue(task.due_at)}
				</span>
			{/if}
			{#if task.estimate_min}
				<span class="text-[rgb(var(--mv-muted))]/60">{task.estimate_min}m</span>
			{/if}
		</div>

		{#if task.pending}
			<span
				class="flex items-center gap-1 rounded-full bg-amber-500/10 px-1.5 py-0.5 text-[9px] font-medium text-amber-400"
			>
				<span class="w-1 h-1 rounded-full bg-amber-400 animate-pulse"></span>
				sync
			</span>
		{/if}
	</div>
</button>
