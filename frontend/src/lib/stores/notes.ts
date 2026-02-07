import { writable, derived, get } from 'svelte/store';
import type { Note } from '$lib/api/notes';
import { listNotes, createNote, updateNote, deleteNote } from '$lib/api/notes';
import { db } from '$lib/db';
import { activeNamespace } from '$lib/stores/namespace';
import { pushUndo } from '$lib/stores/undo';

export const notesStore = writable<Note[]>([]);
export const noteSearchQuery = writable('');

export const filteredNotes = derived([notesStore, noteSearchQuery], ([$notes, $query]) => {
	let items = $notes.filter((n) => !n.tags?.includes('trashed'));
	if ($query.trim()) {
		const q = $query.toLowerCase();
		items = items.filter(
			(n) =>
				(n.title ?? '').toLowerCase().includes(q) ||
				n.markdown.toLowerCase().includes(q) ||
				n.tags.some((t) => t.toLowerCase().includes(q))
		);
	}
	return items.sort((a, b) => {
		// Pinned first, then by updated_at descending
		if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
		return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
	});
});

export async function loadNotes(): Promise<void> {
	try {
		const ns = get(activeNamespace);
		const items = await listNotes(50, ns);
		await db.notes.clear();
		await db.notes.bulkPut(items);
		notesStore.set(items);
	} catch {
		const cached = await db.notes.toArray();
		if (cached.length) {
			notesStore.set(cached);
		}
	}
}

export async function createNoteOptimistic(markdown: string, title?: string): Promise<Note> {
	const created = await createNote(markdown, title);
	await db.notes.put(created);
	notesStore.update((items) => [created, ...items]);
	return created;
}

export async function updateNoteOptimistic(
	noteId: string,
	payload: { markdown?: string; title?: string; pinned?: boolean }
): Promise<void> {
	// Optimistic update
	notesStore.update((items) =>
		items.map((n) =>
			n.id === noteId ? { ...n, ...payload, updated_at: new Date().toISOString() } : n
		)
	);
	try {
		const updated = await updateNote(noteId, payload);
		if (updated) await db.notes.put(updated);
	} catch {
		// Revert on failure
		await loadNotes();
		throw new Error('Failed to update note');
	}
}

export async function trashNoteOptimistic(noteId: string): Promise<void> {
	const items = get(notesStore);
	const existing = items.find((n) => n.id === noteId);
	if (!existing) return;
	const newTags = [...(existing.tags ?? []).filter((t) => t !== 'trashed'), 'trashed'];
	const newMeta = { ...existing.metadata, trashed_at: new Date().toISOString() };
	notesStore.update((list) =>
		list.map((n) =>
			n.id === noteId ? { ...n, tags: newTags, metadata: newMeta, updated_at: new Date().toISOString() } : n
		)
	);
	try {
		const updated = await updateNote(noteId, { tags: newTags });
		if (updated) await db.notes.put(updated);
	} catch {
		await loadNotes();
	}
}

export async function restoreNoteOptimistic(noteId: string): Promise<void> {
	const items = get(notesStore);
	const existing = items.find((n) => n.id === noteId);
	if (!existing) return;
	const newTags = (existing.tags ?? []).filter((t) => t !== 'trashed');
	const { trashed_at: _, ...restMeta } = (existing.metadata ?? {}) as Record<string, unknown>;
	notesStore.update((list) =>
		list.map((n) =>
			n.id === noteId ? { ...n, tags: newTags, metadata: restMeta, updated_at: new Date().toISOString() } : n
		)
	);
	try {
		const updated = await updateNote(noteId, { tags: newTags });
		if (updated) await db.notes.put(updated);
	} catch {
		await loadNotes();
	}
}

export async function deleteNoteOptimistic(noteId: string): Promise<string | undefined> {
	const items = get(notesStore);
	const existing = items.find((n) => n.id === noteId);
	const snapshot = existing ? { ...existing } : undefined;

	notesStore.update((items) => items.filter((n) => n.id !== noteId));
	await db.notes.delete(noteId);

	let undoId: string | undefined;
	if (snapshot) {
		undoId = pushUndo(
			`Delete "${snapshot.title ?? 'note'}"`,
			async () => {
				await db.notes.put(snapshot);
				notesStore.update((list) => [snapshot, ...list]);
				try {
					await createNote(snapshot.markdown, snapshot.title ?? undefined);
				} catch {
					// local restore still works
				}
			},
			async () => {
				await deleteNoteOptimistic(noteId);
			},
			'note'
		);
	}

	try {
		await deleteNote(noteId);
	} catch {
		await loadNotes();
		throw new Error('Failed to delete note');
	}
	return undoId;
}
