import { describe, expect, it } from 'vitest';
import {
	computeTimeBlockSuggestions,
	type TimeBlockBusyWindow,
	type TimeBlockTaskCandidate
} from './time-blocks';

describe('computeTimeBlockSuggestions', () => {
	it('schedules highest-priority tasks into earliest free slots', () => {
		const candidates: TimeBlockTaskCandidate[] = [
			{ id: 'task-a', title: 'Plan sprint', score: 0.95, estimateMinutes: 60 },
			{ id: 'task-b', title: 'Review PRs', score: 0.8, estimateMinutes: 30 }
		];
		const busy: TimeBlockBusyWindow[] = [
			{ start: '2026-02-08T09:00:00.000Z', end: '2026-02-08T09:30:00.000Z' }
		];

		const suggestions = computeTimeBlockSuggestions(candidates, busy, {
			date: '2026-02-08',
			workdayStartHour: 9,
			workdayEndHour: 12,
			defaultBlockMinutes: 45,
			limit: 3
		});

		expect(suggestions).toHaveLength(2);
		expect(suggestions[0].taskId).toBe('task-a');
		expect(suggestions[0].durationMinutes).toBe(60);
		expect(suggestions[0].start).toBe('2026-02-08T09:30:00.000Z');
		expect(suggestions[1].taskId).toBe('task-b');
		expect(suggestions[1].start).toBe('2026-02-08T10:30:00.000Z');
	});

	it('uses fallback duration when estimate is missing', () => {
		const candidates: TimeBlockTaskCandidate[] = [
			{ id: 'task-a', title: 'Write notes', score: 0.7, estimateMinutes: null }
		];
		const suggestions = computeTimeBlockSuggestions(candidates, [], {
			date: '2026-02-08',
			workdayStartHour: 13,
			workdayEndHour: 15,
			defaultBlockMinutes: 50,
			limit: 2
		});
		expect(suggestions).toHaveLength(1);
		expect(suggestions[0].durationMinutes).toBe(50);
		expect(suggestions[0].start).toBe('2026-02-08T13:00:00.000Z');
		expect(suggestions[0].end).toBe('2026-02-08T13:50:00.000Z');
	});

	it('skips tasks when no slot can fit the estimated duration', () => {
		const candidates: TimeBlockTaskCandidate[] = [
			{ id: 'task-a', title: 'Deep work', score: 0.9, estimateMinutes: 120 }
		];
		const busy: TimeBlockBusyWindow[] = [
			{ start: '2026-02-08T09:00:00.000Z', end: '2026-02-08T10:00:00.000Z' },
			{ start: '2026-02-08T10:15:00.000Z', end: '2026-02-08T11:00:00.000Z' }
		];
		const suggestions = computeTimeBlockSuggestions(candidates, busy, {
			date: '2026-02-08',
			workdayStartHour: 9,
			workdayEndHour: 11,
			defaultBlockMinutes: 45,
			limit: 3
		});
		expect(suggestions).toHaveLength(0);
	});
});
