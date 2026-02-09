import { fetchJson } from './client';

export type ConflictType = 'contradiction' | 'supersession' | 'ambiguity';

export interface ConflictAlert {
	id: string;
	node_a: string;
	node_b: string;
	conflict_type: ConflictType;
	score: number;
	explanation: string;
	resolved: boolean;
	created_at: string;
}

/** List conflict alerts with optional resolved filter. */
export async function listConflicts(
	resolved?: boolean,
	limit?: number,
	offset?: number
): Promise<ConflictAlert[]> {
	const params = new URLSearchParams();
	if (resolved != null) params.set('resolved', String(resolved));
	if (limit != null) params.set('limit', String(limit));
	if (offset != null) params.set('offset', String(offset));
	const qs = params.toString();
	return fetchJson<ConflictAlert[]>(`/api/v1/conflicts${qs ? `?${qs}` : ''}`);
}

/** Get a single conflict alert. */
export async function getConflict(id: string): Promise<ConflictAlert> {
	return fetchJson<ConflictAlert>(`/api/v1/conflicts/${id}`);
}

/** Resolve (dismiss) a conflict alert. */
export async function resolveConflict(id: string): Promise<{ resolved: boolean }> {
	return fetchJson<{ resolved: boolean }>(`/api/v1/conflicts/${id}/resolve`, {
		method: 'POST'
	});
}
