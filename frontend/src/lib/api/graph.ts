import { fetchJson } from './client';
import type { KnowledgeNode } from './types';

export interface GraphNeighbor {
	node: KnowledgeNode;
	relationship_kind: string;
	direction: 'incoming' | 'outgoing';
}

export interface GraphNeighborsResponse {
	center_id: string;
	neighbors: GraphNeighbor[];
}

export async function getNeighbors(
	nodeId: string,
	depth = 1
): Promise<GraphNeighborsResponse> {
	return await fetchJson<GraphNeighborsResponse>(
		`/api/v1/graph/neighbors/${nodeId}?depth=${depth}`
	);
}

export async function addRelationship(
	from: string,
	to: string,
	kind: string
): Promise<void> {
	await fetchJson('/api/v1/graph/relationships', {
		method: 'POST',
		body: JSON.stringify({ from, to, kind })
	});
}

// ============================================================================
// Relationship Queries
// ============================================================================

export interface NodeRelationship {
	id: string;
	from_node_id: string;
	to_node_id: string;
	kind: string;
	weight?: number;
	metadata?: Record<string, unknown>;
	created_at: string;
}

export interface NodeRelationshipsResponse {
	node_id: string;
	incoming: NodeRelationship[];
	outgoing: NodeRelationship[];
}

/**
 * Get all relationships for a node (both incoming and outgoing).
 */
export async function getNodeRelationships(nodeId: string): Promise<NodeRelationshipsResponse> {
	return await fetchJson<NodeRelationshipsResponse>(
		`/api/v1/graph/relationships/${nodeId}`
	);
}

/**
 * Delete a relationship by ID.
 */
export async function deleteRelationship(relationshipId: string): Promise<void> {
	await fetchJson(`/api/v1/graph/relationships/${relationshipId}`, {
		method: 'DELETE'
	});
}

/**
 * Update relationship metadata or weight.
 */
export async function updateRelationship(
	relationshipId: string,
	updates: { weight?: number; metadata?: Record<string, unknown> }
): Promise<NodeRelationship> {
	return await fetchJson<NodeRelationship>(
		`/api/v1/graph/relationships/${relationshipId}`,
		{
			method: 'PATCH',
			body: JSON.stringify(updates)
		}
	);
}

// ============================================================================
// Graph Analysis
// ============================================================================

export interface GraphCluster {
	id: string;
	name?: string;
	node_ids: string[];
	center_node_id: string;
	density: number;
}

export interface GraphClustersResponse {
	clusters: GraphCluster[];
	total_nodes: number;
	unclustered_count: number;
}

/**
 * Get clusters of related nodes.
 */
export async function getGraphClusters(params?: {
	min_size?: number;
	max_clusters?: number;
	namespace?: string;
}): Promise<GraphClustersResponse> {
	const searchParams = new URLSearchParams();
	if (params?.min_size !== undefined) searchParams.set('min_size', String(params.min_size));
	if (params?.max_clusters !== undefined)
		searchParams.set('max_clusters', String(params.max_clusters));
	if (params?.namespace) searchParams.set('namespace', params.namespace);

	const query = searchParams.toString();
	return await fetchJson<GraphClustersResponse>(
		`/api/v1/graph/clusters${query ? `?${query}` : ''}`
	);
}

export interface GraphPathResponse {
	paths: GraphPath[];
	shortest_length: number;
}

export interface GraphPath {
	nodes: string[];
	relationships: string[];
	total_weight: number;
}

/**
 * Find paths between two nodes.
 */
export async function findPaths(
	fromId: string,
	toId: string,
	params?: { max_depth?: number; max_paths?: number }
): Promise<GraphPathResponse> {
	const searchParams = new URLSearchParams();
	searchParams.set('from', fromId);
	searchParams.set('to', toId);
	if (params?.max_depth !== undefined) searchParams.set('max_depth', String(params.max_depth));
	if (params?.max_paths !== undefined) searchParams.set('max_paths', String(params.max_paths));

	return await fetchJson<GraphPathResponse>(`/api/v1/graph/paths?${searchParams.toString()}`);
}

export interface GraphSearchParams {
	/** Starting node ID */
	start_id: string;
	/** Relationship kinds to follow */
	relationship_kinds?: string[];
	/** Maximum depth to search */
	max_depth?: number;
	/** Node kinds to include in results */
	node_kinds?: string[];
	/** Minimum relevance score */
	min_score?: number;
}

export interface GraphSearchResult {
	node: KnowledgeNode;
	path_length: number;
	score: number;
	via_relationships: string[];
}

export interface GraphSearchResponse {
	results: GraphSearchResult[];
	total_explored: number;
}

/**
 * Search the graph starting from a node.
 */
export async function searchGraph(params: GraphSearchParams): Promise<GraphSearchResponse> {
	return await fetchJson<GraphSearchResponse>('/api/v1/graph/search', {
		method: 'POST',
		body: JSON.stringify(params)
	});
}
