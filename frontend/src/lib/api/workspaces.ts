import { fetchJson } from './client';

export type WorkspaceTreeEntryKind = 'directory' | 'document';
export type WorkspaceManifestMatch = 'current' | 'changed' | 'untracked';
export type WorkspaceProjectionState = 'pending' | 'ready' | 'failed' | 'stale';

export interface WorkspaceSummary {
	id: string;
	namespace: string;
	display_name: string;
	root_name: string;
	mode: string;
	state: string;
	revision: number;
	last_reconciled_at: string | null;
}

export interface WorkspacePathIssue {
	code: string;
	[key: string]: unknown;
}

export interface WorkspaceScanDiagnostic {
	severity: 'warning' | 'error';
	code: string;
	relative_path: string | null;
	message: string;
}

export interface WorkspaceTreeEntry {
	kind: WorkspaceTreeEntryKind;
	relative_path: string;
	parent_path: string | null;
	name: string;
	document_id: string | null;
	lifecycle: string | null;
	projection_state: WorkspaceProjectionState | null;
	projected_node_id: string | null;
	content_status: string | null;
	manifest_match: WorkspaceManifestMatch | null;
	content_hash: string | null;
	byte_size: number | null;
	modified_at: string | null;
	portable: boolean;
	portability_issues: WorkspacePathIssue[];
}

export interface WorkspaceTree {
	workspace_id: string;
	workspace_revision: number;
	scanned_at: string;
	entries: WorkspaceTreeEntry[];
	diagnostics: WorkspaceScanDiagnostic[];
}

export interface WorkspaceDocumentRead {
	workspace_id: string;
	document_id: string;
	relative_path: string;
	content: string;
	content_hash: string;
	byte_size: number;
	modified_at: string | null;
	manifest_match: WorkspaceManifestMatch;
}

export interface WorkspaceReconciliation {
	applied: boolean;
	inserted_documents: number;
	updated_documents: number;
	unchanged_documents: number;
	renamed_documents: number;
	projection: WorkspaceProjectionOutcome;
	diagnostics: WorkspaceScanDiagnostic[];
}

export interface WorkspaceProjectionOutcome {
	attempted_documents: number;
	projected_documents: number;
	removed_documents: number;
	unchanged_documents: number;
	failed_documents: number;
}

export interface RebuildWorkspaceProjectionsResponse {
	workspace: WorkspaceSummary;
	projection: WorkspaceProjectionOutcome;
}

export interface ReconcileWorkspaceResponse {
	workspace: WorkspaceSummary;
	reconciliation: WorkspaceReconciliation;
}

export interface MountWorkspaceRequest {
	root_path: string;
	namespace?: string;
	display_name?: string;
}

export interface MountWorkspaceResponse {
	workspace: WorkspaceSummary;
	reconciliation: WorkspaceReconciliation;
}

export function listWorkspaces(namespace?: string): Promise<WorkspaceSummary[]> {
	const params = new URLSearchParams();
	if (namespace) params.set('namespace', namespace);
	const query = params.toString();
	return fetchJson<WorkspaceSummary[]>(`/api/v1/workspaces${query ? `?${query}` : ''}`);
}

export function mountWorkspace(request: MountWorkspaceRequest): Promise<MountWorkspaceResponse> {
	return fetchJson<MountWorkspaceResponse>('/api/v1/workspaces', {
		method: 'POST',
		body: JSON.stringify(request)
	});
}

export function getWorkspaceTree(workspaceId: string): Promise<WorkspaceTree> {
	return fetchJson<WorkspaceTree>(`/api/v1/workspaces/${encodeURIComponent(workspaceId)}/tree`);
}

export function readWorkspaceDocument(
	workspaceId: string,
	documentId: string
): Promise<WorkspaceDocumentRead> {
	return fetchJson<WorkspaceDocumentRead>(
		`/api/v1/workspaces/${encodeURIComponent(workspaceId)}/documents/${encodeURIComponent(documentId)}`
	);
}

export function reconcileWorkspace(workspaceId: string): Promise<ReconcileWorkspaceResponse> {
	return fetchJson<ReconcileWorkspaceResponse>(
		`/api/v1/workspaces/${encodeURIComponent(workspaceId)}/reconcile`,
		{ method: 'POST' }
	);
}

export function rebuildWorkspaceProjections(
	workspaceId: string
): Promise<RebuildWorkspaceProjectionsResponse> {
	return fetchJson<RebuildWorkspaceProjectionsResponse>(
		`/api/v1/workspaces/${encodeURIComponent(workspaceId)}/projections/rebuild`,
		{ method: 'POST' }
	);
}
