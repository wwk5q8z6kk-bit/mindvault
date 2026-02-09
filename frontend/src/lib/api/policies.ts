import { fetchJson } from './client';

export interface AccessPolicy {
	id: string;
	secret_key: string;
	consumer: string;
	allowed: boolean;
	scopes: string[];
	max_ttl_seconds: number | null;
	expires_at: string | null;
	require_approval: boolean;
	created_at: string;
	updated_at: string;
}

export interface SetPolicyRequest {
	secret_key: string;
	consumer: string;
	allowed: boolean;
	scopes?: string[];
	max_ttl_seconds?: number;
	expires_at?: string;
	require_approval?: boolean;
}

export interface PolicyMatrix {
	secrets: string[];
	consumers: string[];
	matrix: Record<string, Record<string, boolean>>;
}

/** Create or update an access policy. */
export async function setPolicy(req: SetPolicyRequest): Promise<AccessPolicy> {
	return fetchJson<AccessPolicy>('/api/v1/policies', {
		method: 'POST',
		body: JSON.stringify(req)
	});
}

/** List access policies, optionally filtered by secret_key and/or consumer. */
export async function listPolicies(
	secretKey?: string,
	consumer?: string
): Promise<AccessPolicy[]> {
	const params = new URLSearchParams();
	if (secretKey) params.set('secret_key', secretKey);
	if (consumer) params.set('consumer', consumer);
	const qs = params.toString();
	const path = qs ? `/api/v1/policies?${qs}` : '/api/v1/policies';
	return fetchJson<AccessPolicy[]>(path);
}

/** Get the full policy matrix (secrets x consumers grid). */
export async function getPolicyMatrix(): Promise<PolicyMatrix> {
	return fetchJson<PolicyMatrix>('/api/v1/policies/matrix');
}

/** Get the list of secret keys accessible by the current consumer. */
export async function getMyAccess(): Promise<string[]> {
	return fetchJson<string[]>('/api/v1/policies/my-access');
}

/** Delete an access policy by ID. */
export async function deletePolicy(id: string): Promise<void> {
	return fetchJson<void>(`/api/v1/policies/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}
