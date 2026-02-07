<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { tasksStore, loadTasks, updateTaskOptimistic, completeTaskOptimistic, reopenTaskOptimistic } from '$lib/stores/tasks';
	import { selectedTaskId, taskModalState } from '$lib/stores/ui';
	import { pushToast } from '$lib/stores/toast';
	import type { TaskRecord } from '$lib/db';
	import type { TaskStatus } from '$lib/api/tasks';
	import KanbanColumn from '$lib/components/KanbanColumn.svelte';
	import TaskDetailPanel from '$lib/components/TaskDetailPanel.svelte';
	import TaskFormModal from '$lib/components/TaskFormModal.svelte';
	import SavedViewSelector from '$lib/components/SavedViewSelector.svelte';
	import { activeSavedView, loadSavedViews, setActiveSavedView } from '$lib/stores/saved-views';
	import { filterTasksBySavedView } from '$lib/utils/saved-views';
	import { goto } from '$app/navigation';
	import type { SavedView } from '$lib/api/saved-views';

	const columns: Array<{ status: TaskStatus; label: string }> = [
		{ status: 'inbox', label: 'Inbox' },
		{ status: 'planned', label: 'Planned' },
		{ status: 'in_progress', label: 'In Progress' },
		{ status: 'waiting', label: 'Waiting' },
		{ status: 'review', label: 'Review' },
		{ status: 'done', label: 'Done' }
	];

	let selectedTask: TaskRecord | null = null;
	let showDetail = false;
	let isModalOpen = false;
	let editingTask: TaskRecord | null = null;

	onMount(async () => {
		await loadTasks();
		await loadSavedViews();
	});

	$: selectedTask = $tasksStore.find((t) => t.id === $selectedTaskId) ?? null;

	$: if ($taskModalState.open) {
		isModalOpen = true;
		if ($taskModalState.mode === 'edit' && $taskModalState.taskId) {
			editingTask = $tasksStore.find((t) => t.id === $taskModalState.taskId) ?? null;
		} else {
			editingTask = null;
		}
	} else {
		isModalOpen = false;
		editingTask = null;
	}

	function tasksByStatus(status: TaskStatus): TaskRecord[] {
		const base = filterTasksBySavedView($tasksStore, $activeSavedView, 'kanban');
		return base
			.filter((t) => t.status === status)
			.sort((a, b) => {
				if (a.priority !== b.priority) return a.priority - b.priority;
				const aDue = a.due_at ? Date.parse(a.due_at) : Infinity;
				const bDue = b.due_at ? Date.parse(b.due_at) : Infinity;
				return aDue - bDue;
			});
	}

	async function handleDrop(event: CustomEvent<{ taskId: string; status: TaskStatus }>) {
		const { taskId, status } = event.detail;
		const task = $tasksStore.find((t) => t.id === taskId);
		if (!task || task.status === status) return;

		try {
			if (status === 'done') {
				const undoId = await completeTaskOptimistic(taskId);
				pushToast(`Moved to done`, 'success', 3000, undoId);
				return;
			} else if (task.status === 'done') {
				await reopenTaskOptimistic(taskId);
				if (status !== 'inbox') {
					await updateTaskOptimistic(taskId, { status });
				}
			} else {
				await updateTaskOptimistic(taskId, { status });
			}
			pushToast(`Moved to ${status.replace('_', ' ')}`, 'success');
		} catch {
			pushToast('Failed to move task', 'danger');
		}
	}

	function handleSelect(event: CustomEvent<string>) {
		selectedTaskId.set(event.detail);
		showDetail = true;
	}

	function openCreate() {
		taskModalState.set({ open: true, mode: 'create', taskId: null });
	}

	function applySavedView(view: SavedView | null) {
		if (!view) {
			setActiveSavedView(null);
			return;
		}
		setActiveSavedView(view);
		if (view.view_type === 'list') {
			goto('/tasks');
			return;
		}
		if (view.view_type === 'calendar') {
			goto('/calendar');
		}
	}

	function openEdit() {
		if (!selectedTask) return;
		taskModalState.set({ open: true, mode: 'edit', taskId: selectedTask.id });
	}

	async function handleSave(event: CustomEvent) {
		const payload = event.detail;
		try {
			if (editingTask) {
				await updateTaskOptimistic(editingTask.id, payload);
				pushToast('Task updated', 'success');
			} else {
				const { createTaskOptimistic } = await import('$lib/stores/tasks');
				await createTaskOptimistic(payload);
				pushToast('Task created', 'success');
			}
			taskModalState.set({ open: false, mode: 'create', taskId: null });
		} catch {
			pushToast('Unable to save task', 'danger');
		}
	}

	function handleClose() {
		taskModalState.set({ open: false, mode: 'create', taskId: null });
	}

	async function markDone() {
		if (!selectedTask) return;
		const undoId = await completeTaskOptimistic(selectedTask.id);
		pushToast('Marked done', 'success', 3000, undoId);
	}

	function hideDetail() {
		showDetail = false;
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key.toLowerCase() === 'n' && !isModalOpen) {
			openCreate();
		}
		if (event.key === 'Escape' && showDetail) {
			hideDetail();
		}
	}
</script>

<svelte:window on:keydown={onKeydown} />

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Kanban Board</h2>
			<p class="text-xs text-slate-400">
				Drag tasks between columns to update status. {$tasksStore.length} total tasks.
			</p>
		</div>
		<button
			class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
			on:click={openCreate}
		>
			New task
		</button>
	</div>

	<div class="mt-3">
		<SavedViewSelector currentView="kanban" on:apply={(event) => applySavedView(event.detail)} />
	</div>

	{#if $tasksStore.length === 0}
		<div class="rounded-2xl border border-dashed border-slate-800 bg-slate-900/20 p-12 text-center">
			<div class="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-violet-500/20 text-violet-300">
				<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
				</svg>
			</div>
			<h3 class="text-sm font-medium text-white">No tasks yet</h3>
			<p class="mt-1 text-xs text-slate-500">Create your first task to start organizing your work</p>
			<button
				class="mt-4 rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={openCreate}
			>
				Create your first task
			</button>
			<p class="mt-2 text-[10px] text-slate-600">
				Press <kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5">N</kbd> anytime to add a task
			</p>
		</div>
	{:else}
		<div class="flex gap-3 overflow-x-auto pb-2">
			{#each columns as col (col.status)}
				<KanbanColumn
					status={col.status}
					label={col.label}
					tasks={tasksByStatus(col.status)}
					selectedTaskId={$selectedTaskId}
					on:drop={handleDrop}
					on:select={handleSelect}
				/>
			{/each}
		</div>
	{/if}

	{#if showDetail && selectedTask}
		<div class="mt-2 rounded-2xl border border-slate-800/60 bg-slate-900/40 p-4">
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-semibold text-white">Task Detail</h3>
				<button
					class="rounded-lg px-2 py-1 text-xs text-slate-400 hover:bg-slate-800 hover:text-white"
					on:click={hideDetail}
				>
					Close
				</button>
			</div>
			<TaskDetailPanel
				task={selectedTask}
				on:edit={openEdit}
				on:complete={markDone}
				on:statusChange={async (e) => {
					if (selectedTask) {
						await updateTaskOptimistic(selectedTask.id, { status: e.detail });
						pushToast(`Status: ${e.detail.replace('_', ' ')}`, 'success');
					}
				}}
			/>
		</div>
	{/if}
</div>

<TaskFormModal
	open={isModalOpen}
	task={editingTask}
	availableTasks={$tasksStore}
	on:save={handleSave}
	on:close={handleClose}
/>
