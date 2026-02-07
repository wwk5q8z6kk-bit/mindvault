import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { notesStore, noteSearchQuery, filteredNotes } from './notes';
import type { Note } from '$lib/api/notes';

function makeNote(overrides: Partial<Note> = {}): Note {
	return {
		id: crypto.randomUUID(),
		markdown: '# Test Note\n\nSome content here.',
		title: 'Test Note',
		tags: [],
		backlinks: [],
		pinned: false,
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString(),
		metadata: {},
		...overrides
	};
}

beforeEach(() => {
	notesStore.set([]);
	noteSearchQuery.set('');
});

describe('notesStore', () => {
	it('starts empty', () => {
		expect(get(notesStore)).toEqual([]);
	});

	it('holds notes when set', () => {
		const note = makeNote();
		notesStore.set([note]);
		expect(get(notesStore)).toHaveLength(1);
		expect(get(notesStore)[0].id).toBe(note.id);
	});
});

describe('filteredNotes', () => {
	it('returns all notes when no query', () => {
		const notes = [makeNote({ title: 'Alpha' }), makeNote({ title: 'Beta' })];
		notesStore.set(notes);
		expect(get(filteredNotes)).toHaveLength(2);
	});

	it('filters by title', () => {
		const notes = [makeNote({ title: 'Alpha' }), makeNote({ title: 'Beta' })];
		notesStore.set(notes);
		noteSearchQuery.set('alpha');
		expect(get(filteredNotes)).toHaveLength(1);
		expect(get(filteredNotes)[0].title).toBe('Alpha');
	});

	it('filters by markdown content', () => {
		const notes = [
			makeNote({ title: 'Note 1', markdown: 'This is about Rust programming' }),
			makeNote({ title: 'Note 2', markdown: 'This is about cooking' })
		];
		notesStore.set(notes);
		noteSearchQuery.set('rust');
		expect(get(filteredNotes)).toHaveLength(1);
	});

	it('filters by tags', () => {
		const notes = [
			makeNote({ title: 'Note 1', tags: ['rust', 'code'] }),
			makeNote({ title: 'Note 2', tags: ['cooking'] })
		];
		notesStore.set(notes);
		noteSearchQuery.set('rust');
		expect(get(filteredNotes)).toHaveLength(1);
	});

	it('sorts pinned notes first', () => {
		const unpinned = makeNote({ title: 'Unpinned', pinned: false, updated_at: new Date().toISOString() });
		const pinned = makeNote({ title: 'Pinned', pinned: true, updated_at: new Date(Date.now() - 100000).toISOString() });
		notesStore.set([unpinned, pinned]);
		const result = get(filteredNotes);
		expect(result[0].title).toBe('Pinned');
		expect(result[1].title).toBe('Unpinned');
	});

	it('sorts by updated_at within same pin status', () => {
		const older = makeNote({ title: 'Older', updated_at: new Date(2024, 0, 1).toISOString() });
		const newer = makeNote({ title: 'Newer', updated_at: new Date(2025, 0, 1).toISOString() });
		notesStore.set([older, newer]);
		const result = get(filteredNotes);
		expect(result[0].title).toBe('Newer');
		expect(result[1].title).toBe('Older');
	});
});
