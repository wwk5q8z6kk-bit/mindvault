import { get } from 'svelte/store';
import { describe, it, expect, beforeEach, vi } from 'vitest';

// Use vi.hoisted so mock variables are available in vi.mock factories (which are hoisted)
const {
	mockTasksTable,
	mockQueueTable,
	mockCreateTask,
	mockUpdateTask,
	mockDeleteTask,
	mockListTasks,
	mockCompleteTask,
	mockReopenTask,
	mockSnoozeTask,
	mockQuickAddTask
} = vi.hoisted(() => ({
	mockTasksTable: {
		put: vi.fn().mockResolvedValue(undefined),
		delete: vi.fn().mockResolvedValue(undefined),
		clear: vi.fn().mockResolvedValue(undefined),
		bulkPut: vi.fn().mockResolvedValue(undefined),
		toArray: vi.fn().mockResolvedValue([])
	},
	mockQueueTable: {
		put: vi.fn().mockResolvedValue(undefined),
		delete: vi.fn().mockResolvedValue(undefined),
		count: vi.fn().mockResolvedValue(0),
		orderBy: vi.fn().mockReturnValue({
			toArray: vi.fn().mockResolvedValue([])
		})
	},
	mockCreateTask: vi.fn(),
	mockUpdateTask: vi.fn(),
	mockDeleteTask: vi.fn(),
	mockListTasks: vi.fn(),
	mockCompleteTask: vi.fn(),
	mockReopenTask: vi.fn(),
	mockSnoozeTask: vi.fn(),
	mockQuickAddTask: vi.fn()
}));

vi.mock('$lib/db', () => ({
	db: {
		tasks: mockTasksTable,
		queue: mockQueueTable
	}
}));

vi.mock('$lib/api/tasks', () => ({
	createTask: (...args: unknown[]) => mockCreateTask(...args),
	updateTask: (...args: unknown[]) => mockUpdateTask(...args),
	deleteTask: (...args: unknown[]) => mockDeleteTask(...args),
	listTasks: (...args: unknown[]) => mockListTasks(...args),
	completeTask: (...args: unknown[]) => mockCompleteTask(...args),
	reopenTask: (...args: unknown[]) => mockReopenTask(...args),
	snoozeTask: (...args: unknown[]) => mockSnoozeTask(...args),
	quickAddTask: (...args: unknown[]) => mockQuickAddTask(...args)
}));

vi.mock('$lib/stores/namespace', () => ({
	activeNamespace: { subscribe: vi.fn((fn: (v: string | null) => void) => { fn(null); return () => {}; }) }
}));

vi.mock('$lib/stores/undo', () => ({
	pushUndo: vi.fn().mockReturnValue('undo-id')
}));

import type { TaskRecord } from '$lib/db';
import {
	tasksStore,
	pendingSyncCount,
	createTaskOptimistic,
	updateTaskOptimistic,
	completeTaskOptimistic,
	deleteTaskOptimistic,
	syncQueue,
	mergeWithConflictDetection
} from './tasks';

function makeTask(overrides: Partial<TaskRecord> = {}): TaskRecord {
	return {
		id: crypto.randomUUID(),
		title: 'Test Task',
		description: null,
		status: 'inbox',
		priority: 3,
		due_at: null,
		estimate_min: null,
		labels: [],
		assignee: null,
		dependencies: [],
		recurrence: null,
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString(),
		completed_at: null,
		metadata: {},
		...overrides
	};
}

function setOnline(online: boolean) {
	Object.defineProperty(navigator, 'onLine', {
		value: online,
		writable: true,
		configurable: true
	});
}

beforeEach(() => {
	tasksStore.set([]);
	vi.clearAllMocks();
	mockQueueTable.count.mockResolvedValue(0);
	mockQueueTable.orderBy.mockReturnValue({
		toArray: vi.fn().mockResolvedValue([])
	});
	setOnline(true);
});

