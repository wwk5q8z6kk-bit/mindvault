import { fetchJson } from './client';

export interface SyncStats {
	device_id: string;
	status: string;
	node_count: number;
	last_export: string | null;
	last_import: string | null;
}

export interface SyncConflict {
	id: string;
	node_id: string;
	reason: string;
	local_updated_at: string;
	remote_updated_at: string;
	remote_device_id: string;
	local_content_preview: string;
	remote_content_preview: string;
	proposal_id: string | null;
	resolved: boolean;
	detected_at: string;
}

export interface SyncImportStats {
	scanned: number;
	inserted: number;
	updated: number;
	skipped: number;
	conflicts: number;
	conflict_details: SyncConflict[];
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

export async function syncImport(snapshot: unknown): Promise<SyncImportStats> {
	return fetchJson<SyncImportStats>('/api/v1/sync/import', {
		method: 'POST',
		body: JSON.stringify(snapshot)
	});
}
