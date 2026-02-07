import { fetchJson } from './client';

export interface BackendStatus {
	name: string;
	available: boolean;
	keys: string[];
}

export interface SecretStatusResponse {
	backends: BackendStatus[];
}

export interface SetSecretRequest {
	key: string;
	value: string;
}

export interface SetSecretResponse {
	key: string;
	stored_in: string;
}

export interface DeleteSecretResponse {
	key: string;
	deleted_from: string[];
}

/** Fetch the status of all credential backends and which keys are stored. */
export async function getSecretStatus(): Promise<SecretStatusResponse> {
	return fetchJson<SecretStatusResponse>('/api/v1/secrets/status');
}

/** Store a secret in the highest-priority writable backend. */
export async function setSecret(key: string, value: string): Promise<SetSecretResponse> {
	return fetchJson<SetSecretResponse>('/api/v1/secrets', {
		method: 'POST',
		body: JSON.stringify({ key, value })
	});
}

/** Delete a secret from all backends. */
export async function deleteSecret(key: string): Promise<DeleteSecretResponse> {
	return fetchJson<DeleteSecretResponse>(`/api/v1/secrets/${encodeURIComponent(key)}`, {
		method: 'DELETE'
	});
}

/** Well-known secret keys that MindVault uses. */
export const KNOWN_SECRETS = [
	{
		key: 'OPENAI_API_KEY',
		label: 'OpenAI API Key',
		description: 'Used for embeddings (vector search) and AI enrichment',
		required: true
	},
	{
		key: 'MINDVAULT_EMBEDDING_API_KEY',
		label: 'Embedding API Key',
		description: 'Override for OpenAI-compatible embedding providers',
		required: false
	},
	{
		key: 'MINDVAULT_ENCRYPTION_KEY',
		label: 'Encryption Key',
		description: 'Master key for at-rest encryption',
		required: false
	}
] as const;
