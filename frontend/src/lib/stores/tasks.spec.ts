import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { tasksStore, taskFilter, filteredTasks, type TaskView } from './tasks';
import type { TaskRecord } from '$lib/db';

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

beforeEach(() => {
	tasksStore.set([]);
	taskFilter.set({ status: 'all', query: '', view: 'all', tags: [], sort: null });
});

describe('tasksStore', () => {
	it('starts empty', () => {
		expect(get(tasksStore)).toEqual([]);
	});

	it('holds tasks when set', () => {
		const task = makeTask();
		tasksStore.set([task]);
		expect(get(tasksStore)).toHaveLength(1);
		expect(get(tasksStore)[0].id).toBe(task.id);
	});
});

describe('filteredTasks', () => {
	it('returns all non-trashed tasks with default filter', () => {
		const tasks = [makeTask({ title: 'Alpha' }), makeTask({ title: 'Beta' })];
		tasksStore.set(tasks);
		expect(get(filteredTasks)).toHaveLength(2);
	});

	it('excludes trashed tasks', () => {
		const tasks = [
			makeTask({ title: 'Visible' }),
			makeTask({ title: 'Trashed', labels: ['trashed'] })
		];
		tasksStore.set(tasks);
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Visible');
	});

	it('filters by status', () => {
		const tasks = [
			makeTask({ title: 'Inbox', status: 'inbox' }),
			makeTask({ title: 'Done', status: 'done' })
		];
		tasksStore.set(tasks);
		taskFilter.update((f) => ({ ...f, status: 'done' }));
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Done');
	});

	it('filters by query text', () => {
		const tasks = [makeTask({ title: 'Buy groceries' }), makeTask({ title: 'Write tests' })];
		tasksStore.set(tasks);
		taskFilter.update((f) => ({ ...f, query: 'groceries' }));
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Buy groceries');
	});

	it('filters by tags', () => {
		const tasks = [
			makeTask({ title: 'Work', labels: ['work', 'urgent'] }),
			makeTask({ title: 'Personal', labels: ['personal'] })
		];
		tasksStore.set(tasks);
		taskFilter.update((f) => ({ ...f, tags: ['work'] }));
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Work');
	});

	it('requires all tags to match', () => {
		const tasks = [
			makeTask({ title: 'Both', labels: ['work', 'urgent'] }),
			makeTask({ title: 'One', labels: ['work'] })
		];
		tasksStore.set(tasks);
		taskFilter.update((f) => ({ ...f, tags: ['work', 'urgent'] }));
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Both');
	});

	it('view=inbox shows only inbox status', () => {
		const tasks = [
			makeTask({ title: 'Inbox', status: 'inbox' }),
			makeTask({ title: 'Planned', status: 'planned' })
		];
		tasksStore.set(tasks);
		taskFilter.update((f) => ({ ...f, view: 'inbox' as TaskView }));
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Inbox');
	});

	it('view=today shows tasks due today', () => {
		const todayStr = new Date().toISOString();
		const tomorrowStr = new Date(Date.now() + 2 * 86400000).toISOString();
		const tasks = [
			makeTask({ title: 'Due Today', due_at: todayStr }),
			makeTask({ title: 'Due Later', due_at: tomorrowStr }),
			makeTask({ title: 'No Due', due_at: null })
		];
		tasksStore.set(tasks);
		taskFilter.update((f) => ({ ...f, view: 'today' as TaskView }));
		expect(get(filteredTasks)).toHaveLength(1);
		expect(get(filteredTasks)[0].title).toBe('Due Today');
	});

	it('sorts pinned tasks first', () => {
		const pinned = makeTask({ title: 'Pinned', metadata: { pinned: true } });
		const unpinned = makeTask({ title: 'Unpinned', metadata: {} });
		tasksStore.set([unpinned, pinned]);
		const result = get(filteredTasks);
		expect(result[0].title).toBe('Pinned');
		expect(result[1].title).toBe('Unpinned');
	});

	it('sorts by due date by default', () => {
		const sooner = makeTask({ title: 'Sooner', due_at: '2025-01-01T00:00:00Z' });
		const later = makeTask({ title: 'Later', due_at: '2025-06-01T00:00:00Z' });
		const noDue = makeTask({ title: 'NoDue', due_at: null });
		tasksStore.set([noDue, later, sooner]);
		const result = get(filteredTasks);
		expect(result[0].title).toBe('Sooner');
		expect(result[1].title).toBe('Later');
		expect(result[2].title).toBe('NoDue');
	});

	it('sorts by priority when specified', () => {
		const low = makeTask({ title: 'Low', priority: 5 });
		const high = makeTask({ title: 'High', priority: 1 });
		tasksStore.set([low, high]);
		taskFilter.update((f) => ({ ...f, sort: { field: 'priority', direction: 'asc' } }));
		const result = get(filteredTasks);
		expect(result[0].title).toBe('High');
		expect(result[1].title).toBe('Low');
	});

	it('sorts by title when specified', () => {
		const banana = makeTask({ title: 'Banana' });
		const apple = makeTask({ title: 'Apple' });
		tasksStore.set([banana, apple]);
		taskFilter.update((f) => ({ ...f, sort: { field: 'title', direction: 'asc' } }));
		const result = get(filteredTasks);
		expect(result[0].title).toBe('Apple');
		expect(result[1].title).toBe('Banana');
	});
});
