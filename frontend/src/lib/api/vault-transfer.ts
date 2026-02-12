import { fetchJson } from './client';
import type { KnowledgeNode } from './types';

export interface TransferRelationship {
	id: string;
	from_node: string;
	to_node: string;
	kind: string;
	metadata?: Record<string, unknown>;
	weight?: number;
	created_at: string;
}

export interface VaultTransferBundle {
	format_version: string;
	exported_at: string;
	scope_namespace?: string | null;
	nodes: KnowledgeNode[];
	relationships: TransferRelationship[];
}

export interface ImportBundleRequest {
	nodes: KnowledgeNode[];
	relationships?: TransferRelationship[];
	overwrite_existing?: boolean;
	include_relationships?: boolean;
	namespace_override?: string;
}

export interface ImportBundleResponse {
	imported_nodes: number;
	updated_nodes: number;
	skipped_nodes: number;
	imported_relationships: number;
	skipped_relationships: number;
}

export async function exportVaultBundle(options?: {
	namespace?: string;
	includeRelationships?: boolean;
}): Promise<VaultTransferBundle> {
	const params = new URLSearchParams();
	if (options?.namespace) params.set('namespace', options.namespace);
	if (options?.includeRelationships !== undefined) {
		params.set('include_relationships', String(options.includeRelationships));
	}
	const query = params.toString();
	return await fetchJson<VaultTransferBundle>(`/api/v1/export${query ? `?${query}` : ''}`);
}

export async function importVaultBundle(
	request: ImportBundleRequest
): Promise<ImportBundleResponse> {
	return await fetchJson<ImportBundleResponse>('/api/v1/import', {
		method: 'POST',
		body: JSON.stringify(request)
	});
}
