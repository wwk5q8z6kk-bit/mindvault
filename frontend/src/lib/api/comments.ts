import { fetchJson } from './client';

export interface NodeComment {
	id: string;
	node_id: string;
	author?: string | null;
	body: string;
	created_at: string;
	updated_at: string;
	resolved_at?: string | null;
}

export async function listNodeComments(
	nodeId: string,
	includeResolved = false
): Promise<NodeComment[]> {
	const params = new URLSearchParams();
	if (includeResolved) params.set('include_resolved', 'true');
	const suffix = params.toString();
	return fetchJson<NodeComment[]>(
		`/api/v1/nodes/${nodeId}/comments${suffix ? `?${suffix}` : ''}`
	);
}

export async function createNodeComment(payload: {
	node_id: string;
	body: string;
	author?: string;
}): Promise<NodeComment> {
	return fetchJson<NodeComment>(`/api/v1/nodes/${payload.node_id}/comments`, {
		method: 'POST',
		body: JSON.stringify({ body: payload.body, author: payload.author })
	});
}

export async function resolveNodeComment(nodeId: string, commentId: string): Promise<NodeComment> {
	return fetchJson<NodeComment>(
		`/api/v1/nodes/${nodeId}/comments/${commentId}/resolve`,
		{ method: 'PUT' }
	);
}

export async function deleteNodeComment(nodeId: string, commentId: string): Promise<NodeComment> {
	return fetchJson<NodeComment>(`/api/v1/nodes/${nodeId}/comments/${commentId}`, {
		method: 'DELETE'
	});
}
