import { describe, expect, it } from 'vitest';
import {
	calculateBestStreak,
	calculateCurrentStreak,
	latestCheckinDate,
	normalizeDateList,
	toggleCheckinDate
} from './habits';

describe('normalizeDateList', () => {
	it('deduplicates, validates, and sorts dates', () => {
		expect(
			normalizeDateList([
				'2026-02-06',
				'2026-02-05',
				'2026-02-06',
				'bad-date',
				'2026-01-31'
			])
		).toEqual(['2026-01-31', '2026-02-05', '2026-02-06']);
	});
});

describe('toggleCheckinDate', () => {
	it('adds and removes a checkin date', () => {
		const first = toggleCheckinDate(['2026-02-05'], '2026-02-06');
		expect(first).toEqual(['2026-02-05', '2026-02-06']);
		expect(toggleCheckinDate(first, '2026-02-05')).toEqual(['2026-02-06']);
	});
});

describe('streak calculations', () => {
	it('calculates current streak ending yesterday', () => {
		const now = new Date('2026-02-06T09:00:00.000Z');
		const streak = calculateCurrentStreak(
			['2026-02-02', '2026-02-03', '2026-02-04', '2026-02-05'],
			now
		);
		expect(streak).toBe(4);
	});

	it('returns 0 when latest checkin is stale', () => {
		const now = new Date('2026-02-10T09:00:00.000Z');
		expect(calculateCurrentStreak(['2026-02-05', '2026-02-06'], now)).toBe(0);
	});

	it('calculates best streak and latest date', () => {
		const dates = ['2026-02-01', '2026-02-02', '2026-02-04', '2026-02-05', '2026-02-06'];
		expect(calculateBestStreak(dates)).toBe(3);
		expect(latestCheckinDate(dates)).toBe('2026-02-06');
	});
});
