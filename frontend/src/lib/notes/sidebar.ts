export type NotesListTab = 'all' | 'pinned' | string;

export interface NotesSidebarNote {
	kind: string;
	pinned?: boolean;
	title?: string | null;
	markdown?: string | null;
	tags?: string[] | null;
}

export function normalizeNotesListTab(value: unknown, allowedKinds: readonly string[] = []): NotesListTab {
	if (typeof value !== 'string') return 'all';
	const tab = value.trim().toLowerCase();
	if (tab === 'all' || tab === 'pinned') return tab;
	if (allowedKinds.includes(tab)) return tab;
	// Allow unknown kinds so persisted tabs survive before notes load.
	if (tab.length > 0 && /^[a-z][a-z0-9_-]*$/.test(tab)) return tab;
	return 'all';
}

export function filterNotesForSidebar<T extends NotesSidebarNote>(
	notes: T[],
	options: {
		tab: NotesListTab;
		searchQuery?: string;
		tagFilter?: string;
	}
): T[] {
	const query = (options.searchQuery ?? '').trim().toLowerCase();
	const tag = (options.tagFilter ?? '').trim();
	const tab = options.tab;

	return notes.filter((note) => {
		if (tab === 'pinned') {
			if (!note.pinned) return false;
		} else if (tab !== 'all' && note.kind !== tab) {
			return false;
		}

		if (query) {
			const title = (note.title ?? '').toLowerCase();
			const content = (note.markdown ?? '').toLowerCase();
			if (!title.includes(query) && !content.includes(query)) return false;
		}

		if (tag && !(note.tags ?? []).includes(tag)) return false;
		return true;
	});
}
