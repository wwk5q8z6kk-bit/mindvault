import { fetchJson } from './client';

// --- Types ---

export interface AdapterConfig {
	id: string;
	adapter_type: string;
	name: string;
	enabled: boolean;
	created_at: string;
	updated_at: string | null;
}

export interface AdapterStatus {
	adapter_type: string;
	name: string;
	connected: boolean;
	last_send: string | null;
	last_receive: string | null;
	error: string | null;
}

export interface RegisterAdapterRequest {
	adapter_type: string;
	name: string;
	enabled?: boolean;
	settings: Record<string, string>;
}

export interface SendMessageRequest {
	channel: string;
	content: string;
	thread_id?: string;
	metadata?: Record<string, string>;
}

// --- API functions ---

export async function listAdapters(): Promise<AdapterConfig[]> {
	return fetchJson<AdapterConfig[]>('/api/v1/adapters');
}

export async function registerAdapter(req: RegisterAdapterRequest): Promise<AdapterConfig> {
	return fetchJson<AdapterConfig>('/api/v1/adapters', {
		method: 'POST',
		body: JSON.stringify(req)
	});
}

export async function getAdapterStatus(id: string): Promise<AdapterStatus> {
	return fetchJson<AdapterStatus>(`/api/v1/adapters/${id}`);
}

export async function removeAdapter(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/adapters/${id}`, { method: 'DELETE' });
}

export async function sendAdapterMessage(id: string, req: SendMessageRequest): Promise<void> {
	await fetchJson<void>(`/api/v1/adapters/${id}/send`, {
		method: 'POST',
		body: JSON.stringify(req)
	});
}

export async function healthCheckAdapter(
	id: string
): Promise<{ adapter_id: string; healthy: boolean }> {
	return fetchJson<{ adapter_id: string; healthy: boolean }>(
		`/api/v1/adapters/${id}/health`,
		{ method: 'POST' }
	);
}

export async function listAdapterStatuses(): Promise<AdapterStatus[]> {
	return fetchJson<AdapterStatus[]>('/api/v1/adapters/statuses');
}
