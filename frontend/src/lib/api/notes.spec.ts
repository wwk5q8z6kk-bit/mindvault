import { describe, expect, it, vi, beforeEach } from 'vitest';
import type { KnowledgeNode } from './types';

function makeFactNode(overrides: Partial<KnowledgeNode> = {}): KnowledgeNode {
	return {
		id: 'note-1',
		kind: 'fact',
		title: 'Test Note',
		content: '# Hello\n\nSome content',
		source: null,
		namespace: 'default',
		tags: ['journal'],
		importance: 0.5,
		temporal: {
			created_at: '2026-01-15T10:00:00Z',
			updated_at: '2026-01-16T12:00:00Z',
			accessed_at: null,
			expires_at: null
		},
		metadata: {},
		...overrides
	};
}

const fetchJsonMock = vi.fn();
const listNodesMock = vi.fn();
const createNodeMock = vi.fn();
const deleteNodeMock = vi.fn();

vi.mock('./client', () => ({
	fetchJson: (...args: unknown[]) => fetchJsonMock(...args)
}));

vi.mock('./nodes', () => ({
	listNodes: (...args: unknown[]) => listNodesMock(...args),
	createNode: (...args: unknown[]) => createNodeMock(...args),
	deleteNode: (...args: unknown[]) => deleteNodeMock(...args)
}));

describe('notes api', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('listNotes fetches fact nodes and filters out daily notes', async () => {
		const regularNote = makeFactNode({ id: 'n1', tags: ['journal'] });
		const dailyNote = makeFactNode({ id: 'n2', tags: ['day:2026-02-08'] });
		listNodesMock.mockResolvedValue([regularNote, dailyNote]);

		const { listNotes } = await import('./notes');
		const notes = await listNotes();

		expect(listNodesMock).toHaveBeenCalledWith({ kind: 'fact', limit: 50, namespace: undefined });
		expect(notes).toHaveLength(1);
		expect(notes[0].id).toBe('n1');
	});

	it('listNotes passes limit and namespace', async () => {
		listNodesMock.mockResolvedValue([]);

		const { listNotes } = await import('./notes');
		await listNotes(100, 'work');

		expect(listNodesMock).toHaveBeenCalledWith({ kind: 'fact', limit: 100, namespace: 'work' });
	});

	it('createNote with string payload creates a fact node', async () => {
		const created = makeFactNode({ id: 'new-1' });
		createNodeMock.mockResolvedValue(created);

		const { createNote } = await import('./notes');
		const note = await createNote('markdown body', 'My Title');

		expect(createNodeMock).toHaveBeenCalledWith({
			kind: 'fact',
			title: 'My Title',
			content: 'markdown body'
		});
		expect(note.id).toBe('new-1');
		expect(note.markdown).toBe('# Hello\n\nSome content');
	});

	it('createNote with object payload passes all fields', async () => {
		const created = makeFactNode();
		createNodeMock.mockResolvedValue(created);

		const { createNote } = await import('./notes');
		await createNote({
			title: 'Obj Title',
			markdown: 'body',
			namespace: 'work',
			tags: ['tag1'],
			importance: 0.8
		});

		expect(createNodeMock).toHaveBeenCalledWith({
			kind: 'fact',
			title: 'Obj Title',
			content: 'body',
			namespace: 'work',
			tags: ['tag1'],
			importance: 0.8
		});
	});

	it('updateNote fetches existing node, merges, and puts', async () => {
		const existing = makeFactNode({ id: 'u1', content: 'old' });
		const updated = makeFactNode({ id: 'u1', content: 'new content' });
		fetchJsonMock
			.mockResolvedValueOnce(existing) // GET existing
			.mockResolvedValueOnce(updated); // PUT updated

		const { updateNote } = await import('./notes');
		const result = await updateNote('u1', { markdown: 'new content', title: 'New Title' });

		// First call: GET existing node
		expect(fetchJsonMock.mock.calls[0][0]).toBe('/api/v1/nodes/u1');
		// Second call: PUT merged node
		expect(fetchJsonMock.mock.calls[1][0]).toBe('/api/v1/nodes/u1');
		expect(fetchJsonMock.mock.calls[1][1].method).toBe('PUT');
		expect(result.id).toBe('u1');
	});

	it('updateNote sets pinned in metadata', async () => {
		const existing = makeFactNode({ id: 'u2', metadata: {} });
		const updated = makeFactNode({ id: 'u2', metadata: { pinned: true } });
		fetchJsonMock
			.mockResolvedValueOnce(existing)
			.mockResolvedValueOnce(updated);

		const { updateNote } = await import('./notes');
		await updateNote('u2', { pinned: true });

		const putBody = JSON.parse(fetchJsonMock.mock.calls[1][1].body);
		expect(putBody.metadata.pinned).toBe(true);
	});

	it('deleteNote calls deleteNode', async () => {
		deleteNodeMock.mockResolvedValue(undefined);

		const { deleteNote } = await import('./notes');
		await deleteNote('d1');

		expect(deleteNodeMock).toHaveBeenCalledWith('d1');
	});

	it('listNotes maps nodes through nodeToNote correctly', async () => {
		const node = makeFactNode({
			id: 'mapped-1',
			title: 'Mapped Note',
			content: '## Subheading\n\nParagraph',
			tags: ['research', 'ml'],
			metadata: { pinned: true }
		});
		listNodesMock.mockResolvedValue([node]);

		const { listNotes } = await import('./notes');
		const notes = await listNotes();

		expect(notes[0].id).toBe('mapped-1');
		expect(notes[0].title).toBe('Mapped Note');
		expect(notes[0].markdown).toBe('## Subheading\n\nParagraph');
		expect(notes[0].tags).toEqual(['research', 'ml']);
		expect(notes[0].pinned).toBe(true);
		expect(notes[0].backlinks).toEqual([]);
	});
});
