import { fetchJson } from './client';
import { createNode, deleteNode, listNodes } from './nodes';
import type { KnowledgeNode, NodeKind, StoreNodeRequest } from './types';
import { nodeToNote, notePatchToNode } from './mappers';

export interface Note {
	id: string;
	markdown: string;
	title: string | null;
	namespace?: string | null;
	tags: string[];
	backlinks: string[];
	pinned: boolean;
	created_at: string;
	updated_at: string;
	metadata: Record<string, unknown>;
}

const NOTE_KIND: NodeKind = 'fact';

function mapNote(node: KnowledgeNode): Note {
	return nodeToNote(node);
}

export async function listNotes(limit = 50, namespace?: string | null): Promise<Note[]> {
	const nodes = await listNodes({ kind: NOTE_KIND, limit, namespace: namespace ?? undefined });
	return nodes
		.filter((node) => !node.tags.some((tag) => tag.startsWith('day:')))
		.map(mapNote);
}

export async function createNote(
	payload: { title: string; markdown: string; namespace?: string; tags?: string[]; importance?: number } | string,
	title?: string
): Promise<Note> {
	const request: StoreNodeRequest =
		typeof payload === 'string'
			? {
					kind: NOTE_KIND,
					title: title ?? 'Untitled',
					content: payload
			  }
			: {
					kind: NOTE_KIND,
					title: payload.title,
					content: payload.markdown,
					namespace: payload.namespace,
					tags: payload.tags,
					importance: payload.importance
			  };
	const node = await createNode(request);
	return mapNote(node);
}

export async function updateNote(
	id: string,
	payload: {
		title?: string;
		markdown?: string;
		pinned?: boolean;
		namespace?: string;
		tags?: string[];
		importance?: number;
	}
): Promise<Note> {
	const existing = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${id}`);
	const merged = notePatchToNode(existing, {
		title: payload.title,
		markdown: payload.markdown
	});
	if (payload.pinned !== undefined) {
		merged.metadata = { ...merged.metadata, pinned: payload.pinned };
	}
	if (payload.namespace) merged.namespace = payload.namespace;
	if (payload.tags) merged.tags = payload.tags;
	if (payload.importance !== undefined) merged.importance = payload.importance;
	const node = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${id}`, {
		method: 'PUT',
		body: JSON.stringify(merged)
	});
	return mapNote(node);
}

export async function deleteNote(id: string): Promise<void> {
	await deleteNode(id);
}
