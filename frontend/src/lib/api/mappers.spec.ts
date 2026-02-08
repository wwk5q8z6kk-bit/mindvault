import { describe, expect, it } from 'vitest';
import {
	nodeToTask,
	taskToNodePayload,
	taskPatchToNode,
	nodeToNote,
	noteToNodePayload,
	notePatchToNode
} from './mappers';
import type { KnowledgeNode } from './types';
import {
	TASK_DUE_AT,
	TASK_COMPLETED,
	TASK_COMPLETED_AT,
	TASK_STATUS,
	TASK_RECURRENCE,
	TASK_ESTIMATE_MIN,
	TASK_ASSIGNEE,
	TASK_DEPENDENCIES,
	NOTE_PINNED
} from './types';

function makeNode(overrides: Partial<KnowledgeNode> = {}): KnowledgeNode {
	return {
		id: 'test-id-1',
		kind: 'task',
		title: 'Test Task',
		content: 'Task description',
		source: null,
		namespace: 'default',
		tags: ['work', 'urgent'],
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

describe('nodeToTask', () => {
	it('maps basic node fields to task', () => {
		const node = makeNode();
		const task = nodeToTask(node);

		expect(task.id).toBe('test-id-1');
		expect(task.title).toBe('Test Task');
		expect(task.description).toBe('Task description');
		expect(task.labels).toEqual(['work', 'urgent']);
		expect(task.created_at).toBe('2026-01-15T10:00:00Z');
		expect(task.updated_at).toBe('2026-01-16T12:00:00Z');
	});

	it('converts importance 0..1 to priority 1..5', () => {
		expect(nodeToTask(makeNode({ importance: 0 })).priority).toBe(1);
		expect(nodeToTask(makeNode({ importance: 0.25 })).priority).toBe(2);
		expect(nodeToTask(makeNode({ importance: 0.5 })).priority).toBe(3);
		expect(nodeToTask(makeNode({ importance: 0.75 })).priority).toBe(4);
		expect(nodeToTask(makeNode({ importance: 1.0 })).priority).toBe(5);
	});

	it('defaults status to inbox when no metadata', () => {
		const task = nodeToTask(makeNode());
		expect(task.status).toBe('inbox');
	});

	it('reads status from metadata.task_status', () => {
		const node = makeNode({ metadata: { [TASK_STATUS]: 'in_progress' } });
		expect(nodeToTask(node).status).toBe('in_progress');
	});

	it('overrides status to done when task_completed is true', () => {
		const node = makeNode({
			metadata: { [TASK_STATUS]: 'in_progress', [TASK_COMPLETED]: true }
		});
		expect(nodeToTask(node).status).toBe('done');
	});

	it('ignores invalid status values and defaults to inbox', () => {
		const node = makeNode({ metadata: { [TASK_STATUS]: 'banana' } });
		expect(nodeToTask(node).status).toBe('inbox');
	});

	it('extracts due_at from metadata', () => {
		const node = makeNode({ metadata: { [TASK_DUE_AT]: '2026-02-10T09:00:00Z' } });
		expect(nodeToTask(node).due_at).toBe('2026-02-10T09:00:00Z');
	});

	it('extracts completed_at from metadata', () => {
		const node = makeNode({
			metadata: {
				[TASK_COMPLETED]: true,
				[TASK_COMPLETED_AT]: '2026-02-08T15:00:00Z'
			}
		});
		expect(nodeToTask(node).completed_at).toBe('2026-02-08T15:00:00Z');
	});

	it('extracts estimate_min, assignee, dependencies, recurrence', () => {
		const node = makeNode({
			metadata: {
				[TASK_ESTIMATE_MIN]: 45,
				[TASK_ASSIGNEE]: 'alice',
				[TASK_DEPENDENCIES]: ['dep-1', 'dep-2'],
				[TASK_RECURRENCE]: 'weekly'
			}
		});
		const task = nodeToTask(node);
		expect(task.estimate_min).toBe(45);
		expect(task.assignee).toBe('alice');
		expect(task.dependencies).toEqual(['dep-1', 'dep-2']);
		expect(task.recurrence).toBe('weekly');
	});

	it('returns null for missing optional fields', () => {
		const task = nodeToTask(makeNode());
		expect(task.due_at).toBeNull();
		expect(task.completed_at).toBeNull();
		expect(task.estimate_min).toBeNull();
		expect(task.assignee).toBeNull();
		expect(task.recurrence).toBeNull();
	});

	it('returns empty array for missing dependencies', () => {
		expect(nodeToTask(makeNode()).dependencies).toEqual([]);
	});

	it('passes through full metadata object', () => {
		const meta = { custom_field: 'value', [TASK_STATUS]: 'planned' };
		const task = nodeToTask(makeNode({ metadata: meta }));
		expect(task.metadata).toEqual(meta);
	});
});

describe('taskToNodePayload', () => {
	it('creates a node payload with kind=task', () => {
		const payload = taskToNodePayload({ title: 'New Task' });
		expect(payload.kind).toBe('task');
		expect(payload.title).toBe('New Task');
	});

	it('maps description to content', () => {
		const payload = taskToNodePayload({ title: 'T', description: 'Details here' });
		expect(payload.content).toBe('Details here');
	});

	it('maps labels to tags', () => {
		const payload = taskToNodePayload({ title: 'T', labels: ['bug', 'frontend'] });
		expect(payload.tags).toEqual(['bug', 'frontend']);
	});

	it('converts priority 1..5 to importance 0..1', () => {
		const p1 = taskToNodePayload({ title: 'T', priority: 1 });
		const p3 = taskToNodePayload({ title: 'T', priority: 3 });
		const p5 = taskToNodePayload({ title: 'T', priority: 5 });
		expect(p1.importance).toBe(0);
		expect(p3.importance).toBe(0.5);
		expect(p5.importance).toBe(1);
	});

	it('defaults importance to 0.5 when no priority', () => {
		const payload = taskToNodePayload({ title: 'T' });
		expect(payload.importance).toBe(0.5);
	});

	it('packs status into metadata', () => {
		const payload = taskToNodePayload({ title: 'T', status: 'in_progress' });
		expect(payload.metadata?.[TASK_STATUS]).toBe('in_progress');
	});

	it('sets task_completed when status is done', () => {
		const payload = taskToNodePayload({ title: 'T', status: 'done' });
		expect(payload.metadata?.[TASK_COMPLETED]).toBe(true);
	});

	it('packs due_at, recurrence, estimate, assignee, dependencies', () => {
		const payload = taskToNodePayload({
			title: 'T',
			due_at: '2026-03-01',
			recurrence: 'daily',
			estimate_min: 30,
			assignee: 'bob',
			dependencies: ['task-a']
		});
		expect(payload.metadata?.[TASK_DUE_AT]).toBe('2026-03-01');
		expect(payload.metadata?.[TASK_RECURRENCE]).toBe('daily');
		expect(payload.metadata?.[TASK_ESTIMATE_MIN]).toBe(30);
		expect(payload.metadata?.[TASK_ASSIGNEE]).toBe('bob');
		expect(payload.metadata?.[TASK_DEPENDENCIES]).toEqual(['task-a']);
	});

	it('preserves extra metadata from payload', () => {
		const payload = taskToNodePayload({
			title: 'T',
			metadata: { custom: 'data' }
		});
		expect(payload.metadata?.custom).toBe('data');
	});

	it('maps null description to undefined content', () => {
		const payload = taskToNodePayload({ title: 'T', description: null });
		expect(payload.content).toBeUndefined();
	});
});

describe('taskPatchToNode', () => {
	const base = makeNode({
		metadata: { [TASK_STATUS]: 'inbox' }
	});

	it('updates title', () => {
		const result = taskPatchToNode(base, { title: 'New Title' });
		expect(result.title).toBe('New Title');
	});

	it('updates description to content', () => {
		const result = taskPatchToNode(base, { description: 'New desc' });
		expect(result.content).toBe('New desc');
	});

	it('updates labels to tags', () => {
		const result = taskPatchToNode(base, { labels: ['new-label'] });
		expect(result.tags).toEqual(['new-label']);
	});

	it('updates priority to importance', () => {
		const result = taskPatchToNode(base, { priority: 5 });
		expect(result.importance).toBe(1);
	});

	it('sets completed metadata when status changes to done', () => {
		const result = taskPatchToNode(base, { status: 'done' });
		expect(result.metadata[TASK_STATUS]).toBe('done');
		expect(result.metadata[TASK_COMPLETED]).toBe(true);
		expect(result.metadata[TASK_COMPLETED_AT]).toBeDefined();
	});

	it('clears completed metadata when status changes from done', () => {
		const doneNode = makeNode({
			metadata: {
				[TASK_STATUS]: 'done',
				[TASK_COMPLETED]: true,
				[TASK_COMPLETED_AT]: '2026-01-01T00:00:00Z'
			}
		});
		const result = taskPatchToNode(doneNode, { status: 'planned' });
		expect(result.metadata[TASK_STATUS]).toBe('planned');
		expect(result.metadata[TASK_COMPLETED]).toBe(false);
		expect(result.metadata[TASK_COMPLETED_AT]).toBeUndefined();
	});

	it('preserves existing fields when not in patch', () => {
		const result = taskPatchToNode(base, { title: 'Changed' });
		expect(result.content).toBe('Task description');
		expect(result.tags).toEqual(['work', 'urgent']);
		expect(result.importance).toBe(0.5);
	});

	it('does not mutate the original node', () => {
		const original = makeNode({ metadata: { foo: 'bar' } });
		const originalMeta = { ...original.metadata };
		taskPatchToNode(original, { status: 'done' });
		expect(original.metadata).toEqual(originalMeta);
	});

	it('merges extra metadata from patch', () => {
		const result = taskPatchToNode(base, { metadata: { custom: 'value' } });
		expect(result.metadata.custom).toBe('value');
	});
});

describe('nodeToNote', () => {
	it('maps basic node fields to note', () => {
		const node = makeNode({
			kind: 'fact',
			title: 'My Note',
			content: '# Hello\n\nWorld',
			tags: ['journal']
		});
		const note = nodeToNote(node);

		expect(note.id).toBe('test-id-1');
		expect(note.title).toBe('My Note');
		expect(note.markdown).toBe('# Hello\n\nWorld');
		expect(note.tags).toEqual(['journal']);
		expect(note.backlinks).toEqual([]);
		expect(note.created_at).toBe('2026-01-15T10:00:00Z');
		expect(note.updated_at).toBe('2026-01-16T12:00:00Z');
	});

	it('reads pinned from metadata', () => {
		const pinned = nodeToNote(makeNode({ metadata: { [NOTE_PINNED]: true } }));
		const unpinned = nodeToNote(makeNode({ metadata: {} }));
		expect(pinned.pinned).toBe(true);
		expect(unpinned.pinned).toBe(false);
	});

	it('defaults markdown to empty string when content is null', () => {
		const note = nodeToNote(makeNode({ content: null }));
		expect(note.markdown).toBe('');
	});

	it('returns null title for empty string title', () => {
		const note = nodeToNote(makeNode({ title: '' }));
		expect(note.title).toBeNull();
	});
});

describe('noteToNodePayload', () => {
	it('creates payload with kind=fact', () => {
		const payload = noteToNodePayload('Some markdown', 'My Title');
		expect(payload.kind).toBe('fact');
		expect(payload.title).toBe('My Title');
		expect(payload.content).toBe('Some markdown');
	});

	it('extracts title from first heading when no title provided', () => {
		const payload = noteToNodePayload('# Auto Title\n\nBody here');
		expect(payload.title).toBe('Auto Title');
	});

	it('uses first line as title when no heading found', () => {
		const payload = noteToNodePayload('Just a plain line\n\nMore text');
		expect(payload.title).toBe('Just a plain line');
	});

	it('returns Untitled for empty markdown', () => {
		const payload = noteToNodePayload('');
		expect(payload.title).toBe('Untitled');
	});

	it('truncates long first-line titles to 80 chars', () => {
		const longLine = 'A'.repeat(100);
		const payload = noteToNodePayload(longLine);
		expect(payload.title.length).toBe(80);
	});
});

describe('notePatchToNode', () => {
	const base = makeNode({ kind: 'fact', content: 'Original content' });

	it('updates content from markdown', () => {
		const result = notePatchToNode(base, { markdown: 'Updated content' });
		expect(result.content).toBe('Updated content');
	});

	it('updates title', () => {
		const result = notePatchToNode(base, { title: 'New Title' });
		expect(result.title).toBe('New Title');
	});

	it('sets pinned in metadata', () => {
		const result = notePatchToNode(base, { pinned: true });
		expect(result.metadata[NOTE_PINNED]).toBe(true);
	});

	it('preserves existing fields when not in patch', () => {
		const result = notePatchToNode(base, { title: 'Changed' });
		expect(result.content).toBe('Original content');
	});

	it('does not mutate the original node', () => {
		const original = makeNode({ metadata: { foo: 'bar' } });
		const originalMeta = { ...original.metadata };
		notePatchToNode(original, { pinned: true });
		expect(original.metadata).toEqual(originalMeta);
	});
});
