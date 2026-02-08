import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getCalendarItems } from './calendar';

const API_BASE = 'http://127.0.0.1:9470';

describe('calendar api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('maps date to anchor and forwards include_tasks filter', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					items: [],
					view: 'day',
					range_start: '2026-02-09T00:00:00.000Z',
					range_end: '2026-02-10T00:00:00.000Z'
				}),
				{
					status: 200,
					headers: { 'Content-Type': 'application/json' }
				}
			)
		);

		await getCalendarItems({
			date: '2026-02-09',
			view: 'day',
			include_tasks: false,
			namespace: 'ops'
		});

		expect(fetchMock).toHaveBeenCalledTimes(1);
		const [url] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(
			`${API_BASE}/api/v1/calendar/items?date=2026-02-09&view=day&include_tasks=false&namespace=ops`
		);
	});
});
