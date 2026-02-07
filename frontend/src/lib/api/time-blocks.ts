import { prioritizeTasks } from './ai';
import { getCalendarItems } from './calendar';
import { createNode } from './nodes';
import { computeTimeBlockSuggestions, type TimeBlockSuggestion } from '$lib/automation/time-blocks';

export type { TimeBlockSuggestion } from '$lib/automation/time-blocks';

export type SuggestTimeBlocksParams = {
	date: string;
	namespace?: string | null;
	limit?: number;
	workdayStartHour?: number;
	workdayEndHour?: number;
	defaultBlockMinutes?: number;
};

export function estimateTaskMinutes(task: {
	estimate_min?: number | null;
	metadata?: Record<string, unknown>;
}): number | null {
	const direct = task.estimate_min;
	if (typeof direct === 'number' && Number.isFinite(direct) && direct > 0) {
		return direct;
	}

	const metadata = task.metadata ?? {};
	for (const key of ['task_estimate_minutes', 'task_estimate_min', 'estimate_min']) {
		const value = metadata[key];
		if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
			return value;
		}
		if (typeof value === 'string') {
			const parsed = Number(value);
			if (Number.isFinite(parsed) && parsed > 0) {
				return parsed;
			}
		}
	}
	return null;
}

export async function suggestTimeBlocks(
	params: SuggestTimeBlocksParams
): Promise<TimeBlockSuggestion[]> {
	const [prioritized, calendar] = await Promise.all([
		prioritizeTasks({
			namespace: params.namespace ?? undefined,
			limit: Math.max((params.limit ?? 6) * 2, 8),
			include_done: false,
			statuses: ['inbox', 'planned', 'in_progress']
		}),
		getCalendarItems({
			date: params.date,
			view: 'day',
			include_tasks: true,
			namespace: params.namespace ?? undefined
		})
	]);

	const candidates = prioritized.items.map((item) => ({
		id: item.task.id,
		title: item.task.title || 'Untitled task',
		score: item.score,
		estimateMinutes: estimateTaskMinutes(item.task),
		dueAt: item.task.due_at ?? null,
		reason: item.reason ?? null
	}));

	const busy = calendar.items.map((item) => ({
		start: item.start,
		end: item.end ?? null
	}));

	return computeTimeBlockSuggestions(candidates, busy, {
		date: params.date,
		limit: params.limit ?? 6,
		workdayStartHour: params.workdayStartHour ?? 9,
		workdayEndHour: params.workdayEndHour ?? 18,
		defaultBlockMinutes: params.defaultBlockMinutes ?? 45
	});
}

export async function applyTimeBlocks(
	suggestions: TimeBlockSuggestion[],
	params: { namespace?: string | null } = {}
): Promise<{ created: number; ids: string[] }> {
	const ids: string[] = [];
	for (const suggestion of suggestions) {
		const created = await createNode({
			kind: 'event',
			title: `Focus: ${suggestion.taskTitle}`,
			content: [
				`Planned focus block for task: ${suggestion.taskTitle}`,
				`Task ID: ${suggestion.taskId}`,
				suggestion.reason ? `Reason: ${suggestion.reason}` : '',
				`Score: ${suggestion.score.toFixed(3)}`
			]
				.filter(Boolean)
				.join('\n'),
			source: 'assistant:time-blocks',
			namespace: params.namespace ?? undefined,
			tags: ['time-block', 'ai-suggested'],
			metadata: {
				event_start_at: suggestion.start,
				event_end_at: suggestion.end,
				time_block_task_id: suggestion.taskId,
				time_block_score: suggestion.score,
				time_block_reason: suggestion.reason ?? null
			}
		});
		ids.push(created.id);
	}
	return { created: ids.length, ids };
}
