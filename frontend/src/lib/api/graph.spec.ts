import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { addRelationship } from './graph';
import { markApiSuccess } from '$lib/stores/api-health';

const API_BASE = 'http://127.0.0.1:9470';

describe('graph api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
		markApiSuccess('/test-init');
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('sends relationship payload using backend field names', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify({ id: 'rel-1' }), {
				status: 201,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		await addRelationship('node-a', 'node-b', 'references');

		expect(fetchMock).toHaveBeenCalledTimes(1);
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/graph/relationships`);
		expect(options.method).toBe('POST');
		expect(JSON.parse(String(options.body))).toEqual({
			from_node: 'node-a',
			to_node: 'node-b',
			kind: 'references'
		});
	});
});
