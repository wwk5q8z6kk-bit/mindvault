import { fetchJson } from './client';

export interface FederationPeer {
	id: string;
	vault_id: string;
	display_name: string;
	endpoint: string;
	public_key: string | null;
	allowed_namespaces: string[];
	max_results: number;
	enabled: boolean;
	last_seen: string | null;
	created_at: string;
}

export interface FederatedResult {
	source_vault: string;
	source_peer_name: string;
	node: Record<string, unknown>;
	relevance_score: number;
}

export async function listPeers(): Promise<{ peers: FederationPeer[]; count: number }> {
	return fetchJson('/api/v1/federation/peers');
}

export async function addPeer(data: {
	vault_id: string;
	display_name: string;
	endpoint: string;
	public_key?: string;
	allowed_namespaces?: string[];
	max_results?: number;
}): Promise<{ id: string }> {
	return fetchJson('/api/v1/federation/peers', {
		method: 'POST',
		body: JSON.stringify(data)
	});
}

export async function removePeer(id: string): Promise<void> {
	await fetchJson(`/api/v1/federation/peers/${id}`, { method: 'DELETE' });
}

export async function peerHealth(id: string): Promise<{ peer_id: string; healthy: boolean }> {
	return fetchJson(`/api/v1/federation/peers/${id}/health`);
}

export async function federatedQuery(
	query: string,
	limit = 50
): Promise<{ results: FederatedResult[]; peer_count: number }> {
	return fetchJson('/api/v1/federation/query', {
		method: 'POST',
		body: JSON.stringify({ query, limit })
	});
}
