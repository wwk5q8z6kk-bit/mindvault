<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import {
		filteredTasks,
		loadTasks,
		taskFilter,
		tasksStore,
		updateTaskOptimistic,
		createTaskOptimistic,
		quickAddTaskOptimistic,
		completeTaskOptimistic,
		reopenTaskOptimistic,
		trashTaskOptimistic,
		deleteTaskOptimistic
	} from '$lib/stores/tasks';
	import type { TaskRecord } from '$lib/db';
	import { parseQuickAddPreview, type TaskStatus } from '$lib/api/tasks';
	import type { TaskView } from '$lib/stores/tasks';
	import { page } from '$app/stores';
	import TaskListItem from '$lib/components/TaskListItem.svelte';
	import TaskDetailPanel from '$lib/components/TaskDetailPanel.svelte';
	import TaskFormModal from '$lib/components/TaskFormModal.svelte';
	import SavedViewSelector from '$lib/components/SavedViewSelector.svelte';
	import { pushToast } from '$lib/stores/toast';
	import { quickAddFocus, selectedTaskId, taskModalState } from '$lib/stores/ui';
	import { buildPlanningSnapshot, proposeDependencyDueDates } from '$lib/tasks/planning';
	import { recentItems } from '$lib/stores/recent';
	import { goto } from '$app/navigation';
	import type { SavedView } from '$lib/api/saved-views';
	import { taskFilterFromSavedView } from '$lib/utils/saved-views';
	import { setActiveSavedView } from '$lib/stores/saved-views';
	import TaskPrioritizationModal from '$lib/components/TaskPrioritizationModal.svelte';

	let showPrioritization = false;

	let selectedTask: TaskRecord | null = null;
	let isModalOpen = false;
	let editingTask: TaskRecord | null = null;
	let query = '';
	let status: TaskStatus | 'all' = 'all';
	let view: TaskView = 'all';
	let quickAddText = '';
	let quickAddInput: HTMLInputElement | null = null;

	// Bulk selection state
	let selectedIds = new Set<string>();
	let bulkMoveStatus: TaskStatus = 'planned';
	let bulkPriority: 1 | 2 | 3 | 4 | 5 = 3;
	let bulkLabelInput = '';
	let showDeleteConfirm = false;

	$: selectionCount = selectedIds.size;

	function toggleSelection(taskId: string) {
		const next = new Set(selectedIds);
		if (next.has(taskId)) {
			next.delete(taskId);
		} else {
			next.add(taskId);
		}
		selectedIds = next;
	}

	function toggleSelectAll() {
		if (selectedIds.size === $filteredTasks.length) {
			selectedIds = new Set();
		} else {
			selectedIds = new Set($filteredTasks.map((t) => t.id));
		}
	}

	function clearSelection() {
		selectedIds = new Set();
	}

	async function bulkMove() {
		let moved = 0;
		for (const id of selectedIds) {
			try {
				await updateTaskOptimistic(id, { status: bulkMoveStatus });
				moved++;
			} catch {
				// continue
			}
		}
		pushToast(`Moved ${moved} task${moved === 1 ? '' : 's'} to ${bulkMoveStatus.replace('_', ' ')}`, 'success');
		clearSelection();
	}

	async function bulkDelete() {
		let deleted = 0;
		for (const id of selectedIds) {
			try {
				await trashTaskOptimistic(id);
				deleted++;
			} catch {
				// continue
			}
		}
		pushToast(`Trashed ${deleted} task${deleted === 1 ? '' : 's'}`, 'success');
		clearSelection();
		showDeleteConfirm = false;
	}

	async function bulkSetPriority() {
		let updated = 0;
		for (const id of selectedIds) {
			try {
				await updateTaskOptimistic(id, { priority: bulkPriority });
				updated++;
			} catch {
				// continue
			}
		}
		pushToast(`Set priority P${bulkPriority} on ${updated} task${updated === 1 ? '' : 's'}`, 'success');
		clearSelection();
	}

	async function bulkAddLabels() {
		const newLabels = bulkLabelInput
			.split(/[,\s]+/)
			.map((t) => t.replace(/^#/, '').trim())
			.filter((t) => t.length > 0);
		if (newLabels.length === 0) {
			pushToast('Enter at least one label', 'warning');
			return;
		}
		let updated = 0;
		for (const id of selectedIds) {
			const task = $tasksStore.find((t) => t.id === id);
			if (!task) continue;
			const existingLabels = task.labels ?? [];
			const mergedLabels = [...new Set([...existingLabels, ...newLabels])];
			try {
				await updateTaskOptimistic(id, { labels: mergedLabels });
				updated++;
			} catch {
				// continue
			}
		}
		pushToast(`Added ${newLabels.length} label${newLabels.length === 1 ? '' : 's'} to ${updated} task${updated === 1 ? '' : 's'}`, 'success');
		bulkLabelInput = '';
		clearSelection();
	}

	// Quick add preview
	$: quickAddPreview = quickAddText.trim() ? parseQuickAddPreview(quickAddText) : null;

	function formatPreviewDate(isoDate: string): string {
		const date = new Date(isoDate);
		const today = new Date();
		const tomorrow = new Date(today);
		tomorrow.setDate(tomorrow.getDate() + 1);

		if (date.toDateString() === today.toDateString()) {
			return 'Today ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		}
		if (date.toDateString() === tomorrow.toDateString()) {
			return 'Tomorrow ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		}
		return date.toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' }) +
			' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	const statusOptions: Array<TaskStatus | 'all'> = [
		'all',
		'inbox',
		'planned',
		'in_progress',
		'waiting',
		'review',
		'done'
	];

	const bulkStatusOptions: TaskStatus[] = [
		'inbox',
		'planned',
		'in_progress',
		'waiting',
		'review',
		'done'
	];

	onMount(() => {
		let active = true;
		const unsubscribe = taskFilter.subscribe((filter) => {
			if (filter.status !== status) status = filter.status;
			if (filter.query !== query) query = filter.query;
			if (filter.view !== view) view = filter.view;
		});
		quickAddFocus.set(() => quickAddInput?.focus());
		(async () => {
			await loadTasks();
			if (!active) return;
			const items = get(tasksStore);
			const urlTaskId = get(page).url.searchParams.get('task');
			if (urlTaskId && items.some((task) => task.id === urlTaskId)) {
				selectedTaskId.set(urlTaskId);
			} else if (!get(selectedTaskId) && items[0]) {
				selectedTaskId.set(items[0].id);
			}
		})();
		return () => {
			active = false;
			quickAddFocus.set(null);
			unsubscribe();
		};
	});

	$: selectedTask = $tasksStore.find((task) => task.id === $selectedTaskId) ?? null;

	$: urlTaskId = $page.url.searchParams.get('task');
	$: if (urlTaskId && $tasksStore.some((task) => task.id === urlTaskId) && $selectedTaskId !== urlTaskId) {
		selectedTaskId.set(urlTaskId);
	}

	$: if ($taskModalState.open) {
		isModalOpen = true;
		if ($taskModalState.mode === 'edit' && $taskModalState.taskId) {
			editingTask = $tasksStore.find((task) => task.id === $taskModalState.taskId) ?? null;
		} else {
			editingTask = null;
		}
	} else {
		isModalOpen = false;
		editingTask = null;
	}

	function applyFilter(
		partial: Partial<{
			status: TaskStatus | 'all';
			query: string;
			view: TaskView;
			tags: string[];
			sort: { field: string; direction: 'asc' | 'desc' } | null;
		}>
	) {
		taskFilter.update((filter) => ({ ...filter, ...partial }));
	}

	function applySavedView(view: SavedView | null) {
		if (!view) {
			setActiveSavedView(null);
			return;
		}
		setActiveSavedView(view);
		if (view.view_type === 'kanban') {
			goto('/kanban');
			return;
		}
		if (view.view_type === 'calendar') {
			goto('/calendar');
			return;
		}
		const filter = taskFilterFromSavedView(view);
		taskFilter.set({
			status: filter.status,
			query: filter.query,
			view: 'all',
			tags: filter.tags,
			sort: filter.sort ?? null
		});
	}

	function handleSelect(id: string) {
		selectedTaskId.set(id);
		// Track in recent items
		const task = $tasksStore.find(t => t.id === id);
		if (task) {
			recentItems.addTask(task.id, task.title);
		}
	}

	function openTaskById(taskId: string) {
		selectedTaskId.set(taskId);
	}

	function openCreate() {
		taskModalState.set({ open: true, mode: 'create', taskId: null });
	}

	function openEdit() {
		if (!selectedTask) return;
		taskModalState.set({ open: true, mode: 'edit', taskId: selectedTask.id });
	}

	async function handleQuickAdd() {
		const text = quickAddText.trim();
		if (!text) return;
		try {
			await quickAddTaskOptimistic(text);
			pushToast('Quick add created', 'success');
			quickAddText = '';
		} catch {
			pushToast('Quick add failed', 'danger');
		}
	}

	async function handleSave(event: CustomEvent) {
		const payload = event.detail;
		try {
			if (editingTask) {
				await updateTaskOptimistic(editingTask.id, payload);
				pushToast('Task updated', 'success');
			} else {
				await createTaskOptimistic(payload);
				pushToast('Task created', 'success');
			}
			taskModalState.set({ open: false, mode: 'create', taskId: null });
		} catch {
			pushToast('Unable to save task. Check connection.', 'danger');
		}
	}

	function handleClose() {
		taskModalState.set({ open: false, mode: 'create', taskId: null });
	}

	async function markDone() {
		if (!selectedTask) return;
		await toggleComplete(selectedTask.id);
	}

	async function toggleComplete(taskId: string) {
		const task = $tasksStore.find((t) => t.id === taskId);
		if (!task) return;
		if (task.status === 'done') {
			await reopenTaskOptimistic(taskId);
			pushToast('Reopened', 'success');
		} else {
			const undoId = await completeTaskOptimistic(taskId);
			pushToast('Marked done', 'success', 3000, undoId);
		}
	}

	async function togglePin() {
		if (!selectedTask) return;
		await updateTaskOptimistic(selectedTask.id, {
			metadata: { ...selectedTask.metadata, pinned: !selectedTask.metadata?.pinned }
		});
		pushToast(selectedTask.metadata?.pinned ? 'Unpinned' : 'Pinned', 'success');
	}

	async function autoScheduleDependencies() {
		if (planning.cycles.length > 0) {
			pushToast('Resolve dependency cycles before auto-scheduling.', 'warning');
			return;
		}
		const updates = proposeDependencyDueDates($tasksStore);
		const entries = Object.entries(updates);
		if (entries.length === 0) {
			pushToast('No schedule changes needed.', 'info');
			return;
		}

		let applied = 0;
		for (const [taskId, dueAt] of entries) {
			try {
				await updateTaskOptimistic(taskId, { due_at: dueAt });
				applied += 1;
			} catch {
				// Keep applying the rest so one failure does not block full scheduling pass.
			}
		}
		pushToast(`Auto-scheduled ${applied} task${applied === 1 ? '' : 's'}.`, 'success');
	}

	$: viewTabs = (() => {
		const now = new Date();
		const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const todayEnd = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
		const weekEnd = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 7);
		const allActive = $tasksStore.filter((t) => t.status !== 'done');
		return [
			{ key: 'all' as TaskView, label: 'All', count: allActive.length },
			{ key: 'inbox' as TaskView, label: 'Inbox', count: $tasksStore.filter((t) => t.status === 'inbox').length },
			{ key: 'today' as TaskView, label: 'Today', count: $tasksStore.filter((t) => {
				if (!t.due_at || t.status === 'done') return false;
				const d = new Date(t.due_at);
				return d >= todayStart && d < todayEnd;
			}).length },
			{ key: 'upcoming' as TaskView, label: 'Upcoming', count: $tasksStore.filter((t) => {
				if (!t.due_at || t.status === 'done') return false;
				const d = new Date(t.due_at);
				return d >= now && d <= weekEnd;
			}).length }
		];
	})();

	$: planning = buildPlanningSnapshot($tasksStore);
	$: blockedSet = new Set(planning.blockedTaskIds);
	$: taskById = new Map($tasksStore.map((task) => [task.id, task]));
	$: selectedDependencyTasks =
		selectedTask?.dependencies
			?.map((id) => taskById.get(id))
			.filter((task): task is TaskRecord => Boolean(task)) ?? [];
	$: selectedDependentTasks =
		(selectedTask ? planning.dependentsByTask[selectedTask.id] ?? [] : [])
			.map((id) => taskById.get(id))
			.filter((task): task is TaskRecord => Boolean(task));
	$: selectedBlockedReason = selectedTask ? planning.blockedReasonByTask[selectedTask.id] ?? null : null;
	$: criticalPathTitles = planning.criticalPathTaskIds
		.map((taskId) => taskById.get(taskId)?.title)
		.filter((title): title is string => Boolean(title));

	function formatMinutes(minutes: number) {
		if (minutes < 60) return `${minutes}m`;
		const hours = Math.floor(minutes / 60);
		const remainder = minutes % 60;
		return remainder === 0 ? `${hours}h` : `${hours}h ${remainder}m`;
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key.toLowerCase() === 'n') {
			openCreate();
		}
	}
</script>

<svelte:window on:keydown={onKeydown} />

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-5">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold text-white">Tasks</h2>
				<p class="text-xs text-slate-400">Total {$tasksStore.length} tasks</p>
			</div>
			<div class="flex gap-2">
				<button
					class="rounded-lg border border-purple-500/30 bg-purple-500/10 px-3 py-2 text-xs font-medium text-purple-300 hover:bg-purple-500/20"
					on:click={() => (showPrioritization = true)}
					title="AI Prioritize"
				>
					<span class="hidden sm:inline">AI Prioritize</span>
					<span class="sm:hidden">AI</span>
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={openCreate}
				>
					New task
				</button>
			</div>
		</div>

		<div class="mt-4">
			<label class="text-xs uppercase tracking-wide text-slate-500" for="quick-add">
				Quick add
			</label>
			<input
				id="quick-add"
				class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
				placeholder="buy milk tomorrow 5pm p2 #home @alex 30m every week"
				bind:value={quickAddText}
				bind:this={quickAddInput}
				on:keydown={(event) => {
					if (event.key === 'Enter') {
						event.preventDefault();
						handleQuickAdd();
					}
				}}
			/>
			<p class="mt-2 text-[11px] text-slate-500">
				Use Cmd/Ctrl+K for the command palette. Quick add supports p1–p5, #tags, @assignee, 30m/1h,
				and simple recurrence (every week/month).
			</p>

			{#if quickAddPreview && quickAddText.trim()}
				<div class="mt-2 flex flex-wrap items-center gap-2 rounded-lg border border-slate-800/60 bg-slate-800/30 px-3 py-2 text-[11px]">
					<span class="text-slate-500">Preview:</span>
					{#if quickAddPreview.priority !== 3}
						<span class={`rounded px-1.5 py-0.5 ${quickAddPreview.priority <= 2 ? 'bg-red-500/20 text-red-300' : 'bg-amber-500/20 text-amber-300'}`}>
							P{quickAddPreview.priority}
						</span>
					{/if}
					{#if quickAddPreview.due_at}
						<span class="rounded bg-sky-500/20 px-1.5 py-0.5 text-sky-300">
							{formatPreviewDate(quickAddPreview.due_at)}
						</span>
					{/if}
					{#if quickAddPreview.estimate_min}
						<span class="rounded bg-violet-500/20 px-1.5 py-0.5 text-violet-300">
							{quickAddPreview.estimate_min >= 60 ? `${Math.floor(quickAddPreview.estimate_min / 60)}h${quickAddPreview.estimate_min % 60 ? ` ${quickAddPreview.estimate_min % 60}m` : ''}` : `${quickAddPreview.estimate_min}m`}
						</span>
					{/if}
					{#if quickAddPreview.assignee}
						<span class="rounded bg-emerald-500/20 px-1.5 py-0.5 text-emerald-300">
							@{quickAddPreview.assignee}
						</span>
					{/if}
					{#if quickAddPreview.recurrence}
						<span class="rounded bg-amber-500/20 px-1.5 py-0.5 text-amber-300">
							Every {quickAddPreview.recurrence}
						</span>
					{/if}
					{#each quickAddPreview.labels as label}
						<span class="rounded bg-slate-700 px-1.5 py-0.5 text-slate-300">#{label}</span>
					{/each}
				</div>
			{/if}
		</div>

		<div class="mt-4">
			<SavedViewSelector currentView="list" on:apply={(event) => applySavedView(event.detail)} />
		</div>

		<!-- View tabs -->
		<div class="mt-4 flex gap-1 rounded-lg border border-slate-800 bg-slate-900/60 p-1">
			{#each viewTabs as tab (tab.key)}
				<button
					class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[11px] font-medium transition {view === tab.key
						? 'bg-slate-700 text-white'
						: 'text-slate-400 hover:text-slate-200'}"
					on:click={() => applyFilter({ view: tab.key, status: 'all' })}
				>
					{tab.label}
					{#if tab.count > 0}
						<span class="rounded-full bg-slate-800 px-1.5 py-0.5 text-[9px] {view === tab.key ? 'text-white' : 'text-slate-500'}">
							{tab.count}
						</span>
					{/if}
				</button>
			{/each}
		</div>

		<div class="mt-3 flex flex-wrap gap-2">
			<input
				class="flex-1 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
				placeholder="Search tasks"
				aria-label="Search tasks"
				bind:value={query}
				on:input={() => applyFilter({ query })}
			/>
			<select
				class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
				aria-label="Filter tasks by status"
				bind:value={status}
				on:change={() => applyFilter({ status, view: 'all' })}
			>
				{#each statusOptions as statusOption (statusOption)}
					<option value={statusOption}>{statusOption.replace('_', ' ')}</option>
				{/each}
			</select>
		</div>

		<div class="mt-3 rounded-xl border border-slate-800 bg-slate-900/40 p-3">
			<div class="flex items-center justify-between">
				<div>
					<p class="text-[10px] uppercase tracking-wide text-slate-500">Planning Radar</p>
					<p class="mt-1 text-xs text-slate-300">
						{planning.readyTaskIds.length} ready · {planning.blockedTaskIds.length} blocked · {planning.cycles.length} cycles
					</p>
				</div>
				<button
					class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800 disabled:opacity-40"
					on:click={autoScheduleDependencies}
					disabled={$tasksStore.length === 0}
				>
					Auto-schedule
				</button>
			</div>
			{#if planning.cycles.length > 0}
				<p class="mt-2 text-[11px] text-red-300">
					Cycles detected: {planning.cycles.length}. Break circular dependencies first.
				</p>
			{/if}
			{#if criticalPathTitles.length > 0}
				<p class="mt-2 text-[11px] text-slate-400">
					Critical path ({formatMinutes(planning.criticalPathMinutes)}):
					<span class="text-slate-200">{criticalPathTitles.join(' -> ')}</span>
				</p>
			{/if}
		</div>

		<!-- Select all toggle -->
		{#if $filteredTasks.length > 0}
			<div class="mt-3 flex items-center gap-2">
				<button
					class="flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] text-slate-400 hover:text-slate-200 transition"
					on:click={toggleSelectAll}
				>
					<span class="flex h-4 w-4 items-center justify-center rounded border {selectedIds.size === $filteredTasks.length && $filteredTasks.length > 0
						? 'border-sky-500 bg-sky-500/20 text-sky-300'
						: 'border-slate-700'}">
						{#if selectedIds.size === $filteredTasks.length && $filteredTasks.length > 0}
							<svg class="h-2.5 w-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
							</svg>
						{/if}
					</span>
					{selectedIds.size === $filteredTasks.length && $filteredTasks.length > 0 ? 'Deselect all' : 'Select all'}
				</button>
				{#if selectionCount > 0}
					<span class="text-[11px] text-slate-500">{selectionCount} selected</span>
				{/if}
			</div>
		{/if}

		<div class="mt-3 flex flex-col gap-3">
			{#if $filteredTasks.length === 0}
				<div class="rounded-xl border border-dashed border-slate-800 bg-slate-900/20 p-8 text-center">
					{#if $tasksStore.length === 0}
						<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-violet-500/20 text-lg text-violet-300">
							+
						</div>
						<h3 class="text-sm font-medium text-white">No tasks yet</h3>
						<p class="mt-1 text-xs text-slate-500">
							Add your first task to start getting things done.
						</p>
						<div class="mt-4 flex justify-center gap-2">
							<button
								class="rounded-lg bg-violet-500 px-4 py-2 text-xs font-semibold text-white hover:bg-violet-400"
								on:click={openCreate}
							>
								Create your first task
							</button>
						</div>
						<p class="mt-3 text-[10px] text-slate-600">
							Or use <kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5">Cmd+Shift+N</kbd> for quick capture
						</p>
					{:else}
						<p class="text-sm text-slate-400">No tasks match your current filter.</p>
						<button
							class="mt-2 text-xs text-sky-400 hover:text-sky-300"
							on:click={() => applyFilter({ status: 'all', view: 'all', query: '' })}
						>
							Clear filters
						</button>
					{/if}
				</div>
			{:else}
				{#each $filteredTasks as task (task.id)}
					<div class="flex items-start gap-2">
						<button
							class="mt-3 flex h-4 w-4 flex-shrink-0 items-center justify-center rounded border transition {selectedIds.has(task.id)
								? 'border-sky-500 bg-sky-500/20 text-sky-300'
								: 'border-slate-700 hover:border-slate-500'}"
							on:click|stopPropagation={() => toggleSelection(task.id)}
							aria-label={selectedIds.has(task.id) ? 'Deselect task' : 'Select task'}
						>
							{#if selectedIds.has(task.id)}
								<svg class="h-2.5 w-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
								</svg>
							{/if}
						</button>
						<div class="min-w-0 flex-1">
							<TaskListItem
								{task}
								active={selectedTask?.id === task.id}
								blocked={blockedSet.has(task.id)}
								blockedReason={planning.blockedReasonByTask[task.id] ?? ''}
								on:select={(e) => handleSelect(e.detail)}
								on:complete={(e) => toggleComplete(e.detail)}
							/>
						</div>
					</div>
				{/each}
			{/if}
		</div>
	</section>

	<section class="lg:col-span-7">
		<TaskDetailPanel
			task={selectedTask}
			dependencyTasks={selectedDependencyTasks}
			dependentTasks={selectedDependentTasks}
			blockedReason={selectedBlockedReason}
			on:edit={openEdit}
			on:complete={markDone}
			on:togglePin={togglePin}
			on:openTask={(e) => openTaskById(e.detail)}
			on:statusChange={async (e) => {
				if (selectedTask) {
					await updateTaskOptimistic(selectedTask.id, { status: e.detail });
					pushToast(`Status: ${e.detail.replace('_', ' ')}`, 'success');
				}
			}}
			on:subtasksUpdate={async (e) => {
				if (selectedTask) {
					await updateTaskOptimistic(selectedTask.id, {
						metadata: { ...(selectedTask.metadata ?? {}), subtasks: e.detail }
					});
				}
			}}
		/>
	</section>
</div>

<!-- Bulk action bar -->
{#if selectionCount > 0}
	<div class="fixed bottom-6 left-1/2 z-40 flex -translate-x-1/2 items-center gap-3 rounded-xl border border-slate-700 bg-slate-900/95 px-5 py-3 shadow-xl backdrop-blur">
		<span class="text-xs font-medium text-white">{selectionCount} selected</span>

		<div class="flex items-center gap-1.5">
			<label class="text-[10px] text-slate-400" for="bulk-move">Status</label>
			<select
				id="bulk-move"
				class="rounded-md border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] text-white"
				bind:value={bulkMoveStatus}
			>
				{#each bulkStatusOptions as opt (opt)}
					<option value={opt}>{opt.replace('_', ' ')}</option>
				{/each}
			</select>
			<button
				class="rounded-md bg-sky-500/20 px-2 py-1 text-[11px] text-sky-200 hover:bg-sky-500/30"
				on:click={bulkMove}
			>
				Apply
			</button>
		</div>

		<div class="h-4 w-px bg-slate-700"></div>

		<div class="flex items-center gap-1.5">
			<label class="text-[10px] text-slate-400" for="bulk-priority">Priority</label>
			<select
				id="bulk-priority"
				class="rounded-md border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] text-white"
				bind:value={bulkPriority}
			>
				<option value={1}>P1</option>
				<option value={2}>P2</option>
				<option value={3}>P3</option>
				<option value={4}>P4</option>
				<option value={5}>P5</option>
			</select>
			<button
				class="rounded-md bg-amber-500/20 px-2 py-1 text-[11px] text-amber-200 hover:bg-amber-500/30"
				on:click={bulkSetPriority}
			>
				Set
			</button>
		</div>

		<div class="h-4 w-px bg-slate-700"></div>

		<div class="flex items-center gap-1.5">
			<label class="text-[10px] text-slate-400" for="bulk-labels">Labels</label>
			<input
				id="bulk-labels"
				type="text"
				class="w-24 rounded-md border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] text-white placeholder:text-slate-500"
				placeholder="work, urgent"
				bind:value={bulkLabelInput}
				on:keydown={(e) => e.key === 'Enter' && bulkAddLabels()}
			/>
			<button
				class="rounded-md bg-violet-500/20 px-2 py-1 text-[11px] text-violet-200 hover:bg-violet-500/30"
				on:click={bulkAddLabels}
			>
				Add
			</button>
		</div>

		<div class="h-4 w-px bg-slate-700"></div>

		<button
			class="rounded-md bg-red-500/20 px-2.5 py-1 text-[11px] text-red-200 hover:bg-red-500/30"
			on:click={() => (showDeleteConfirm = true)}
		>
			Trash
		</button>

		<button
			class="text-[11px] text-slate-400 hover:text-slate-200"
			on:click={clearSelection}
		>
			Clear
		</button>
	</div>
{/if}

<!-- Delete confirmation dialog -->
{#if showDeleteConfirm}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
		<div class="w-full max-w-sm rounded-xl border border-slate-700 bg-slate-900 p-6 shadow-xl">
			<h3 class="text-sm font-semibold text-white">Trash {selectionCount} task{selectionCount === 1 ? '' : 's'}?</h3>
			<p class="mt-2 text-xs text-slate-400">
				Items will be moved to trash and can be restored later.
			</p>
			<div class="mt-4 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:border-slate-500"
					on:click={() => (showDeleteConfirm = false)}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-red-500/20 px-3 py-1.5 text-xs font-medium text-red-200 hover:bg-red-500/30"
					on:click={bulkDelete}
				>
					Trash
				</button>
			</div>
		</div>
	</div>
{/if}

<TaskFormModal
	open={isModalOpen}
	task={editingTask}
	availableTasks={$tasksStore}
	on:save={handleSave}
	on:close={handleClose}
/>

<TaskPrioritizationModal
	bind:open={showPrioritization}
	on:close={() => (showPrioritization = false)}
	on:applied={() => void loadTasks()}
/>
