import { fetchJson } from './client';

export type ProposalSender = 'agent' | 'mcp' | 'webhook' | 'watcher' | 'relay' | 'self';
export type ProposalAction =
	| 'create_node'
	| 'update_node'
	| 'delete_node'
	| 'suggest_tag'
	| 'suggest_link'
	| 'schedule_reminder'
	| string;
export type ProposalState = 'pending' | 'approved' | 'rejected' | 'expired' | 'auto_approved';

export interface Proposal {
	id: string;
	node_id: string | null;
	target_node_id: string | null;
	sender: ProposalSender;
	action: ProposalAction;
	state: ProposalState;
	confidence: number;
	diff_preview: string | null;
	payload: Record<string, unknown>;
	created_at: string;
	updated_at: string | null;
	resolved_at: string | null;
}

export interface SubmitProposalRequest {
	sender: ProposalSender;
	action: ProposalAction;
	target_node_id?: string;
	confidence?: number;
	diff_preview?: string;
	payload?: Record<string, unknown>;
}

export interface ProposalCountResponse {
	count: number;
}

export interface ResolveResponse {
	id: string;
	state: ProposalState;
}

/** List proposals with optional state filter. */
export async function listProposals(
	state?: ProposalState,
	limit?: number,
	offset?: number
): Promise<Proposal[]> {
	const params = new URLSearchParams();
	if (state) params.set('state', state);
	if (limit != null) params.set('limit', String(limit));
	if (offset != null) params.set('offset', String(offset));
	const qs = params.toString();
	return fetchJson<Proposal[]>(`/api/v1/exchange/proposals${qs ? `?${qs}` : ''}`);
}

/** Get a single proposal by ID. */
export async function getProposal(id: string): Promise<Proposal> {
	return fetchJson<Proposal>(`/api/v1/exchange/proposals/${encodeURIComponent(id)}`);
}

/** Submit a new proposal. */
export async function submitProposal(req: SubmitProposalRequest): Promise<Proposal> {
	return fetchJson<Proposal>('/api/v1/exchange/proposals', {
		method: 'POST',
		body: JSON.stringify(req)
	});
}

/** Approve a pending proposal. */
export async function approveProposal(id: string): Promise<ResolveResponse> {
	return fetchJson<ResolveResponse>(
		`/api/v1/exchange/proposals/${encodeURIComponent(id)}/approve`,
		{ method: 'POST' }
	);
}

/** Reject a pending proposal. */
export async function rejectProposal(id: string): Promise<ResolveResponse> {
	return fetchJson<ResolveResponse>(
		`/api/v1/exchange/proposals/${encodeURIComponent(id)}/reject`,
		{ method: 'POST' }
	);
}

/** Get the count of pending proposals in the inbox. */
export async function getInboxCount(): Promise<number> {
	const res = await fetchJson<ProposalCountResponse>('/api/v1/exchange/inbox/count');
	return res.count;
}

/** Undo an approved proposal (if within the undo window). */
export async function undoProposal(
	id: string
): Promise<{ id: string; action: string; undone: boolean }> {
	return fetchJson(`/api/v1/exchange/proposals/${encodeURIComponent(id)}/undo`, {
		method: 'POST'
	});
}

export interface BatchResult {
	id: string;
	success: boolean;
	state?: ProposalState;
	error?: string;
	created_node_id?: string;
	updated_node_id?: string;
	deleted_node_id?: string;
}

export interface BatchResponse {
	total: number;
	succeeded: number;
	failed: number;
	results: BatchResult[];
}

/** Batch approve or reject multiple proposals. */
export async function batchProposals(
	action: 'approve' | 'reject',
	proposalIds: string[]
): Promise<BatchResponse> {
	return fetchJson<BatchResponse>('/api/v1/exchange/proposals/batch', {
		method: 'POST',
		body: JSON.stringify({ action, ids: proposalIds })
	});
}
