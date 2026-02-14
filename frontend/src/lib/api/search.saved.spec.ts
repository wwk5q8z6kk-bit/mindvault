import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	createSavedSearch,
	deleteSavedSearch,
	listSavedSearches,
	runSavedSearch,
	updateSavedSearch,
	type SavedSearch
} from './search';
import { markApiSuccess } from '$lib/api/client';

const API_BASE = 'http://127.0.0.1:9470';

describe('saved search api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
		markApiSuccess('/test-init');
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('lists saved searches with pagination params', async () => {
		const payload: SavedSearch[] = [];
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify(payload), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		const result = await listSavedSearches(25, 5);

		expect(result).toEqual(payload);
		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(fetchMock).toHaveBeenCalledWith(
			`${API_BASE}/api/v1/search/saved?limit=25&offset=5`,
			expect.objectContaining({
				headers: expect.objectContaining({ 'Content-Type': 'application/json' })
			})
		);
	});

	it('creates saved search via POST', async () => {
		const payload = {
			id: 'search-1',
			name: 'Urgent work',
			description: null,
			query: 'status:planned',
			search_type: 'hybrid',
			limit: 50,
			namespace: 'default',
			target_namespace: null,
			kinds: ['task'],
			tags: ['urgent'],
			min_score: null,
			min_importance: null,
			created_at: '2026-02-06T00:00:00Z',
			updated_at: '2026-02-06T00:00:00Z'
		};
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify(payload), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		const result = await createSavedSearch({
			name: 'Urgent work',
			query: 'status:planned',
			search_type: 'hybrid',
			kinds: ['task'],
			tags: ['urgent']
		});

		expect(result).toEqual(payload);
		const [, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(options.method).toBe('POST');
		expect(JSON.parse(options.body as string)).toEqual({
			name: 'Urgent work',
			query: 'status:planned',
			search_type: 'hybrid',
			kinds: ['task'],
			tags: ['urgent']
		});
	});

	it('updates and deletes saved search by id', async () => {
		fetchMock
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify({
						id: 'search-1',
						name: 'Updated',
						description: null,
						query: 'a',
						search_type: 'fulltext',
						limit: 10,
						namespace: 'default',
						target_namespace: null,
						kinds: [],
						tags: [],
						min_score: null,
						min_importance: null,
						created_at: '2026-02-06T00:00:00Z',
						updated_at: '2026-02-06T01:00:00Z'
					}),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			)
			.mockResolvedValueOnce(
				new Response(JSON.stringify({ deleted: true, saved_search_id: 'search-1' }), {
					status: 200,
					headers: { 'Content-Type': 'application/json' }
				})
			);

		await updateSavedSearch('search-1', { name: 'Updated' });
		const [, updateOptions] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(updateOptions.method).toBe('PUT');
		expect(JSON.parse(updateOptions.body as string)).toEqual({ name: 'Updated' });

		const deleted = await deleteSavedSearch('search-1');
		expect(deleted).toEqual({ deleted: true, saved_search_id: 'search-1' });
		expect(fetchMock.mock.calls[1][0]).toBe(`${API_BASE}/api/v1/search/saved/search-1`);
		const [, deleteOptions] = fetchMock.mock.calls[1] as [string, RequestInit];
		expect(deleteOptions.method).toBe('DELETE');
	});

	it('runs a saved search via POST run endpoint', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					saved_search: {
						id: 'search-1',
						name: 'My Search',
						description: null,
						query: 'tasks',
						search_type: 'hybrid',
						limit: 10,
						namespace: 'default',
						target_namespace: null,
						kinds: [],
						tags: [],
						min_score: null,
						min_importance: null,
						created_at: '2026-02-06T00:00:00Z',
						updated_at: '2026-02-06T00:00:00Z'
					},
					executed_at: '2026-02-06T01:00:00Z',
					results: []
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const response = await runSavedSearch('search-1');
		expect(response.results).toEqual([]);
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/search/saved/search-1/run`);
		expect(options.method).toBe('POST');
	});
});
