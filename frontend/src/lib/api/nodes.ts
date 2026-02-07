import { fetchJson } from './client';
import type { KnowledgeNode, NodeKind, StoreNodeRequest } from './types';

export type ListNodesParams = {
	kind?: NodeKind;
	namespace?: string;
	limit?: number;
	offset?: number;
};

export async function listNodes(params: ListNodesParams = {}): Promise<KnowledgeNode[]> {
	const searchParams = new URLSearchParams();
	if (params.kind) searchParams.set('kind', params.kind);
	if (params.namespace) searchParams.set('namespace', params.namespace);
	if (params.limit) searchParams.set('limit', String(params.limit));
	if (params.offset) searchParams.set('offset', String(params.offset));
	const suffix = searchParams.toString();
	return await fetchJson<KnowledgeNode[]>(`/api/v1/nodes${suffix ? `?${suffix}` : ''}`);
}

export async function createNode(payload: StoreNodeRequest): Promise<KnowledgeNode> {
	return await fetchJson<KnowledgeNode>('/api/v1/nodes', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function updateNode(
	id: string,
	payload: Partial<StoreNodeRequest>
): Promise<KnowledgeNode> {
	return await fetchJson<KnowledgeNode>(`/api/v1/nodes/${id}`, {
		method: 'PUT',
		body: JSON.stringify(payload)
	});
}

export async function deleteNode(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/nodes/${id}`, { method: 'DELETE' });
}
