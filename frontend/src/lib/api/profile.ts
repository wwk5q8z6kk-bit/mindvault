import { fetchJson } from './client';

export interface OwnerProfile {
	display_name: string;
	avatar_url: string | null;
	bio: string | null;
	email: string | null;
	preferred_namespace: string;
	default_node_kind: string;
	preferred_llm_provider: string | null;
	timezone: string;
	signature_name: string | null;
	signature_public_key: string | null;
	metadata: Record<string, unknown>;
	created_at: string;
	updated_at: string;
}

export interface UpdateProfilePayload {
	display_name?: string;
	avatar_url?: string;
	bio?: string;
	email?: string;
	preferred_namespace?: string;
	default_node_kind?: string;
	preferred_llm_provider?: string;
	timezone?: string;
	signature_name?: string;
	signature_public_key?: string;
	metadata?: Record<string, unknown>;
}

export async function getProfile(): Promise<OwnerProfile> {
	return fetchJson<OwnerProfile>('/api/v1/profile');
}

export async function updateProfile(data: UpdateProfilePayload): Promise<OwnerProfile> {
	return fetchJson<OwnerProfile>('/api/v1/profile', {
		method: 'PUT',
		body: JSON.stringify(data)
	});
}
