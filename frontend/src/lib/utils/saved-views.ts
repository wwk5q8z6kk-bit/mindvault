import type { SavedView, SavedViewSort } from '$lib/api/saved-views';
import type { TaskRecord } from '$lib/db';
import type { TaskStatus } from '$lib/api/tasks';

export type TaskFilterSpec = {
	status: TaskStatus | 'all';
	query: string;
	tags: string[];
	sort?: SavedViewSort | null;
};

function readStringArray(value: unknown): string[] {
	if (Array.isArray(value)) {
		return value.filter((item): item is string => typeof item === 'string').map((item) => item.trim());
	}
	if (typeof value === 'string') return [value.trim()].filter(Boolean);
	return [];
}

export function taskFilterFromSavedView(view: SavedView): TaskFilterSpec {
	const filters = (view.filters ?? {}) as Record<string, unknown>;
	const statuses = readStringArray(filters.status ?? filters.statuses).filter(Boolean) as TaskStatus[];
	const tags = readStringArray(filters.tags);
	const status = statuses.length > 0 ? statuses[0] : 'all';
	const query = (view.query ?? '').trim();
	return {
		status,
		query,
		tags,
		sort: view.sort ?? null
	};
}

export function filterTasksBySavedView(
	tasks: TaskRecord[],
	view: SavedView | null,
	allowedView: SavedView['view_type']
): TaskRecord[] {
	if (!view || view.view_type !== allowedView) return tasks;
	const filter = taskFilterFromSavedView(view);
	let items = tasks;
	if (filter.status !== 'all') {
		items = items.filter((task) => task.status === filter.status);
	}
	if (filter.tags.length > 0) {
		items = items.filter((task) => filter.tags.every((tag) => task.labels?.includes(tag)));
	}
	if (filter.query) {
		const q = filter.query.toLowerCase();
		items = items.filter((task) => task.title.toLowerCase().includes(q));
	}
	return items;
}
