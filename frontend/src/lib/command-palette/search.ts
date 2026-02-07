import fuzzysort from 'fuzzysort';
import type { CommandAction, RankedAction } from './types';
import { getUsageMap } from './usage';

const RECENT_MS = 1000 * 60 * 60 * 24;

export function rankActions(actions: CommandAction[], query: string): RankedAction[] {
	const usageMap = getUsageMap();

	if (!query.trim()) {
		return actions
			.map((action) => {
				const usage = usageMap[action.id] ?? { count: 0, lastUsed: 0 };
				return {
					action,
					score: usage.count * 10 + (Date.now() - usage.lastUsed < RECENT_MS ? 5 : 0)
				};
			})
			.sort((a, b) => b.score - a.score);
	}

	const targets = actions.map((action) => ({
		...action,
		searchText: [action.title, action.subtitle, ...(action.keywords ?? [])]
			.filter(Boolean)
			.join(' ')
	}));

	const results = fuzzysort.go(query, targets, {
		key: 'searchText',
		threshold: -10000,
	});

	return results
		.map((result) => {
			const usage = usageMap[result.obj.id] ?? { count: 0, lastUsed: 0 };
			const boost =
				Math.log2(usage.count + 1) * 5 + (Date.now() - usage.lastUsed < RECENT_MS ? 2 : 0);
			return { action: result.obj, score: result.score + boost };
		})
		.sort((a, b) => b.score - a.score);
}
