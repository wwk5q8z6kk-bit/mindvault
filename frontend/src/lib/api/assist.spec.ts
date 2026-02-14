import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { assistAutocomplete, assistCompletion, assistLinks } from './assist';
import { markApiSuccess } from '$lib/api/client';

const API_BASE = 'http://127.0.0.1:9470';

describe('assist api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
		markApiSuccess('/test-init');
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('returns grounded completion fields', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					suggestions: ['Ship the migration checklist this week.'],
					sources: [
						{
							node_id: 'node-1',
							title: 'Migration Plan',
							namespace: 'work',
							score: 0.91
						}
					],
					source_nodes: 12,
					strategy: 'retrieval_heuristic_v1'
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await assistCompletion({
			text: 'Prepare migration rollout',
			limit: 3,
			namespace: 'work'
		});
		expect(result.suggestions).toHaveLength(1);
		expect(result.sources?.[0]?.title).toBe('Migration Plan');
		expect(result.source_nodes).toBe(12);
		expect(result.strategy).toBe('retrieval_heuristic_v1');
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/assist/completion`);
		expect(options.method).toBe('POST');
		expect(JSON.parse(options.body as string)).toEqual({
			text: 'Prepare migration rollout',
			limit: 3,
			namespace: 'work'
		});
	});

	it('passes exclude_node_id in link suggestions payload', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					suggestions: [{ node_id: 'node-2', title: 'Roadmap', heading: 'Q2' }],
					source_nodes: 6,
					strategy: 'retrieval_link_suggestion_v1'
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await assistLinks({
			text: 'roadmap',
			limit: 5,
			namespace: 'work',
			exclude_node_id: 'node-1'
		});
		expect(result.suggestions[0]?.title).toBe('Roadmap');
		expect(result.source_nodes).toBe(6);
		expect(result.strategy).toBe('retrieval_link_suggestion_v1');
		const [, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(JSON.parse(options.body as string)).toEqual({
			text: 'roadmap',
			limit: 5,
			namespace: 'work',
			exclude_node_id: 'node-1'
		});
	});

	it('returns autocomplete metadata fields when present', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					completions: ['Roadmap alignment and delivery review'],
					source_nodes: 4,
					strategy: 'retrieval_autocomplete_v1'
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await assistAutocomplete({ text: 'roadmap', namespace: 'work' });
		expect(result.completions[0]).toContain('Roadmap');
		expect(result.source_nodes).toBe(4);
		expect(result.strategy).toBe('retrieval_autocomplete_v1');
	});
});
