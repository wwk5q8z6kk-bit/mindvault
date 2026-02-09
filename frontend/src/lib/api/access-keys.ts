import { fetchJson } from './client';

export interface AccessKey {
	id: string;
	name?: string | null;
	template_id: string;
	template_name?: string | null;
	created_at: string;
	last_used_at?: string | null;
	expires_at?: string | null;
	revoked_at?: string | null;
}

export interface CreateAccessKeyPayload {
	template_id: string;
	name?: string;
	expires_at?: string;
}

export interface CreateAccessKeyResponse {
	token: string;
	access_key: AccessKey;
}

export async function listAccessKeys(): Promise<AccessKey[]> {
	return fetchJson<AccessKey[]>('/api/v1/access-keys');
}

export async function createAccessKey(payload: CreateAccessKeyPayload): Promise<CreateAccessKeyResponse> {
	return fetchJson<CreateAccessKeyResponse>('/api/v1/access-keys', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function revokeAccessKey(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/access-keys/${id}`, { method: 'DELETE' });
}
