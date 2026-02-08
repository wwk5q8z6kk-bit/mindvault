import { fetchJson } from './client';

export interface SyncStats {
	node_count: number;
	last_export: string | null;
	last_import: string | null;
}

export async function syncStatus(): Promise<SyncStats> {
	return fetchJson<SyncStats>('/api/v1/sync/status');
}

export async function syncExport(opts?: {
	namespace?: string;
	since?: string;
}): Promise<unknown> {
	return fetchJson('/api/v1/sync/export', {
		method: 'POST',
		body: JSON.stringify(opts ?? {})
	});
}

export async function syncImport(snapshot: unknown): Promise<{ imported: number }> {
	return fetchJson('/api/v1/sync/import', {
		method: 'POST',
		body: JSON.stringify(snapshot)
	});
}
