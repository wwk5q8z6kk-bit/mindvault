import type { Note } from '$lib/api/notes';
import type { KnowledgeNode, NodeKind } from '$lib/api/types';

export interface WorkbenchNote extends Note {
	kind: NodeKind;
}

export const WORKBENCH_NOTE_KINDS: readonly NodeKind[] = [
	'fact',
	'decision',
	'procedure',
	'observation',
	'preference',
	'concept'
];

export function toWorkbenchNote(node: KnowledgeNode): WorkbenchNote | null {
	if (!WORKBENCH_NOTE_KINDS.includes(node.kind)) return null;
	if (node.tags.some((tag) => tag.startsWith('day:'))) return null;

	return {
		id: node.id,
		title: node.title,
		markdown: node.content ?? '',
		namespace: node.namespace,
		tags: node.tags,
		backlinks: [],
		pinned: Boolean(node.metadata?.pinned),
		created_at: node.temporal.created_at,
		updated_at: node.temporal.updated_at,
		metadata: node.metadata,
		kind: node.kind
	};
}

export function noteSelectionPath(currentUrl: URL, noteId: string | null): string {
	const next = new URL(currentUrl);
	next.searchParams.delete('view');
	next.searchParams.delete('node');
	if (noteId) next.searchParams.set('note', noteId);
	else next.searchParams.delete('note');
	return `${next.pathname}${next.search}${next.hash}`;
}
