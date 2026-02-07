import { fetchJson } from './client';

export type SavedViewType = 'list' | 'kanban' | 'calendar';

export interface SavedViewSort {
	field: string;
	direction: 'asc' | 'desc';
}

export interface SavedView {
	id: string;
	name: string;
	namespace: string;
	view_type: SavedViewType;
	group_by?: string | null;
	query?: string | null;
	filters: Record<string, unknown>;
	sort?: SavedViewSort | null;
	updated_at: string;
}

export interface SavedViewListParams {
	namespace?: string;
	limit?: number;
	offset?: number;
}

export interface CreateSavedViewPayload {
	name: string;
	view_type: SavedViewType;
	filters?: Record<string, unknown>;
	sort?: SavedViewSort | null;
	group_by?: string | null;
	query?: string | null;
	namespace?: string;
}

export interface UpdateSavedViewPayload {
	name?: string;
	view_type?: SavedViewType;
	filters?: Record<string, unknown>;
	sort?: SavedViewSort | null;
	group_by?: string | null;
	query?: string | null;
}

export async function listSavedViews(params: SavedViewListParams = {}): Promise<SavedView[]> {
	const query = new URLSearchParams();
	if (params.namespace) query.set('namespace', params.namespace);
	query.set('limit', String(params.limit ?? 200));
	query.set('offset', String(params.offset ?? 0));
	return await fetchJson<SavedView[]>(`/api/v1/saved_views?${query.toString()}`);
}

export async function createSavedView(payload: CreateSavedViewPayload): Promise<SavedView> {
	return await fetchJson<SavedView>('/api/v1/saved_views', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function updateSavedView(
	savedViewId: string,
	payload: UpdateSavedViewPayload
): Promise<SavedView> {
	return await fetchJson<SavedView>(`/api/v1/saved_views/${savedViewId}`, {
		method: 'PATCH',
		body: JSON.stringify(payload)
	});
}

export async function deleteSavedView(savedViewId: string): Promise<{ deleted: boolean }> {
	return await fetchJson<{ deleted: boolean }>(`/api/v1/saved_views/${savedViewId}`, {
		method: 'DELETE'
	});
}
