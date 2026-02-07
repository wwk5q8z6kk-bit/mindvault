import { fetchJson } from './client';
import type { SearchResultDto } from './types';

export type { SearchResultDto };

export type SavedSearch = {
	id: string;
	name: string;
	description?: string | null;
	query: string;
	search_type: string;
	limit: number;
	namespace: string;
	target_namespace?: string | null;
	kinds: string[];
	tags: string[];
	min_score?: number | null;
	min_importance?: number | null;
	created_at: string;
	updated_at: string;
};

export type CreateSavedSearchPayload = {
	name: string;
	description?: string;
	query: string;
	search_type?: string;
	limit?: number;
	namespace?: string;
	target_namespace?: string;
	kinds?: string[];
	tags?: string[];
	min_score?: number;
	min_importance?: number;
};

export type UpdateSavedSearchPayload = {
	name?: string;
	description?: string | null;
	query?: string;
	search_type?: string;
	limit?: number;
	target_namespace?: string | null;
	kinds?: string[];
	tags?: string[];
	min_score?: number | null;
	min_importance?: number | null;
};

export type SavedSearchRunResponse = {
	saved_search: SavedSearch;
	executed_at: string;
	results: SearchResultDto[];
};

/** Full-text search via GET /api/v1/search */
export async function searchFts(query: string, limit = 20): Promise<SearchResultDto[]> {
	const params = new URLSearchParams({
		q: query,
		type: 'fulltext',
		limit: String(limit)
	});
	return await fetchJson<SearchResultDto[]>(`/api/v1/search?${params.toString()}`);
}

/** Hybrid (vector + FTS) search via POST /api/v1/recall */
export async function searchHybrid(query: string, limit = 20): Promise<SearchResultDto[]> {
	return await fetchJson<SearchResultDto[]>('/api/v1/recall', {
		method: 'POST',
		body: JSON.stringify({ text: query, strategy: 'hybrid', limit })
	});
}

export async function searchFullTextNodes(query: string, limit = 8): Promise<SearchResultDto[]> {
	return await searchFts(query, limit);
}

export async function listSavedSearches(limit = 100, offset = 0): Promise<SavedSearch[]> {
	const params = new URLSearchParams({
		limit: String(limit),
		offset: String(offset)
	});
	return await fetchJson<SavedSearch[]>(`/api/v1/search/saved?${params.toString()}`);
}

export async function createSavedSearch(payload: CreateSavedSearchPayload): Promise<SavedSearch> {
	return await fetchJson<SavedSearch>('/api/v1/search/saved', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function updateSavedSearch(
	id: string,
	payload: UpdateSavedSearchPayload
): Promise<SavedSearch> {
	return await fetchJson<SavedSearch>(`/api/v1/search/saved/${id}`, {
		method: 'PUT',
		body: JSON.stringify(payload)
	});
}

export async function deleteSavedSearch(id: string): Promise<{ deleted: boolean; saved_search_id: string }> {
	return await fetchJson<{ deleted: boolean; saved_search_id: string }>(`/api/v1/search/saved/${id}`, {
		method: 'DELETE'
	});
}

export async function runSavedSearch(id: string): Promise<SavedSearchRunResponse> {
	return await fetchJson<SavedSearchRunResponse>(`/api/v1/search/saved/${id}/run`, {
		method: 'POST'
	});
}
