import { fetchJson } from './client';

export interface BlockedSender {
	id: string;
	sender_type: string;
	sender_pattern: string;
	reason?: string | null;
	blocked_at: string;
	expires_at?: string | null;
}

export interface AutoApproveRule {
	id: string;
	name: string;
	sender_pattern?: string | null;
	action_types: string[];
	min_confidence: number;
	enabled: boolean;
	created_at: string;
	updated_at?: string | null;
}

export interface AddBlockedSenderRequest {
	sender_type: 'agent' | 'mcp' | 'webhook' | 'watcher' | 'relay';
	sender_pattern: string;
	reason?: string;
	expires_at?: string;
}

export interface AddAutoApproveRuleRequest {
	name: string;
	min_confidence: number;
	sender_pattern?: string;
	action_types?: string[];
}

export interface UpdateAutoApproveRuleRequest {
	name?: string;
	sender_pattern?: string | null;
	action_types?: string[];
	min_confidence?: number;
	enabled?: boolean;
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

export async function listAutoApproveRules(): Promise<AutoApproveRule[]> {
	return fetchJson<AutoApproveRule[]>('/api/v1/exchange/auto-approve-rules');
}

export async function addAutoApproveRule(
	payload: AddAutoApproveRuleRequest
): Promise<AutoApproveRule> {
	return fetchJson<AutoApproveRule>('/api/v1/exchange/auto-approve-rules', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function updateAutoApproveRule(
	id: string,
	payload: UpdateAutoApproveRuleRequest
): Promise<AutoApproveRule> {
	return fetchJson<AutoApproveRule>(`/api/v1/exchange/auto-approve-rules/${encodeURIComponent(id)}`, {
		method: 'PUT',
		body: JSON.stringify(payload)
	});
}

export async function removeAutoApproveRule(id: string): Promise<{ id: string; removed: boolean }> {
	return fetchJson(`/api/v1/exchange/auto-approve-rules/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}
