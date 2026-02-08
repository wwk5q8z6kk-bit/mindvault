<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { TaskRecord } from '$lib/db';
	import type { TaskCreatePayload, TaskPatchPayload, TaskStatus } from '$lib/api/tasks';
	import type { KnowledgeNode } from '$lib/api/types';
	import {
		TASK_ASSIGNEE,
		TASK_DUE_AT,
		TASK_ESTIMATE_MIN,
		TASK_RECURRENCE,
		TASK_STATUS
	} from '$lib/api/types';
	import TemplatePicker from '$lib/components/TemplatePicker.svelte';
	import RichNoteEditor from '$lib/components/RichNoteEditor.svelte';
	import { pushToast } from '$lib/stores/toast';

	export let open = false;
	export let task: TaskRecord | null = null;
	export let availableTasks: TaskRecord[] = [];

	const dispatch = createEventDispatcher<{
		close: void;
		save: TaskCreatePayload | TaskPatchPayload;
	}>();

	const statusOptions: TaskStatus[] = [
		'inbox',
		'planned',
		'in_progress',
		'waiting',
		'review',
		'done'
	];

	let title = '';
	let description = '';
	let status: TaskStatus = 'inbox';
	let priority = 3;
	let dueAt = '';
	let estimate = '';
	let labels = '';
	let assignee = '';
	let recurrence = '';
	let dependencies: string[] = [];
	let dependencyQuery = '';

	// Subtasks
	interface Subtask {
		id: string;
		title: string;
		done: boolean;
	}
	let subtasks: Subtask[] = [];
	let newSubtaskTitle = '';

	function addSubtask() {
		if (!newSubtaskTitle.trim()) return;
		subtasks = [...subtasks, { id: crypto.randomUUID(), title: newSubtaskTitle.trim(), done: false }];
		newSubtaskTitle = '';
	}

	function removeSubtask(id: string) {
		subtasks = subtasks.filter((s) => s.id !== id);
	}

	function toggleSubtaskDone(id: string) {
		subtasks = subtasks.map((s) => (s.id === id ? { ...s, done: !s.done } : s));
	}

	$: if (open) {
		if (task) {
			title = task.title;
			description = task.description ?? '';
			status = task.status;
			priority = task.priority;
			dueAt = task.due_at ? formatDateInput(task.due_at) : '';
			estimate = task.estimate_min ? String(task.estimate_min) : '';
			labels = task.labels?.join(', ') ?? '';
			assignee = task.assignee ?? '';
			recurrence = task.recurrence ?? '';
			dependencies = [...(task.dependencies ?? [])];
			dependencyQuery = '';
			subtasks = [...((task.metadata?.subtasks as Subtask[] | undefined) ?? [])];
			newSubtaskTitle = '';
		} else {
			title = '';
			description = '';
			status = 'inbox';
			priority = 3;
			dueAt = '';
			estimate = '';
			labels = '';
			assignee = '';
			recurrence = '';
			dependencies = [];
			dependencyQuery = '';
			subtasks = [];
			newSubtaskTitle = '';
		}
	}

	function formatDateInput(value: string) {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const offset = date.getTimezoneOffset();
		const local = new Date(date.getTime() - offset * 60_000);
		return local.toISOString().slice(0, 16);
	}


	function importanceToPriority(importance: number | null | undefined): number {
		if (importance == null || Number.isNaN(importance)) return 3;
		return Math.max(1, Math.min(5, Math.round(importance * 4) + 1));
	}

	function applyTemplateToTask(template: KnowledgeNode) {
		if (!title.trim() && template.title) title = template.title;
		if (!description.trim() && template.content) description = template.content ?? '';
		if (!labels.trim() && template.tags?.length) {
			const filtered = template.tags.filter((tag) => tag.toLowerCase() !== 'template');
			if (filtered.length) labels = filtered.join(', ');
		}

		const templateStatus = template.metadata?.[TASK_STATUS] as string | undefined;
		if (status === 'inbox' && templateStatus) status = templateStatus as TaskStatus;

		if (!dueAt) {
			const due = template.metadata?.[TASK_DUE_AT] as string | undefined;
			if (due) dueAt = formatDateInput(due);
		}
		if (!estimate) {
			const estimateValue = template.metadata?.[TASK_ESTIMATE_MIN] as number | undefined;
			if (estimateValue != null) estimate = String(estimateValue);
		}
		if (!assignee.trim()) {
			const assigneeValue = template.metadata?.[TASK_ASSIGNEE] as string | undefined;
			if (assigneeValue) assignee = assigneeValue;
		}
		if (!recurrence.trim()) {
			const recurrenceValue = template.metadata?.[TASK_RECURRENCE] as string | undefined;
			if (recurrenceValue) recurrence = recurrenceValue;
		}
		if (priority === 3) {
			priority = importanceToPriority(template.importance);
		}

		pushToast('Template applied — empty fields filled.', 'success');
	}


	function parseDateInput(value: string) {
		if (!value) return null;
		const date = new Date(value);
		return Number.isNaN(date.getTime()) ? null : date.toISOString();
	}

	function toggleDependency(taskId: string) {
		if (taskId === task?.id) return;
		if (dependencies.includes(taskId)) {
			dependencies = dependencies.filter((id) => id !== taskId);
			return;
		}
		dependencies = [...dependencies, taskId];
	}

	$: dependencyCandidates = availableTasks
		.filter((candidate) => candidate.id !== task?.id)
		.filter((candidate) => {
			if (!dependencyQuery.trim()) return true;
			const queryLower = dependencyQuery.trim().toLowerCase();
			return (
				candidate.title.toLowerCase().includes(queryLower) ||
				candidate.id.toLowerCase().includes(queryLower)
			);
		})
		.slice(0, 10);

	$: selectedDependencyTasks = dependencies
		.map((id) => availableTasks.find((candidate) => candidate.id === id))
		.filter((candidate): candidate is TaskRecord => Boolean(candidate));

	function handleSubmit() {
		if (!title.trim()) return;
		const normalizedDependencies = [...new Set(dependencies.filter(Boolean))].filter(
			(id) => id !== task?.id
		);
		const payload: TaskCreatePayload | TaskPatchPayload = {
			title: title.trim(),
			description: description.trim() || null,
			status,
			priority: Number(priority) || 3,
			due_at: parseDateInput(dueAt),
			estimate_min: estimate ? Number(estimate) : null,
			labels: labels
				.split(',')
				.map((item) => item.trim())
				.filter(Boolean),
			assignee: assignee.trim() || null,
			dependencies: normalizedDependencies,
			recurrence: recurrence.trim() || null,
			metadata: {
				...(task?.metadata ?? {}),
				subtasks: subtasks.length > 0 ? subtasks : undefined
			}
		};
		dispatch('save', payload);
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4"
		role="presentation"
	>
		<div
			class="w-full max-w-2xl rounded-2xl border border-slate-800 bg-slate-950 p-6"
			role="dialog"
			aria-modal="true"
			aria-labelledby="task-modal-title"
		>
			<div class="flex items-center justify-between">
				<h3 id="task-modal-title" class="text-lg font-semibold text-white">
					{task ? 'Edit task' : 'New task'}
				</h3>
				<button
					class="rounded-lg border border-slate-800 px-2 py-1 text-xs text-slate-300"
					on:click={() => dispatch('close')}
				>
					Close
				</button>
			</div>

			<form class="mt-6 grid gap-4" on:submit|preventDefault={handleSubmit}>
				<div class="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
					<div>
						<p class="text-[11px] uppercase tracking-wide text-slate-500">Template</p>
						<p class="text-[10px] text-slate-500">Apply a task template to prefill fields.</p>
					</div>
					<TemplatePicker kind="task" label="Browse" on:apply={(event) => applyTemplateToTask(event.detail)} />
				</div>
				<div>
					<label class="text-xs uppercase tracking-wide text-slate-500" for="task-title">
						Title
					</label>
					<input
						id="task-title"
						class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
						bind:value={title}
						placeholder="Ship onboarding flow"
						required
					/>
				</div>
				<div>
					<label class="text-xs uppercase tracking-wide text-slate-500" for="task-description">
						Description
					</label>
					<div class="mt-2">
						<RichNoteEditor bind:markdown={description} placeholder="What does done look like?" />
					</div>
				</div>
				<div class="grid gap-4 md:grid-cols-2">
					<div>
						<label class="text-xs uppercase tracking-wide text-slate-500" for="task-status">
							Status
						</label>
						<select
							id="task-status"
							class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
							bind:value={status}
						>
							{#each statusOptions as option (option)}
								<option value={option}>{option.replace('_', ' ')}</option>
							{/each}
						</select>
					</div>
					<div>
						<label class="text-xs uppercase tracking-wide text-slate-500" for="task-priority">
							Priority
						</label>
						<input
							id="task-priority"
							type="number"
							min="1"
							max="5"
							class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
							bind:value={priority}
						/>
					</div>
					<div>
						<label class="text-xs uppercase tracking-wide text-slate-500" for="task-due">
							Due
						</label>
						<input
							id="task-due"
							type="datetime-local"
							class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
							bind:value={dueAt}
						/>
					</div>
					<div>
						<label class="text-xs uppercase tracking-wide text-slate-500" for="task-estimate">
							Estimate (min)
						</label>
						<input
							id="task-estimate"
							type="number"
							min="0"
							class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
							bind:value={estimate}
						/>
					</div>
				</div>
				<div class="grid gap-4 md:grid-cols-2">
					<div>
						<label class="text-xs uppercase tracking-wide text-slate-500" for="task-labels">
							Labels
						</label>
						<input
							id="task-labels"
							class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
							bind:value={labels}
							placeholder="ops, urgent"
						/>
					</div>
					<div>
						<label class="text-xs uppercase tracking-wide text-slate-500" for="task-assignee">
							Assignee
						</label>
						<input
							id="task-assignee"
							class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
							bind:value={assignee}
						/>
					</div>
				</div>
				<div>
					<label class="text-xs uppercase tracking-wide text-slate-500" for="task-dependency-search">
						Dependencies
					</label>
					<input
						id="task-dependency-search"
						class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
						bind:value={dependencyQuery}
						placeholder="Search tasks to mark as prerequisites"
					/>
					{#if selectedDependencyTasks.length > 0}
						<div class="mt-2 flex flex-wrap gap-2">
							{#each selectedDependencyTasks as dep (dep.id)}
								<button
									type="button"
									class="rounded-full border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:border-red-500/40 hover:text-red-300"
									on:click={() => toggleDependency(dep.id)}
								>
									{dep.title}
								</button>
							{/each}
						</div>
					{/if}
					<div class="mt-2 max-h-32 overflow-y-auto rounded-lg border border-slate-800 bg-slate-900/60 p-2">
						{#if dependencyCandidates.length === 0}
							<p class="text-xs text-slate-500">No task matches.</p>
						{:else}
							{#each dependencyCandidates as candidate (candidate.id)}
								<label class="flex items-center gap-2 rounded px-2 py-1 text-xs text-slate-300 hover:bg-slate-800/70">
									<input
										type="checkbox"
										checked={dependencies.includes(candidate.id)}
										on:change={() => toggleDependency(candidate.id)}
									/>
									<span class="truncate">{candidate.title}</span>
								</label>
							{/each}
						{/if}
					</div>
				</div>
				<div>
					<label class="text-xs uppercase tracking-wide text-slate-500" for="task-recurrence">
						Recurrence (RRULE)
					</label>
					<input
						id="task-recurrence"
						class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
						bind:value={recurrence}
						placeholder="FREQ=WEEKLY;BYDAY=MO"
					/>
				</div>

				<!-- Subtasks -->
				<div>
					<span class="text-xs uppercase tracking-wide text-slate-500">
						Subtasks / Checklist
					</span>
					<div class="mt-2 space-y-1.5">
						{#each subtasks as subtask (subtask.id)}
							<div class="flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
								<button
									type="button"
									class="flex h-4 w-4 flex-shrink-0 items-center justify-center rounded border transition {subtask.done
										? 'border-emerald-500 bg-emerald-500/20 text-emerald-300'
										: 'border-slate-600 hover:border-slate-500'}"
									on:click={() => toggleSubtaskDone(subtask.id)}
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
								<button
									type="button"
									title="Remove subtask"
									class="rounded p-0.5 text-slate-500 hover:bg-red-500/20 hover:text-red-300"
									on:click={() => removeSubtask(subtask.id)}
								>
									<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							</div>
						{/each}
					</div>
					<div class="mt-2 flex gap-2">
						<input
							type="text"
							bind:value={newSubtaskTitle}
							placeholder="Add a subtask..."
							class="flex-1 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white placeholder-slate-600"
							on:keydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addSubtask(); } }}
						/>
						<button
							type="button"
							class="rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-400 hover:border-slate-600 hover:text-slate-300"
							on:click={addSubtask}
							disabled={!newSubtaskTitle.trim()}
						>
							Add
						</button>
					</div>
				</div>

				<div class="flex items-center justify-end gap-2">
					<button
						type="button"
						class="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-300"
						on:click={() => dispatch('close')}
					>
						Cancel
					</button>
					<button
						type="submit"
						class="rounded-lg bg-sky-500 px-3 py-2 text-sm font-semibold text-white hover:bg-sky-400"
					>
						{task ? 'Save changes' : 'Create task'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
