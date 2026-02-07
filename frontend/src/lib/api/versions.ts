/**
 * Node version history API - view and restore previous versions.
 */

import { fetchJson } from './client';
import type { KnowledgeNode } from './types';

export interface NodeVersionSummary {
	version_id: string;
	version_number: number;
	created_at: string;
	author?: string | null;
	change_summary?: string | null;
	size_bytes: number;
}

export interface NodeVersionDetail {
	version_id: string;
	version_number: number;
	created_at: string;
	author?: string | null;
	change_summary?: string | null;
	snapshot: KnowledgeNode;
	diff_from_previous?: VersionDiff | null;
}

export interface VersionDiff {
	title?: { old: string; new: string };
	content?: { old: string; new: string };
	tags?: { added: string[]; removed: string[] };
	metadata?: Record<string, { old: unknown; new: unknown }>;
}

export interface ListVersionsParams {
	limit?: number;
	offset?: number;
}

export interface ListVersionsResponse {
	versions: NodeVersionSummary[];
	total_count: number;
	node_id: string;
}

/**
 * List all versions of a node.
 */
export async function listNodeVersions(
	nodeId: string,
	params: ListVersionsParams = {}
): Promise<ListVersionsResponse> {
	const searchParams = new URLSearchParams();
	if (params.limit !== undefined) searchParams.set('limit', String(params.limit));
	if (params.offset !== undefined) searchParams.set('offset', String(params.offset));

	const query = searchParams.toString();
	const path = `/api/v1/nodes/${nodeId}/versions${query ? `?${query}` : ''}`;

	return await fetchJson<ListVersionsResponse>(path);
}

/**
 * Get a specific version of a node with full snapshot.
 */
export async function getNodeVersion(
	nodeId: string,
	versionId: string
): Promise<NodeVersionDetail> {
	return await fetchJson<NodeVersionDetail>(
		`/api/v1/nodes/${nodeId}/versions/${versionId}`
	);
}

/**
 * Restore a node to a previous version.
 * Creates a new version with the restored content.
 */
export async function restoreNodeVersion(
	nodeId: string,
	versionId: string
): Promise<KnowledgeNode> {
	return await fetchJson<KnowledgeNode>(
		`/api/v1/nodes/${nodeId}/versions/${versionId}/restore`,
		{ method: 'POST' }
	);
}

/**
 * Compare two versions of a node.
 */
export async function compareVersions(
	nodeId: string,
	fromVersionId: string,
	toVersionId: string
): Promise<VersionDiff> {
	return await fetchJson<VersionDiff>(
		`/api/v1/nodes/${nodeId}/versions/compare?from=${fromVersionId}&to=${toVersionId}`
	);
}

/**
 * Get the latest version info for a node.
 */
export async function getLatestVersion(nodeId: string): Promise<NodeVersionSummary | null> {
	const response = await listNodeVersions(nodeId, { limit: 1 });
	return response.versions[0] || null;
}

/**
 * Format version date for display.
 */
export function formatVersionDate(isoDate: string): string {
	const date = new Date(isoDate);
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

	if (diffDays === 0) {
		return `Today at ${date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`;
	} else if (diffDays === 1) {
		return `Yesterday at ${date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`;
	} else if (diffDays < 7) {
		return `${diffDays} days ago`;
	} else {
		return date.toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
}

/**
 * Format byte size for display.
 */
export function formatVersionSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
