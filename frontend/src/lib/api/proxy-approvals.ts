import { fetchJson } from './client';

export interface ProxyApproval {
	id: string;
	consumer: string;
	secret_key: string;
	intent: string;
	request_summary: string;
	state: 'pending' | 'approved' | 'denied' | 'expired' | string;
	created_at: string;
	expires_at: string;
	decided_at?: string | null;
	decided_by?: string | null;
	deny_reason?: string | null;
	scopes: string[];
}

export async function listProxyApprovals(consumer?: string): Promise<ProxyApproval[]> {
	const params = new URLSearchParams();
	if (consumer) params.set('consumer', consumer);
	const query = params.toString();
	const path = query ? `/api/v1/proxy/approvals?${query}` : '/api/v1/proxy/approvals';
	return await fetchJson<ProxyApproval[]>(path);
}

export async function decideProxyApproval(
	id: string,
	approved: boolean,
	denyReason?: string
): Promise<ProxyApproval> {
	return await fetchJson<ProxyApproval>(`/api/v1/proxy/approvals/${encodeURIComponent(id)}`, {
		method: 'POST',
		body: JSON.stringify({
			approved,
			deny_reason: denyReason
		})
	});
}
