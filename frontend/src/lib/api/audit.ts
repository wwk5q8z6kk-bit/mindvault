import { fetchJson } from './client';

export interface AuditEntry {
	request_id: string;
	timestamp: string;
	subject?: string | null;
	role?: string | null;
	namespace?: string | null;
	method: string;
	path: string;
	action?: string | null;
	resource_id?: string | null;
	status_code: number;
	success: boolean;
	latency_ms: number;
	error?: string | null;
}

export interface AuditListParams {
	limit?: number;
	offset?: number;
	subject?: string;
	action?: string;
	after?: string;
	before?: string;
}

export async function listAuditLogs(params: AuditListParams = {}): Promise<AuditEntry[]> {
	const sp = new URLSearchParams();
	if (params.limit) sp.set('limit', String(params.limit));
	if (params.offset) sp.set('offset', String(params.offset));
	if (params.subject) sp.set('subject', params.subject);
	if (params.action) sp.set('action', params.action);
	if (params.after) sp.set('after', params.after);
	if (params.before) sp.set('before', params.before);
	const q = sp.toString();
	return fetchJson<AuditEntry[]>(`/api/v1/audit${q ? `?${q}` : ''}`);
}
