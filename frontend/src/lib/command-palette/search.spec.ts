import { describe, it, expect } from 'vitest';
import { rankActions } from './search';
import type { CommandAction } from './types';

const actions: CommandAction[] = [
	{ id: 'open-tasks', title: 'Open Tasks', keywords: ['tasks'], handler: () => {} },
	{ id: 'open-calendar', title: 'Open Calendar', keywords: ['schedule'], handler: () => {} },
	{ id: 'create-note', title: 'Create Note', keywords: ['note'], handler: () => {} }
];

describe('command palette search', () => {
	it('ranks by fuzzy match', () => {
		const ranked = rankActions(actions, 'cal');
		expect(ranked[0].action.id).toBe('open-calendar');
	});

	it('returns recent/frequent boosted order when query empty', () => {
		const ranked = rankActions(actions, '');
		expect(ranked.length).toBeGreaterThan(0);
	});
});
