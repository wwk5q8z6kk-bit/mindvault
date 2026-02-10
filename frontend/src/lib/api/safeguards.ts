import { fetchJson } from './client';

export interface BlockedSender {
	id: string;
	sender_type: string;
	sender_pattern: string;
	reason?: string | null;
	blocked_at: string;
	expires_at?: string | null;
}

export interface AddBlockedSenderRequest {
	sender_type: 'agent' | 'mcp' | 'webhook' | 'watcher' | 'relay';
	sender_pattern: string;
	reason?: string;
	expires_at?: string;
}

export async function listBlockedSenders(): Promise<BlockedSender[]> {
	return fetchJson<BlockedSender[]>('/api/v1/exchange/blocked-senders');
}

export async function addBlockedSender(
	payload: AddBlockedSenderRequest
): Promise<BlockedSender> {
	return fetchJson<BlockedSender>('/api/v1/exchange/blocked-senders', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function removeBlockedSender(id: string): Promise<{ id: string; removed: boolean }> {
	return fetchJson(`/api/v1/exchange/blocked-senders/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}
