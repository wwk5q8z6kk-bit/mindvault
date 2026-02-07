import { derived, get, writable } from 'svelte/store';
import type { TaskCreatePayload, TaskPatchPayload, TaskStatus, SnoozeTaskParams } from '$lib/api/tasks';

export type TaskView = 'all' | 'inbox' | 'today' | 'upcoming';
import { createTask, deleteTask, listTasks, updateTask, quickAddTask, completeTask, reopenTask, snoozeTask } from '$lib/api/tasks';
import { db, type OfflineOp, type TaskRecord } from '$lib/db';
import { activeNamespace } from '$lib/stores/namespace';
import { pushUndo } from '$lib/stores/undo';

export const tasksStore = writable<TaskRecord[]>([]);
export const pendingSyncCount = writable<number>(0);
export const taskFilter = writable<{
	status: TaskStatus | 'all';
	query: string;
	view: TaskView;
	tags: string[];
	sort?: { field: string; direction: 'asc' | 'desc' } | null;
}>({
	status: 'all',
	query: '',
	view: 'all',
	tags: [],
	sort: null
});

export const filteredTasks = derived([tasksStore, taskFilter], ([$tasks, $filter]) => {
	let items = $tasks.filter((task) => !task.labels?.includes('trashed'));

	if ($filter.view === 'inbox') {
		items = items.filter((task) => task.status === 'inbox');
	} else if ($filter.view === 'today') {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
		items = items.filter((task) => {
			if (!task.due_at) return false;
			const due = new Date(task.due_at);
			return due >= start && due < end;
		});
	} else if ($filter.view === 'upcoming') {
		const now = new Date();
		const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 7);
		items = items.filter((task) => {
			if (!task.due_at) return false;
			const due = new Date(task.due_at);
			return due >= now && due <= end;
		});
	}

	if ($filter.status !== 'all' && $filter.view === 'all') {
		items = items.filter((task) => task.status === $filter.status);
	}

	if ($filter.tags.length > 0) {
		items = items.filter((task) => $filter.tags.every((tag) => task.labels?.includes(tag)));
	}

	if ($filter.query.trim()) {
		const q = $filter.query.toLowerCase();
		items = items.filter((task) => task.title.toLowerCase().includes(q));
	}

	const sortSpec = $filter.sort;
	const direction = sortSpec?.direction === 'desc' ? -1 : 1;

	return items.sort((a, b) => {
		const aPinned = a.metadata?.pinned === true ? 1 : 0;
		const bPinned = b.metadata?.pinned === true ? 1 : 0;
		if (aPinned !== bPinned) return bPinned - aPinned;

		if (sortSpec?.field === 'priority') {
			return (a.priority - b.priority) * direction;
		}
		if (sortSpec?.field === 'created_at') {
			return (Date.parse(a.created_at) - Date.parse(b.created_at)) * direction;
		}
		if (sortSpec?.field === 'updated_at') {
			return (Date.parse(a.updated_at) - Date.parse(b.updated_at)) * direction;
		}
		if (sortSpec?.field === 'title') {
			return a.title.localeCompare(b.title) * direction;
		}

		const aDue = a.due_at ? Date.parse(a.due_at) : Infinity;
		const bDue = b.due_at ? Date.parse(b.due_at) : Infinity;
		return (aDue - bDue) * direction;
	});
});

const ONLINE_PING_MS = 5000;
let syncTimer: ReturnType<typeof setInterval> | null = null;

export async function loadTasks(): Promise<void> {
	try {
		const ns = get(activeNamespace);
		const items = await listTasks(undefined, ns);
		await db.tasks.clear();
		await db.tasks.bulkPut(items);
		tasksStore.set(items);
	} catch {
		const cached = await db.tasks.toArray();
		tasksStore.set(cached);
	}
}