describe('offline task queuing', () => {
	it('queues create locally when navigator.onLine is false', async () => {
		setOnline(false);

		const result = await createTaskOptimistic({ title: 'Offline Task' });

		// Should NOT call remote API
		expect(mockCreateTask).not.toHaveBeenCalled();

		// Should store in local DB
		expect(mockTasksTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				title: 'Offline Task',
				pending: true,
				localOnly: true
			})
		);

		// Should enqueue an offline op
		expect(mockQueueTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				op: 'create',
				payload: expect.objectContaining({ title: 'Offline Task' })
			})
		);

		// Should appear in the store
		const tasks = get(tasksStore);
		expect(tasks).toHaveLength(1);
		expect(tasks[0].title).toBe('Offline Task');
		expect(tasks[0].pending).toBe(true);

		// Return value should have localOnly flag
		expect((result as TaskRecord).localOnly).toBe(true);
	});

	it('queues update locally when offline', async () => {
		setOnline(false);

		const existing = makeTask({ id: 'task-1', title: 'Original' });
		tasksStore.set([existing]);

		await updateTaskOptimistic('task-1', { title: 'Updated Offline' });

		// Should NOT call remote API
		expect(mockUpdateTask).not.toHaveBeenCalled();

		// Should store updated task in Dexie
		expect(mockTasksTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'task-1',
				title: 'Updated Offline',
				pending: true
			})
		);

		// Should enqueue offline op
		expect(mockQueueTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				op: 'update',
				taskId: 'task-1',
				payload: expect.objectContaining({ title: 'Updated Offline' })
			})
		);

		// Store should reflect updated title
		const tasks = get(tasksStore);
		expect(tasks[0].title).toBe('Updated Offline');
	});

	it('queues complete locally when offline', async () => {
		setOnline(false);

		const existing = makeTask({ id: 'task-2', status: 'inbox' });
		tasksStore.set([existing]);

		await completeTaskOptimistic('task-2');

		expect(mockCompleteTask).not.toHaveBeenCalled();

		expect(mockTasksTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'task-2',
				status: 'done',
				pending: true
			})
		);

		expect(mockQueueTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				op: 'complete',
				taskId: 'task-2'
			})
		);

		const tasks = get(tasksStore);
		expect(tasks[0].status).toBe('done');
		expect(tasks[0].completed_at).toBeTruthy();
	});

	it('queues delete locally when offline', async () => {
		setOnline(false);

		const existing = makeTask({ id: 'task-3', title: 'To Delete' });
		tasksStore.set([existing]);

		await deleteTaskOptimistic('task-3');

		expect(mockDeleteTask).not.toHaveBeenCalled();

		expect(mockQueueTable.put).toHaveBeenCalledWith(
			expect.objectContaining({
				op: 'delete',
				taskId: 'task-3'
			})
		);

		// Task should be removed from store
		expect(get(tasksStore)).toHaveLength(0);
	});
});

