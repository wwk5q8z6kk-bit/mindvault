import { writable, get } from 'svelte/store';
import { goto } from '$app/navigation';
import { tasksStore, loadTasks, syncQueue, updateTaskOptimistic } from '$lib/stores/tasks';
import { quickAddFocus, selectedNoteId, selectedTaskId, taskModalState } from '$lib/stores/ui';
import { pushToast } from '$lib/stores/toast';
import { searchFts } from '$lib/api/search';
import type { CommandContext } from './types';
import type { SearchResult } from './types';

export const paletteOpen = writable(false);
export const paletteQuery = writable('');

export function openPalette(initialQuery = '') {
	paletteQuery.set(initialQuery);
	paletteOpen.set(true);
}

export function closePalette() {
	paletteOpen.set(false);
	paletteQuery.set('');
}

export function buildCommandContext(query: string): CommandContext {
	return {
		query,
		selectedTaskId: get(selectedTaskId),
		selectedNoteId: get(selectedNoteId),
		openTaskModal: (mode, taskId = null) => {
			taskModalState.set({ open: true, mode, taskId });
		},
		focusQuickAdd: () => {
			const fn = get(quickAddFocus);
			fn?.();
		},
		setQuery: (value) => paletteQuery.set(value),
		closePalette: () => closePalette(),
		navigate: async (path) => {
			await goto(path as any);
		},
		refreshData: async () => {
			await syncQueue();
			await loadTasks();
		},
		updateTaskStatus: async (taskId, status) => {
			await updateTaskOptimistic(taskId, { status: status as any });
		},
		addTaskLabel: async (taskId, label) => {
			const tasks = get(tasksStore);
			const task = tasks.find((item) => item.id === taskId);
			if (!task) return;
			const labels = Array.from(new Set([...(task.labels ?? []), label]));
			await updateTaskOptimistic(taskId, { labels });
		},
		selectTask: (taskId) => {
			selectedTaskId.set(taskId);
		},
		searchFts: async (searchQuery: string): Promise<SearchResult[]> => {
			const results = await searchFts(searchQuery);
			return results.map((r) => ({
				object_type: r.node.kind,
				object_id: r.node.id,
				content: r.node.title
			}));
		},
		toast: (message, variant = 'info') => pushToast(message, variant),
		tasks: get(tasksStore)
	};
}
