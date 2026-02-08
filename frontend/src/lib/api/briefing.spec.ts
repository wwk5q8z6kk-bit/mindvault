import { describe, expect, it, vi } from 'vitest';

const fetchJsonMock = vi.fn(async () => ({
	date: '2026-02-08',
	due_today: [],
	overdue: [],
	in_progress: [],
	habits_today: [],
	recent_notes: [],
	summary: 'ok'
}));

vi.mock('./client', () => ({
	fetchJson: fetchJsonMock
}));

describe('briefing api client', () => {
	it('calls versioned briefing endpoint', async () => {
		const { fetchBriefing } = await import('./briefing');
		await fetchBriefing('ops');
		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/briefing?namespace=ops');
	});
});
