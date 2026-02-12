import { fetchJson } from './client';
import { insights } from './agent';
import type { ProactiveInsight } from './types';

export type ConceptMapNode = {
	id: string;
	kind: string;
	title?: string | null;
	tags?: string[];
};

export type ConceptMapCluster = {
	topic: string;
	count: number;
	nodes: ConceptMapNode[];
};

export type ConceptMapResponse = {
	clusters: ConceptMapCluster[];
	total_nodes: number;
	generated_at: string;
};

/** POST /api/v1/insights/{id}/dismiss — mark an insight as dismissed */
export async function dismissInsight(id: string): Promise<void> {
	await fetchJson(`/api/v1/insights/${id}/dismiss`, { method: 'POST' });
	insights.update((list) => list.filter((i) => i.id !== id));
}

/** POST /api/v1/insights/scan — trigger a full insight scan */
export async function fullInsightScan(namespace?: string): Promise<ProactiveInsight[]> {
	const params = new URLSearchParams();
	if (namespace) params.set('namespace', namespace);
	const query = params.toString();
	const url = query ? `/api/v1/insights/scan?${query}` : '/api/v1/insights/scan';
	const data = await fetchJson<ProactiveInsight[]>(url, { method: 'POST' }, { timeoutMs: 60000 });
	insights.update((list) => [...data, ...list]);
	return data;
}

/** GET /api/v1/insights/clusters — detect embedding-space clusters */
export async function getEmbeddingClusters(namespace?: string): Promise<ProactiveInsight[]> {
	const params = new URLSearchParams();
	if (namespace) params.set('namespace', namespace);
	const query = params.toString();
	const url = query ? `/api/v1/insights/clusters?${query}` : '/api/v1/insights/clusters';
	return await fetchJson<ProactiveInsight[]>(url, {}, { timeoutMs: 30000 });
}

/** GET /api/v1/insights/gaps */
export async function getKnowledgeGaps(namespace?: string): Promise<ProactiveInsight[]> {
	const params = new URLSearchParams();
	if (namespace) params.set('namespace', namespace);
	const query = params.toString();
	const url = query ? `/api/v1/insights/gaps?${query}` : '/api/v1/insights/gaps';
	return await fetchJson<ProactiveInsight[]>(url);
}

/** GET /api/v1/insights/concept-map */
export async function getConceptMap(
	namespace?: string,
	maxClusters = 10
): Promise<ConceptMapResponse> {
	const params = new URLSearchParams();
	if (namespace) params.set('namespace', namespace);
	params.set('max_clusters', String(maxClusters));
	return await fetchJson<ConceptMapResponse>(`/api/v1/insights/concept-map?${params.toString()}`);
}

/** GET /api/v1/insights/cross-namespace */
export async function getCrossNamespaceInsights(
	namespaces: string[],
	minOverlap = 2
): Promise<ProactiveInsight[]> {
	if (namespaces.length < 2) return [];
	const params = new URLSearchParams({
		namespaces: namespaces.join(','),
		min_overlap: String(minOverlap)
	});
	return await fetchJson<ProactiveInsight[]>(`/api/v1/insights/cross-namespace?${params.toString()}`);
}