export async function createTaskOptimistic(payload: TaskCreatePayload): Promise<void> {
	if (navigator.onLine) {
		const created = await createTask(payload);
		await db.tasks.put(created);
		tasksStore.update((items) => [created, ...items]);
		return;
	}

	const localId = crypto.randomUUID();
	const now = new Date().toISOString();
	const localTask: TaskRecord = {
		id: localId,
		title: payload.title,
		description: payload.description ?? null,
		status: payload.status ?? 'inbox',
		priority: payload.priority ?? 3,
		due_at: payload.due_at ?? null,
		estimate_min: payload.estimate_min ?? null,
		labels: payload.labels ?? [],
		assignee: payload.assignee ?? null,
		dependencies: payload.dependencies ?? [],
		recurrence: payload.recurrence ?? null,
		created_at: now,
		updated_at: now,
		completed_at: null,
		metadata: payload.metadata ?? {},
		pending: true,
		localOnly: true
	};

	await db.tasks.put(localTask);
	tasksStore.update((items) => [localTask, ...items]);
	await enqueueOp({
		id: crypto.randomUUID(),
		op: 'create',
		localId,
		payload,
		createdAt: now
	});
}

export async function quickAddTaskOptimistic(text: string): Promise<void> {
	if (navigator.onLine) {
		const response = await quickAddTask(text);
		await db.tasks.put(response.task);
		tasksStore.update((items) => [response.task, ...items]);
		return;
	}

	await createTaskOptimistic({ title: text });
}

export async function updateTaskOptimistic(
	taskId: string,
	payload: TaskPatchPayload
): Promise<void> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	const updated: TaskRecord = {
		...(existing ?? ({} as TaskRecord)),
		...payload,
		id: taskId,
		updated_at: new Date().toISOString(),
		pending: !navigator.onLine || existing?.pending
	};

	tasksStore.update((list) => list.map((task) => (task.id === taskId ? updated : task)));
	await db.tasks.put(updated);

	if (navigator.onLine) {
		await updateTask(taskId, payload);
		return;
	}

	await enqueueOp({
		id: crypto.randomUUID(),
		op: 'update',
		taskId,
		payload,
		createdAt: new Date().toISOString()
	});
}

export async function trashTaskOptimistic(taskId: string): Promise<void> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	if (!existing) return;
	await updateTaskOptimistic(taskId, {
		labels: [...(existing.labels ?? []).filter((l) => l !== 'trashed'), 'trashed'],
		metadata: { ...existing.metadata, trashed_at: new Date().toISOString() }
	});
}

export async function restoreTaskOptimistic(taskId: string): Promise<void> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	if (!existing) return;
	const { trashed_at: _, ...restMeta } = (existing.metadata ?? {}) as Record<string, unknown>;
	await updateTaskOptimistic(taskId, {
		labels: (existing.labels ?? []).filter((l) => l !== 'trashed'),
		metadata: restMeta
	});
}

export async function deleteTaskOptimistic(taskId: string): Promise<string | undefined> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	const snapshot = existing ? { ...existing } : undefined;

	tasksStore.update((items) => items.filter((task) => task.id !== taskId));
	await db.tasks.delete(taskId);

	let undoId: string | undefined;
	if (snapshot) {
		undoId = pushUndo(
			`Delete "${snapshot.title}"`,
			async () => {
				await db.tasks.put(snapshot);
				tasksStore.update((list) => [snapshot, ...list]);
				if (navigator.onLine) {
					await createTask({
						title: snapshot.title,
						description: snapshot.description,
						status: snapshot.status,
						priority: snapshot.priority,
						due_at: snapshot.due_at,
						labels: snapshot.labels,
						metadata: snapshot.metadata
					});
				}
			},
			async () => {
				await deleteTaskOptimistic(taskId);
			},
			'task'
		);
	}

	if (navigator.onLine) {
		await deleteTask(taskId);
		return undoId;
	}

	await enqueueOp({
		id: crypto.randomUUID(),
		op: 'delete',
		taskId,
		createdAt: new Date().toISOString()
	});
	return undoId;
}

