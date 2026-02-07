import { describe, expect, it } from 'vitest';
import { estimateTaskMinutes } from './time-blocks';

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
