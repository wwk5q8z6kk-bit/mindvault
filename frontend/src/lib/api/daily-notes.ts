import { fetchJson } from './client';
import type { KnowledgeNode } from './types';
import type { Note } from './notes';
import { nodeToNote } from './mappers';

export interface DailyNoteEnsureResponse {
	node: KnowledgeNode;
	created: boolean;
}

/** List recent daily notes. */
export async function listDailyNotes(limit = 30): Promise<Note[]> {
	const nodes = await fetchJson<KnowledgeNode[]>(
		`/api/v1/daily-notes?limit=${limit}`
	);
	return nodes.map(nodeToNote);
}

/** Ensure a daily note exists for the given date (defaults to today). */
export async function ensureDailyNote(date?: string): Promise<{ note: Note; created: boolean }> {
	const body: Record<string, string> = {};
	if (date) body.date = date;
	const res = await fetchJson<DailyNoteEnsureResponse>('/api/v1/daily-notes/ensure', {
		method: 'POST',
		body: JSON.stringify(body)
	});
	return { note: nodeToNote(res.node), created: res.created };
}