export async function completeTaskOptimistic(taskId: string): Promise<string | undefined> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	const now = new Date().toISOString();
	const updated: TaskRecord = {
		...(existing ?? ({} as TaskRecord)),
		id: taskId,
		status: 'done',
		completed_at: now,
		updated_at: now,
		pending: !navigator.onLine || existing?.pending
	};

	tasksStore.update((list) => list.map((task) => (task.id === taskId ? updated : task)));
	await db.tasks.put(updated);

	const undoId = pushUndo(
		`Complete "${existing?.title ?? 'task'}"`,
		async () => {
			await reopenTaskOptimistic(taskId);
		},
		async () => {
			await completeTaskOptimistic(taskId);
		},
		'task'
	);

	if (navigator.onLine) {
		await completeTask(taskId);
		return undoId;
	}

	await enqueueOp({
		id: crypto.randomUUID(),
		op: 'complete',
		taskId,
		createdAt: now
	});
	return undoId;
}

export async function reopenTaskOptimistic(taskId: string): Promise<void> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	const now = new Date().toISOString();
	const updated: TaskRecord = {
		...(existing ?? ({} as TaskRecord)),
		id: taskId,
		status: 'inbox',
		completed_at: null,
		updated_at: now,
		pending: !navigator.onLine || existing?.pending
	};

	tasksStore.update((list) => list.map((task) => (task.id === taskId ? updated : task)));
	await db.tasks.put(updated);

	if (navigator.onLine) {
		await reopenTask(taskId);
		return;
	}

	await enqueueOp({
		id: crypto.randomUUID(),
		op: 'reopen',
		taskId,
		createdAt: now
	});
}

export async function snoozeTaskOptimistic(taskId: string, snoozeUntil: string): Promise<void> {
	const items = get(tasksStore);
	const existing = items.find((task) => task.id === taskId);
	const now = new Date().toISOString();
	const updated: TaskRecord = {
		...(existing ?? ({} as TaskRecord)),
		id: taskId,
		updated_at: now,
		metadata: { ...(existing?.metadata ?? {}), snoozed_until: snoozeUntil },
		pending: !navigator.onLine || existing?.pending
	};

	tasksStore.update((list) => list.map((task) => (task.id === taskId ? updated : task)));
	await db.tasks.put(updated);

	if (navigator.onLine) {
		await snoozeTask(taskId, { snooze_until: snoozeUntil });
		return;
	}

	await enqueueOp({
		id: crypto.randomUUID(),
		op: 'snooze',
		taskId,
		payload: { snooze_until: snoozeUntil },
		createdAt: now
	});
}

async function enqueueOp(op: OfflineOp) {
	await db.queue.put(op);
	pendingSyncCount.set(await db.queue.count());
}

export async function syncQueue(): Promise<void> {
	if (!navigator.onLine) return;

	const queue = await db.queue.orderBy('createdAt').toArray();
	for (const op of queue) {
		try {
			switch (op.op) {
				case 'create': {
					if (!op.payload) break;
					const payload = op.payload as TaskCreatePayload;
					const created = await createTask(payload);
					if (op.localId) {
						await db.tasks.delete(op.localId);
						tasksStore.update((items) =>
							items.map((task) => (task.id === op.localId ? created : task))
						);
					}
					await db.tasks.put(created);
					break;
				}
				case 'update': {
					if (!op.taskId || !op.payload) break;
					await updateTask(op.taskId, op.payload as TaskPatchPayload);
					break;
				}
				case 'delete': {
					if (!op.taskId) break;
					await deleteTask(op.taskId);
					break;
				}
				case 'complete': {
					if (!op.taskId) break;
					await completeTask(op.taskId);
					break;
				}
				case 'reopen': {
					if (!op.taskId) break;
					await reopenTask(op.taskId);
					break;
				}
				case 'snooze': {
					if (!op.taskId) break;
					await snoozeTask(op.taskId, op.payload as SnoozeTaskParams);
					break;
				}
			}
			await db.queue.delete(op.id);
		} catch {
			// stop on first failure to avoid loops
			break;
		}
	}
	pendingSyncCount.set(await db.queue.count());
}

export function startSyncLoop() {
	if (syncTimer) return;
	void db.queue.count().then((c) => pendingSyncCount.set(c));
	syncTimer = setInterval(() => {
		void syncQueue();
	}, ONLINE_PING_MS);
	window.addEventListener('online', () => void syncQueue());
	void syncQueue();
}
