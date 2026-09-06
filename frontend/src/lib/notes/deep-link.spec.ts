import { describe, expect, it } from 'vitest';
import type { KnowledgeNode } from '$lib/api/types';
import { noteSelectionPath, toWorkbenchNote } from './deep-link';

function node(overrides: Partial<KnowledgeNode> = {}): KnowledgeNode {
	return {
		id: 'note-1',
		kind: 'concept',
		title: 'Local-first systems',
		content: 'Keep durable knowledge close to the user.',
		source: 'manual',
		namespace: 'default',
		tags: ['architecture'],
		importance: 0.6,
		temporal: {
			created_at: '2026-07-01T10:00:00.000Z',
			updated_at: '2026-07-02T10:00:00.000Z'
		},
		metadata: { pinned: true },
		...overrides
	};
}

describe('notes deep-link helpers', () => {
	it('maps supported knowledge into the workbench note contract', () => {
		expect(toWorkbenchNote(node())).toMatchObject({
			id: 'note-1',
			kind: 'concept',
			markdown: 'Keep durable knowledge close to the user.',
			pinned: true
		});
	});

	it('keeps tasks and daily entries out of the notes workbench', () => {
		expect(toWorkbenchNote(node({ kind: 'task' }))).toBeNull();
		expect(toWorkbenchNote(node({ kind: 'fact', tags: ['day:2026-07-02'] }))).toBeNull();
	});

	it('creates a canonical selected-note URL and removes graph state', () => {
		const current = new URL('http://mindvault.local/notes?view=graph&node=old&filter=pinned');
		expect(noteSelectionPath(current, 'note 2')).toBe('/notes?filter=pinned&note=note+2');
		expect(noteSelectionPath(current, null)).toBe('/notes?filter=pinned');
	});
});
