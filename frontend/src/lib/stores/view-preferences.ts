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

function normalizeViewValue(value: unknown, fallback: string): string {
	if (typeof value !== 'string') return fallback;
	const normalized = value.trim();
	return normalized.length > 0 ? normalized : fallback;
}

function normalizeCollapsedGroups(value: unknown): string[] {
	if (!Array.isArray(value)) {
		return [...defaults.sidebarCollapsed];
	}

	const normalized = Array.from(
		new Set(
			value
				.filter((entry): entry is string => typeof entry === 'string')
				.map((entry) => entry.trim())
				.filter((entry) => entry.length > 0)
		)
	);

	return normalized.length > 0 ? normalized : [...defaults.sidebarCollapsed];
}

export function normalizeViewPreferences(raw: unknown): ViewPreferences {
	if (!raw || typeof raw !== 'object') {
		return { ...defaults };
	}

	const parsed = raw as Record<string, unknown>;
	return {
		tasks: normalizeViewValue(parsed.tasks, defaults.tasks),
		notes: normalizeViewValue(parsed.notes, defaults.notes),
		review: normalizeViewValue(parsed.review, defaults.review),
		resources: normalizeViewValue(parsed.resources, defaults.resources),
		sidebarCollapsed: normalizeCollapsedGroups(parsed.sidebarCollapsed)
	};
}

function load(): ViewPreferences {
	if (!browser) return { ...defaults };
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return { ...defaults };
		return normalizeViewPreferences(JSON.parse(raw));
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
if (browser) {
	viewPreferences.subscribe((prefs) => {
		save(prefs);
	});
}

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
