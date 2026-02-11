import { fetchJson } from './client';

export interface PublicShareSummary {
	id: string;
	node_id: string;
	created_at: string;
	expires_at?: string | null;
	revoked_at?: string | null;
}

export interface CreatePublicSharePayload {
	node_id: string;
	expires_at?: string;
}

export interface CreatePublicShareResponse extends PublicShareSummary {
	token: string;
	url: string;
}

export async function createPublicShare(
	payload: CreatePublicSharePayload
): Promise<CreatePublicShareResponse> {
	return fetchJson<CreatePublicShareResponse>('/api/v1/shares', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export type ListPublicSharesParams = {
	node_id?: string;
	include_revoked?: boolean;
	namespace?: string;
};

export async function listPublicShares(
	params: ListPublicSharesParams = {}
): Promise<PublicShareSummary[]> {
	const searchParams = new URLSearchParams();
	if (params.node_id) searchParams.set('node_id', params.node_id);
	if (params.include_revoked !== undefined)
		searchParams.set('include_revoked', String(params.include_revoked));
	if (params.namespace) searchParams.set('namespace', params.namespace);
	const suffix = searchParams.toString();
	return fetchJson<PublicShareSummary[]>(`/api/v1/shares${suffix ? `?${suffix}` : ''}`);
}

export async function revokePublicShare(id: string): Promise<PublicShareSummary> {
	return fetchJson<PublicShareSummary>(`/api/v1/shares/${id}`, { method: 'DELETE' });
}
