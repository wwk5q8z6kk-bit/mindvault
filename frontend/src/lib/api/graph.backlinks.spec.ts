import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchJson = vi.fn();
vi.mock('./client', () => ({
	fetchJson: (...args: unknown[]) => fetchJson(...args)
}));

import { getNodeBacklinks } from './graph';

describe('getNodeBacklinks', () => {
	beforeEach(() => {
		fetchJson.mockReset();
		fetchJson.mockResolvedValue({
			node_id: 'n1',
			total_backlinks: 0,
			returned_backlinks: 0,
			has_more: false,
			offset: 0,
			limit: 20,
			backlinks: []
		});
	});

	it('calls the dedicated backlinks endpoint with filters', async () => {
		await getNodeBacklinks('abc-123', {
			limit: 10,
		offset: 5,
			include_auto: true,
			include_manual: false,
			source: 'wikilink'
		});

		expect(fetchJson).toHaveBeenCalledWith(
			'/api/v1/nodes/abc-123/backlinks?limit=10&offset=5&include_auto=true&include_manual=false&source=wikilink'
		);
	});

	it('omits query string when no params provided', async () => {
		await getNodeBacklinks('abc-123');
		expect(fetchJson).toHaveBeenCalledWith('/api/v1/nodes/abc-123/backlinks');
	});
});
