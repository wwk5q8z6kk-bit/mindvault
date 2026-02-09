import { fetchJson } from './client';

export interface ConsumerProfile {
	id: string;
	name: string;
	description?: string;
	created_at: string;
	last_used_at?: string;
	revoked_at?: string;
}

export interface CreateConsumerResponse {
	id: string;
	name: string;
	token: string;
}

/** Create a new consumer profile. Returns the profile and a one-time bearer token. */
export async function createConsumer(
	name: string,
	description?: string
): Promise<CreateConsumerResponse> {
	return fetchJson<CreateConsumerResponse>('/api/v1/consumers', {
		method: 'POST',
		body: JSON.stringify({ name, description })
	});
}

/** List all consumer profiles (no token hashes). */
export async function listConsumers(): Promise<ConsumerProfile[]> {
	return fetchJson<ConsumerProfile[]>('/api/v1/consumers');
}

/** Get a single consumer profile by ID. */
export async function getConsumer(id: string): Promise<ConsumerProfile> {
	return fetchJson<ConsumerProfile>(`/api/v1/consumers/${encodeURIComponent(id)}`);
}

/** Revoke a consumer profile by ID. */
export async function revokeConsumer(id: string): Promise<void> {
	return fetchJson<void>(`/api/v1/consumers/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}

/** Get the consumer profile for the current bearer token. */
export async function whoami(): Promise<ConsumerProfile> {
	return fetchJson<ConsumerProfile>('/api/v1/consumers/whoami');
}