describe('sync queue processing', () => {
	it('processes queued tasks when connection returns', async () => {
		setOnline(true);

		const createdTask = makeTask({ id: 'server-id', title: 'Synced Task' });
		mockCreateTask.mockResolvedValue(createdTask);

		const queuedOps = [
			{
				id: 'op-1',
				op: 'create',
				localId: 'local-1',
				payload: { title: 'Synced Task' },
				createdAt: new Date().toISOString()
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		// Pre-populate store with local task
		tasksStore.set([makeTask({ id: 'local-1', title: 'Synced Task', pending: true, localOnly: true })]);

		await syncQueue();

		// Should call remote API
		expect(mockCreateTask).toHaveBeenCalledWith({ title: 'Synced Task' });

		// Should delete local task and replace with server version
		expect(mockTasksTable.delete).toHaveBeenCalledWith('local-1');
		expect(mockTasksTable.put).toHaveBeenCalledWith(createdTask);

		// Should remove processed op from queue
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-1');
	});

	it('processes update ops in the queue', async () => {
		setOnline(true);

		mockUpdateTask.mockResolvedValue(null);

		const queuedOps = [
			{
				id: 'op-2',
				op: 'update',
				taskId: 'task-10',
				payload: { title: 'New Title' },
				createdAt: new Date().toISOString()
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		await syncQueue();

		expect(mockUpdateTask).toHaveBeenCalledWith('task-10', { title: 'New Title' });
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-2');
	});

	it('processes complete ops in the queue', async () => {
		setOnline(true);

		mockCompleteTask.mockResolvedValue(makeTask({ id: 'task-11', status: 'done' }));

		const queuedOps = [
			{
				id: 'op-3',
				op: 'complete',
				taskId: 'task-11',
				createdAt: new Date().toISOString()
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		await syncQueue();

		expect(mockCompleteTask).toHaveBeenCalledWith('task-11');
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-3');
	});

	it('does not sync when offline', async () => {
		setOnline(false);

		await syncQueue();

		// Should not even read the queue
		expect(mockQueueTable.orderBy).not.toHaveBeenCalled();
	});
});

describe('sync error resilience', () => {
	it('API errors during sync do not lose queued tasks', async () => {
		setOnline(true);

		mockCreateTask.mockRejectedValue(new Error('Network error'));

		const queuedOps = [
			{
				id: 'op-fail',
				op: 'create',
				localId: 'local-fail',
				payload: { title: 'Failing Task' },
				createdAt: new Date().toISOString()
			},
			{
				id: 'op-next',
				op: 'create',
				localId: 'local-next',
				payload: { title: 'Next Task' },
				createdAt: new Date().toISOString()
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		await syncQueue();

		// The first op should NOT be deleted from queue (it failed)
		expect(mockQueueTable.delete).not.toHaveBeenCalledWith('op-fail');

		// The second op should also NOT be processed (sync breaks on first failure)
		expect(mockQueueTable.delete).not.toHaveBeenCalledWith('op-next');
	});

	it('successfully synced ops are removed even if later ops fail', async () => {
		setOnline(true);

		const createdTask = makeTask({ id: 'server-1', title: 'Good Task' });
		mockCreateTask.mockResolvedValueOnce(createdTask);
		mockUpdateTask.mockRejectedValueOnce(new Error('Server error'));

		const queuedOps = [
			{
				id: 'op-good',
				op: 'create',
				localId: 'local-good',
				payload: { title: 'Good Task' },
				createdAt: '2025-01-01T00:00:00Z'
			},
			{
				id: 'op-bad',
				op: 'update',
				taskId: 'task-bad',
				payload: { title: 'Bad Task' },
				createdAt: '2025-01-01T00:01:00Z'
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		// Pre-populate store with local task
		tasksStore.set([makeTask({ id: 'local-good', title: 'Good Task', pending: true })]);

		await syncQueue();

		// First op succeeded and should be removed
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-good');

		// Second op failed and should NOT be removed
		expect(mockQueueTable.delete).not.toHaveBeenCalledWith('op-bad');
	});

	it('updates pendingSyncCount after sync attempt', async () => {
		setOnline(true);

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue([])
		});
		mockQueueTable.count.mockResolvedValue(0);

		await syncQueue();

		expect(mockQueueTable.count).toHaveBeenCalled();
		expect(get(pendingSyncCount)).toBe(0);
	});

	it('pendingSyncCount reflects remaining ops after partial sync failure', async () => {
		setOnline(true);

		mockCreateTask.mockRejectedValue(new Error('fail'));

		const queuedOps = [
			{
				id: 'op-x',
				op: 'create',
				localId: 'local-x',
				payload: { title: 'X' },
				createdAt: new Date().toISOString()
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});
		// After failed sync, 1 op remains
		mockQueueTable.count.mockResolvedValue(1);

		await syncQueue();

		expect(get(pendingSyncCount)).toBe(1);
	});
});

describe('concurrent edit handling', () => {
	it('sync loop handles multiple sequential ops gracefully', async () => {
		setOnline(true);

		const task1 = makeTask({ id: 'server-a', title: 'Task A' });
		const task2 = makeTask({ id: 'server-b', title: 'Task B' });
		mockCreateTask.mockResolvedValueOnce(task1).mockResolvedValueOnce(task2);

		const queuedOps = [
			{
				id: 'op-a',
				op: 'create',
				localId: 'local-a',
				payload: { title: 'Task A' },
				createdAt: '2025-01-01T00:00:00Z'
			},
			{
				id: 'op-b',
				op: 'create',
				localId: 'local-b',
				payload: { title: 'Task B' },
				createdAt: '2025-01-01T00:01:00Z'
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		tasksStore.set([
			makeTask({ id: 'local-a', title: 'Task A', pending: true, localOnly: true }),
			makeTask({ id: 'local-b', title: 'Task B', pending: true, localOnly: true })
		]);

		await syncQueue();

		// Both should be created
		expect(mockCreateTask).toHaveBeenCalledTimes(2);

		// Both should be removed from queue
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-a');
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-b');

		// Store should have replaced local IDs with server IDs
		const tasks = get(tasksStore);
		expect(tasks.some((t) => t.id === 'server-a')).toBe(true);
		expect(tasks.some((t) => t.id === 'server-b')).toBe(true);
	});

	it('skips ops with missing required fields', async () => {
		setOnline(true);

		const queuedOps = [
			{
				id: 'op-no-payload',
				op: 'create',
				localId: 'local-x',
				payload: undefined,
				createdAt: new Date().toISOString()
			},
			{
				id: 'op-no-taskid',
				op: 'update',
				taskId: undefined,
				payload: { title: 'Hello' },
				createdAt: new Date().toISOString()
			}
		];

		mockQueueTable.orderBy.mockReturnValue({
			toArray: vi.fn().mockResolvedValue(queuedOps)
		});

		await syncQueue();

		// API should not be called for either
		expect(mockCreateTask).not.toHaveBeenCalled();
		expect(mockUpdateTask).not.toHaveBeenCalled();

		// Both ops should still be removed from queue (they were "processed" via break/skip)
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-no-payload');
		expect(mockQueueTable.delete).toHaveBeenCalledWith('op-no-taskid');
	});
});

describe('mergeWithConflictDetection', () => {
	it('accepts remote when no local version exists', () => {
		const remote = makeTask({ id: 'r1', title: 'Remote Only' });
		const result = mergeWithConflictDetection([], [remote]);
		expect(result).toHaveLength(1);
		expect(result[0].id).toBe('r1');
	});

	it('accepts remote when local is not pending', () => {
		const local = makeTask({ id: 'shared', title: 'Old Title', pending: false, updated_at: '2025-01-01T00:00:00Z' });
		const remote = makeTask({ id: 'shared', title: 'New Title', updated_at: '2025-01-02T00:00:00Z' });
		const result = mergeWithConflictDetection([local], [remote]);
		expect(result).toHaveLength(1);
		expect(result[0].title).toBe('New Title');
	});

	it('accepts remote when remote is newer and local is pending', () => {
		const local = makeTask({
			id: 'shared',
			title: 'Local Edit',
			pending: true,
			updated_at: '2025-01-01T00:00:00Z'
		});
		const remote = makeTask({
			id: 'shared',
			title: 'Remote Edit',
			updated_at: '2025-01-02T00:00:00Z'
		});

		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
		const result = mergeWithConflictDetection([local], [remote]);

		expect(result).toHaveLength(1);
		expect(result[0].title).toBe('Remote Edit');
		expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('remote wins'));
		warnSpy.mockRestore();
	});

	it('keeps local when local is newer and pending', () => {
		const local = makeTask({
			id: 'shared',
			title: 'Local Edit',
			pending: true,
			updated_at: '2025-01-03T00:00:00Z'
		});
		const remote = makeTask({
			id: 'shared',
			title: 'Remote Edit',
			updated_at: '2025-01-02T00:00:00Z'
		});

		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
		const result = mergeWithConflictDetection([local], [remote]);

		expect(result).toHaveLength(1);
		expect(result[0].title).toBe('Local Edit');
		expect(result[0].pending).toBe(true);
		expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('local wins'));
		warnSpy.mockRestore();
	});

	it('keeps local when timestamps are equal and local is pending', () => {
		const ts = '2025-01-01T12:00:00Z';
		const local = makeTask({
			id: 'shared',
			title: 'Local Edit',
			pending: true,
			updated_at: ts
		});
		const remote = makeTask({
			id: 'shared',
			title: 'Remote Edit',
			updated_at: ts
		});

		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
		const result = mergeWithConflictDetection([local], [remote]);

		expect(result).toHaveLength(1);
		expect(result[0].title).toBe('Local Edit');
		warnSpy.mockRestore();
	});

	it('preserves local-only tasks not present on remote', () => {
		const localOnly = makeTask({
			id: 'local-only',
			title: 'Offline Created',
			localOnly: true
		});
		const remote = makeTask({ id: 'remote-1', title: 'Server Task' });

		const result = mergeWithConflictDetection([localOnly], [remote]);
		expect(result).toHaveLength(2);
		expect(result.find((t) => t.id === 'local-only')?.title).toBe('Offline Created');
		expect(result.find((t) => t.id === 'remote-1')?.title).toBe('Server Task');
	});

	it('does not preserve non-localOnly tasks missing from remote', () => {
		const deletedOnServer = makeTask({
			id: 'gone',
			title: 'Was Deleted',
			localOnly: false
		});
		const remote = makeTask({ id: 'still-here', title: 'Still Here' });

		const result = mergeWithConflictDetection([deletedOnServer], [remote]);
		expect(result).toHaveLength(1);
		expect(result[0].id).toBe('still-here');
	});

	it('handles empty inputs', () => {
		expect(mergeWithConflictDetection([], [])).toEqual([]);
	});

	it('logs conflicts for debugging', () => {
		const local = makeTask({
			id: 'x',
			title: 'T',
			pending: true,
			updated_at: '2025-06-01T00:00:00Z'
		});
		const remote = makeTask({
			id: 'x',
			title: 'T',
			updated_at: '2025-01-01T00:00:00Z'
		});

		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
		mergeWithConflictDetection([local], [remote]);

		expect(warnSpy).toHaveBeenCalledTimes(1);
		expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('[sync] conflict'));
		warnSpy.mockRestore();
	});
});
