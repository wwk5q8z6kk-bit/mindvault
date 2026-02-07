import { derived, get, writable } from 'svelte/store';

export type SelectionKind = 'task' | 'note';

export type SelectionEntry = {
	kind: SelectionKind;
	id: string;
};

const selectionStore = writable<SelectionEntry[]>([]);

const anchors = new Map<SelectionKind, string>();

export const selectionByKind = derived(selectionStore, (entries) => {
	const map: Record<SelectionKind, Set<string>> = {
		task: new Set<string>(),
		note: new Set<string>()
	};
	for (const entry of entries) {
		map[entry.kind].add(entry.id);
	}
	return map;
});

export const selectedCount = derived(selectionStore, (entries) => entries.length);

export const selectedKinds = derived(selectionStore, (entries) => {
	const kinds = new Set<SelectionKind>();
	for (const entry of entries) kinds.add(entry.kind);
	return Array.from(kinds);
});

export function getSelectedIds(kind: SelectionKind): Set<string> {
	return get(selectionByKind)[kind];
}

export function clearSelection(kind?: SelectionKind): void {
	selectionStore.update((entries) => {
		if (!kind) return [];
		return entries.filter((entry) => entry.kind !== kind);
	});
}

export function toggleSelection(kind: SelectionKind, id: string): void {
	selectionStore.update((entries) => {
		const existing = entries.find((entry) => entry.kind === kind && entry.id === id);
		if (existing) {
			return entries.filter((entry) => !(entry.kind === kind && entry.id === id));
		}
		return [...entries, { kind, id }];
	});
	anchors.set(kind, id);
}

export function replaceSelection(kind: SelectionKind, ids: string[]): void {
	selectionStore.update((entries) => {
		const preserved = entries.filter((entry) => entry.kind !== kind);
		const next = ids.map((id) => ({ kind, id }));
		return [...preserved, ...next];
	});
	if (ids.length > 0) anchors.set(kind, ids[ids.length - 1]);
}

export function selectRange(kind: SelectionKind, orderedIds: string[], targetId: string): void {
	const anchor = anchors.get(kind) ?? targetId;
	const startIndex = orderedIds.indexOf(anchor);
	const endIndex = orderedIds.indexOf(targetId);
	if (startIndex === -1 || endIndex === -1) {
		toggleSelection(kind, targetId);
		return;
	}
	const [start, end] = startIndex < endIndex ? [startIndex, endIndex] : [endIndex, startIndex];
	const rangeIds = orderedIds.slice(start, end + 1);
	selectionStore.update((entries) => {
		const preserved = entries.filter((entry) => entry.kind !== kind);
		const existingIds = new Set(entries.filter((entry) => entry.kind === kind).map((e) => e.id));
		for (const id of rangeIds) {
			existingIds.add(id);
		}
		const merged = Array.from(existingIds).map((id) => ({ kind, id }));
		return [...preserved, ...merged];
	});
	anchors.set(kind, targetId);
}

export function isSelected(kind: SelectionKind, id: string): boolean {
	return get(selectionByKind)[kind].has(id);
}
