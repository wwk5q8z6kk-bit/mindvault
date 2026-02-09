import { fetchJson } from './client';

export interface ProxyAuditEntry {
	id: string;
	consumer: string;
	secret_ref: string;
	action: string;
	target: string;
	intent: string;
	timestamp: string;
	success: boolean | null;
	sanitized: boolean;
	error: string | null;
	request_summary: string;
	response_status: number | null;
}

/** List proxy audit log entries with optional consumer filter and pagination. */
export async function listProxyAudit(
	consumer?: string,
	limit?: number,
	offset?: number
): Promise<ProxyAuditEntry[]> {
	const params = new URLSearchParams();
	if (consumer) params.set('consumer', consumer);
	if (limit !== undefined) params.set('limit', String(limit));
	if (offset !== undefined) params.set('offset', String(offset));
	const qs = params.toString();
	const path = qs ? `/api/v1/proxy/audit?${qs}` : '/api/v1/proxy/audit';
	return fetchJson<ProxyAuditEntry[]>(path);
}
