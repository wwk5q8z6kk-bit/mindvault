import { fetchJson } from './client';

export interface OAuthClient {
	client_id: string;
	name: string;
	template_id: string;
	created_at: string;
	updated_at: string;
	last_used_at?: string | null;
	expires_at?: string | null;
	revoked_at?: string | null;
	token_ttl_seconds: number;
	description?: string | null;
}

export interface CreateOAuthClientPayload {
	name: string;
	template_id: string;
	description?: string;
	token_ttl_seconds?: number;
	expires_at?: string;
}

export interface CreateOAuthClientResponse {
	client: OAuthClient;
	client_secret: string;
}

export async function listOAuthClients(): Promise<OAuthClient[]> {
	return fetchJson<OAuthClient[]>('/api/v1/oauth/clients');
}

export async function createOAuthClient(
	payload: CreateOAuthClientPayload
): Promise<CreateOAuthClientResponse> {
	return fetchJson<CreateOAuthClientResponse>('/api/v1/oauth/clients', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function revokeOAuthClient(id: string): Promise<void> {
	await fetchJson(`/api/v1/oauth/clients/${id}`, { method: 'DELETE' });
}
