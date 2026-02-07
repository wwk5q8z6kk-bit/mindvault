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
	class={`flex min-w-[220px] flex-1 flex-col rounded-2xl border transition-colors ${
		dragOver
			? 'border-sky-500/50 bg-sky-500/5'
			: 'border-slate-800/60 bg-slate-900/30'
	}`}
	role="region"
	aria-label="{label} column"
	on:dragover={onDragOver}
	on:dragleave={onDragLeave}
	on:drop={onDrop}
>
	<div class="flex items-center gap-2 px-4 py-3">
		<span class={`h-2 w-2 rounded-full ${statusAccent[status]}`}></span>
		<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-300">{label}</h3>
		<span class="ml-auto rounded-full bg-slate-800 px-2 py-0.5 text-[10px] text-slate-400">
			{tasks.length}
		</span>
	</div>

	<div class="flex flex-1 flex-col gap-2 overflow-y-auto px-3 pb-3" style="max-height: calc(100vh - 220px);">
		{#if tasks.length === 0}
			<div
				class="flex min-h-[60px] items-center justify-center rounded-xl border border-dashed border-slate-800/60 text-[10px] text-slate-600"
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
