import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { enrichClip } from './clips';

const API_BASE = 'http://127.0.0.1:9470';

describe('clips api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('calls clip enrichment endpoint', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					normalized_url: 'https://example.com/article',
					title: 'Example article',
					description: 'Summary',
					suggested_tags: ['example', 'article'],
					fetched: true
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await enrichClip({ url: 'example.com/article' });
		expect(result.normalized_url).toBe('https://example.com/article');
		expect(fetchMock).toHaveBeenCalledTimes(1);
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/clips/enrich`);
		expect(options.method).toBe('POST');
		expect(JSON.parse(String(options.body))).toEqual({
			url: 'example.com/article'
		});
	});

	it('throws on empty url before network request', async () => {
		await expect(enrichClip({ url: '' })).rejects.toThrow('URL is required');
		expect(fetchMock).not.toHaveBeenCalled();
	});
});
