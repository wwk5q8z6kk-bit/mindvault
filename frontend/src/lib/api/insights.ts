import { fetchJson } from './client';
import { insights } from './agent';
import type { ProactiveInsight } from './types';

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
