import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'mv-view-preferences';

interface ViewPreferences {
	tasks: string;
	notes: string;
	review: string;
	resources: string;
	sidebarCollapsed: string[];
}

const defaults: ViewPreferences = {
	tasks: 'list',
	notes: 'list',
	review: 'digest',
	resources: 'bookmarks',
	sidebarCollapsed: ['System']
};

function load(): ViewPreferences {
	if (!browser) return { ...defaults };
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return { ...defaults };
		const parsed = JSON.parse(raw);
		return { ...defaults, ...parsed };
	} catch {
		return { ...defaults };
	}
}

function save(prefs: ViewPreferences) {
	if (!browser) return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
	} catch {
		// storage full or unavailable
	}
}

const initial = load();

export const viewPreferences = writable<ViewPreferences>(initial);

// Auto-save on change
viewPreferences.subscribe((prefs) => {
	save(prefs);
});

/**
 * Update a single view preference.
 */
export function setViewPreference(key: keyof Omit<ViewPreferences, 'sidebarCollapsed'>, value: string) {
	viewPreferences.update((prefs) => ({ ...prefs, [key]: value }));
}

/**
 * Toggle a sidebar group's collapsed state.
 */
export function toggleSidebarGroup(group: string) {
	viewPreferences.update((prefs) => {
		const collapsed = prefs.sidebarCollapsed.includes(group)
			? prefs.sidebarCollapsed.filter((g) => g !== group)
			: [...prefs.sidebarCollapsed, group];
		return { ...prefs, sidebarCollapsed: collapsed };
	});
}

/**
 * Check if a sidebar group is collapsed.
 */
export function isSidebarGroupCollapsed(collapsed: string[], group: string): boolean {
	return collapsed.includes(group);
}
