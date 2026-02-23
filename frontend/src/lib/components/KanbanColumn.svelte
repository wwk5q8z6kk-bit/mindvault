<script lang="ts">
	import type { TaskRecord } from '$lib/db';
	import type { TaskStatus } from '$lib/api/tasks';
	import { createEventDispatcher } from 'svelte';
	import KanbanCard from './KanbanCard.svelte';

	export let status: TaskStatus;
	export let label: string;
	export let tasks: TaskRecord[];
	export let selectedTaskId: string | null = null;

	const dispatch = createEventDispatcher<{
		drop: { taskId: string; status: TaskStatus };
		select: string;
	}>();

	let dragOver = false;

	const statusAccent: Record<TaskStatus, string> = {
		inbox: 'bg-slate-500',
		planned: 'bg-violet-500',
		in_progress: 'bg-sky-500',
		waiting: 'bg-amber-500',
		review: 'bg-cyan-500',
		done: 'bg-emerald-500'
	};

	function onDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer) {
			event.dataTransfer.dropEffect = 'move';
		}
		dragOver = true;
	}

	function onDragLeave() {
		dragOver = false;
	}

	function onDrop(event: DragEvent) {
		event.preventDefault();
		dragOver = false;
		const taskId = event.dataTransfer?.getData('text/plain');
		if (taskId) {
			dispatch('drop', { taskId, status });
		}
	}
</script>

<div
	class={`flex min-w-[250px] flex-1 flex-col rounded-[var(--mv-radius)] border backdrop-blur-2xl transition-all duration-300 ${
		dragOver
			? 'border-sky-500/50 bg-sky-500/10 shadow-[0_0_20px_rgba(14,165,233,0.15)] scale-[1.01]'
			: 'border-white/5 bg-[rgb(var(--mv-panel))]/30 hover:bg-[rgb(var(--mv-panel))]/40 shadow-lg'
	}`}
	role="region"
	aria-label="{label} column"
	on:dragover={onDragOver}
	on:dragleave={onDragLeave}
	on:drop={onDrop}
>
	<div class="flex items-center gap-2 px-4 py-3">
		<span class={`h-2 w-2 rounded-full ${statusAccent[status]}`}></span>
		<h3 class="text-xs font-semibold uppercase tracking-wide text-[rgb(var(--mv-muted))]">
			{label}
		</h3>
		<span
			class="ml-auto rounded-full bg-[rgb(var(--mv-panel-strong))] px-2 py-0.5 text-[10px] text-[rgb(var(--mv-muted))]"
		>
			{tasks.length}
		</span>
	</div>

	<div
		class="flex flex-1 flex-col gap-2 overflow-y-auto px-3 pb-3"
		style="max-height: calc(100vh - 220px);"
	>
		{#if tasks.length === 0}
			<div
				class="flex min-h-[60px] items-center justify-center rounded-xl border border-dashed border-[rgb(var(--mv-border))]/60 text-[10px] text-slate-600"
			>
				Drop tasks here
			</div>
		{:else}
			{#each tasks as task (task.id)}
				<KanbanCard
					{task}
					active={selectedTaskId === task.id}
					on:select={(e) => dispatch('select', e.detail)}
				/>
			{/each}
		{/if}
	</div>
</div>
