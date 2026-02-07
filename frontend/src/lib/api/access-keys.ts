import { fetchJson } from './client';

export interface AccessKey {
	id: string;
	name: string;
	description?: string | null;
	permission_template_id?: string | null;
	created_at: string;
	last_used_at?: string | null;
	expires_at?: string | null;
}

export interface CreateAccessKeyPayload {
	name: string;
	description?: string;
	permission_template_id?: string;
	expires_at?: string;
}

export interface CreateAccessKeyResponse {
	key_id: string;
	raw_key: string;
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
	await fetchJson<{ deleted: boolean }>(`/api/v1/access-keys/${id}`, { method: 'DELETE' });
}
