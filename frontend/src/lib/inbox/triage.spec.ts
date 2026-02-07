import { describe, expect, it } from 'vitest';
import {
	buildInboxTriagePatch,
	buildInboxTriageSuggestions,
	recommendInboxStatus,
	scoreToTaskPriority,
	type InboxTriageSuggestion
} from './triage';

describe('inbox triage', () => {
	it('maps score ranges to task priority', () => {
		expect(scoreToTaskPriority(0.9)).toBe(1);
		expect(scoreToTaskPriority(0.7)).toBe(2);
		expect(scoreToTaskPriority(0.5)).toBe(3);
		expect(scoreToTaskPriority(0.3)).toBe(4);
		expect(scoreToTaskPriority(0.1)).toBe(5);
	});

	it('recommends in-progress only for top urgent items', () => {
		expect(recommendInboxStatus(1, 1)).toBe('in_progress');
		expect(recommendInboxStatus(2, 2)).toBe('planned');
		expect(recommendInboxStatus(6, 4)).toBe('inbox');
	});

	it('builds suggestions and patches only changed fields', () => {
		const suggestions = buildInboxTriageSuggestions([
			{
				task: {
					id: 'task-1',
					title: 'Review report',
					description: null,
					status: 'inbox',
					priority: 4,
					labels: [],
					dependencies: [],
					metadata: {},
					created_at: '2026-01-01T00:00:00.000Z',
					updated_at: '2026-01-01T00:00:00.000Z'
				},
				score: 0.88,
				rank: 1,
				reason: 'Due soon and blocked tasks depend on it'
			}
		]);
		const suggestion = suggestions.get('task-1') as InboxTriageSuggestion;
		expect(suggestion.suggestedPriority).toBe(1);
		expect(suggestion.suggestedStatus).toBe('in_progress');

		const patch = buildInboxTriagePatch(
			{ status: 'inbox', priority: 4 },
			suggestion
		);
		expect(patch).toEqual({ priority: 1, status: 'in_progress' });

		const noOpPatch = buildInboxTriagePatch(
			{ status: 'inbox', priority: suggestion.suggestedPriority },
			{ ...suggestion, suggestedStatus: 'inbox' }
		);
		expect(noOpPatch).toBeNull();
	});
});
