import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'mv_recent_items';
const MAX_ITEMS = 15;

export type RecentItemType = 'note' | 'task' | 'search';

export interface RecentItem {
	id: string;
	type: RecentItemType;
	title: string;
	timestamp: number;
}

function loadRecent(): RecentItem[] {
	if (!browser) return [];
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored) {
			return JSON.parse(stored);
		}
	} catch {
		// ignore
	}
	return [];
}

function saveRecent(items: RecentItem[]) {
	if (!browser) return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(items.slice(0, MAX_ITEMS)));
	} catch {
		// ignore
	}
}

function createRecentStore() {
	const { subscribe, set, update } = writable<RecentItem[]>(loadRecent());

	return {
		subscribe,

		addNote(id: string, title: string) {
			update(items => {
				const filtered = items.filter(i => !(i.type === 'note' && i.id === id));
				const newItems = [{ id, type: 'note' as RecentItemType, title, timestamp: Date.now() }, ...filtered].slice(0, MAX_ITEMS);
				saveRecent(newItems);
				return newItems;
			});
		},

		addTask(id: string, title: string) {
			update(items => {
				const filtered = items.filter(i => !(i.type === 'task' && i.id === id));
				const newItems = [{ id, type: 'task' as RecentItemType, title, timestamp: Date.now() }, ...filtered].slice(0, MAX_ITEMS);
				saveRecent(newItems);
				return newItems;
			});
		},

		addSearch(query: string) {
			update(items => {
				const filtered = items.filter(i => !(i.type === 'search' && i.title === query));
				const newItems = [{ id: `search-${Date.now()}`, type: 'search' as RecentItemType, title: query, timestamp: Date.now() }, ...filtered].slice(0, MAX_ITEMS);
				saveRecent(newItems);
				return newItems;
			});
		},

		remove(id: string, type: RecentItemType) {
			update(items => {
				const filtered = items.filter(i => !(i.type === type && i.id === id));
				saveRecent(filtered);
				return filtered;
			});
		},

		clear() {
			set([]);
			saveRecent([]);
		}
	};
}

export const recentItems = createRecentStore();

export const recentNotes = derived(recentItems, $items =>
	$items.filter(i => i.type === 'note').slice(0, 5)
);

export const recentTasks = derived(recentItems, $items =>
	$items.filter(i => i.type === 'task').slice(0, 5)
);

export const recentSearches = derived(recentItems, $items =>
	$items.filter(i => i.type === 'search').slice(0, 5)
);
