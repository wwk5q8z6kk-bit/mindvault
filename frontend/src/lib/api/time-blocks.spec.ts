import { describe, expect, it } from 'vitest';
import {
	computeRescheduleSuggestions,
	estimateTaskMinutes,
	isAiTimeBlockCalendarItem,
	type ExistingTimeBlock
} from './time-blocks';

describe('estimateTaskMinutes', () => {
	it('prefers direct estimate_min field when available', () => {
		expect(estimateTaskMinutes({ estimate_min: 35, metadata: {} })).toBe(35);
	});

	it('falls back to metadata estimate keys', () => {
		expect(
			estimateTaskMinutes({ metadata: { task_estimate_minutes: 55 } })
		).toBe(55);
		expect(
			estimateTaskMinutes({ metadata: { task_estimate_min: '40' } })
		).toBe(40);
		expect(
			estimateTaskMinutes({ metadata: { estimate_min: 25 } })
		).toBe(25);
	});

	it('returns null when no estimate can be inferred', () => {
		expect(estimateTaskMinutes({ metadata: {} })).toBeNull();
		expect(estimateTaskMinutes({ metadata: { estimate_min: 'n/a' } })).toBeNull();
	});
});

describe('isAiTimeBlockCalendarItem', () => {
	it('detects events tagged with time block task metadata', () => {
		expect(
			isAiTimeBlockCalendarItem({
				id: 'event-1',
				title: 'Focus',
				kind: 'event',
				start: '2026-02-08T09:00:00.000Z',
				end: '2026-02-08T09:45:00.000Z',
				all_day: false,
				node: {
					id: 'event-1',
					kind: 'event',
					title: 'Focus',
					content: '',
					source: 'assistant',
					namespace: 'default',
					tags: ['time-block'],
					importance: 0.5,
					temporal: {
						created_at: '2026-02-08T08:00:00.000Z',
						updated_at: '2026-02-08T08:00:00.000Z'
					},
					metadata: { time_block_task_id: 'task-1' }
				}
			})
		).toBe(true);
	});
});

describe('computeRescheduleSuggestions', () => {
	it('reflows existing blocks around fixed busy windows', () => {
		const existing: ExistingTimeBlock[] = [
			{
				eventId: 'event-1',
				taskId: 'task-1',
				taskTitle: 'Plan sprint',
				start: '2026-02-08T09:00:00.000Z',
				end: '2026-02-08T10:00:00.000Z',
				score: 0.9,
				reason: 'High urgency',
				durationMinutes: 60,
				dueAt: null
			}
		];
		const suggestions = computeRescheduleSuggestions(
			existing,
			[{ start: '2026-02-08T09:00:00.000Z', end: '2026-02-08T10:30:00.000Z' }],
			{
				date: '2026-02-08',
				workdayStartHour: 9,
				workdayEndHour: 12,
				defaultBlockMinutes: 45
			}
		);

		expect(suggestions).toHaveLength(1);
		expect(suggestions[0].eventId).toBe('event-1');
		expect(suggestions[0].start).toBe('2026-02-08T10:30:00.000Z');
		expect(suggestions[0].end).toBe('2026-02-08T11:30:00.000Z');
	});

	it('drops suggestions that do not move block timing', () => {
		const existing: ExistingTimeBlock[] = [
			{
				eventId: 'event-1',
				taskId: 'task-1',
				taskTitle: 'Plan sprint',
				start: '2026-02-08T09:00:00.000Z',
				end: '2026-02-08T10:00:00.000Z',
				score: 0.9,
				reason: null,
				durationMinutes: 60,
				dueAt: null
			}
		];
		const suggestions = computeRescheduleSuggestions(existing, [], {
			date: '2026-02-08',
			workdayStartHour: 9,
			workdayEndHour: 12,
			defaultBlockMinutes: 45
		});
		expect(suggestions).toHaveLength(0);
	});
});
