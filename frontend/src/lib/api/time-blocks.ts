import {
	computeTimeBlockSuggestions,
	type TimeBlockBusyWindow,
	type TimeBlockSuggestion
} from '$lib/automation/time-blocks';
import { prioritizeTasks } from './ai';
import type { CalendarItem } from './calendar';
import { getCalendarItems } from './calendar';
import { fetchJson } from './client';
import { addRelationship } from './graph';
import { createNode } from './nodes';
import type { KnowledgeNode } from './types';

export type { TimeBlockSuggestion } from '$lib/automation/time-blocks';

export type SuggestTimeBlocksParams = {
	date: string;
	namespace?: string | null;
	limit?: number;
	workdayStartHour?: number;
	workdayEndHour?: number;
	defaultBlockMinutes?: number;
};

export type ExistingTimeBlock = {
	eventId: string;
	taskId: string;
	taskTitle: string;
	start: string;
	end: string;
	score: number;
	reason: string | null;
	durationMinutes: number;
	dueAt: string | null;
};

export type TimeBlockRescheduleSuggestion = {
	eventId: string;
	taskId: string;
	taskTitle: string;
	score: number;
	reason: string | null;
	previousStart: string;
	previousEnd: string;
	start: string;
	end: string;
	durationMinutes: number;
	dueAt: string | null;
};

type RescheduleOptions = {
	date: string;
	limit?: number;
	workdayStartHour?: number;
	workdayEndHour?: number;
	defaultBlockMinutes?: number;
};

const TIME_BLOCK_TASK_ID_KEY = 'time_block_task_id';
const TIME_BLOCK_TASK_TITLE_KEY = 'time_block_task_title';
const TIME_BLOCK_SCORE_KEY = 'time_block_score';
const TIME_BLOCK_REASON_KEY = 'time_block_reason';
const TIME_BLOCK_DUE_AT_KEY = 'time_block_due_at';

function toIsoStringOrNull(value: unknown): string | null {
	if (typeof value !== 'string' || value.trim().length === 0) return null;
	const millis = new Date(value).getTime();
	if (!Number.isFinite(millis)) return null;
	return new Date(millis).toISOString();
}

function normalizeDurationMinutes(start: string, end: string, fallbackMinutes: number): number {
	const startMs = new Date(start).getTime();
	const endMs = new Date(end).getTime();
	if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || endMs <= startMs) {
		return fallbackMinutes;
	}
	const duration = Math.round((endMs - startMs) / 60000);
	if (!Number.isFinite(duration) || duration <= 0) {
		return fallbackMinutes;
	}
	return duration;
}

function metadataString(metadata: Record<string, unknown>, key: string): string | null {
	const raw = metadata[key];
	return typeof raw === 'string' && raw.trim().length > 0 ? raw.trim() : null;
}

function metadataNumber(metadata: Record<string, unknown>, key: string): number | null {
	const raw = metadata[key];
	if (typeof raw === 'number' && Number.isFinite(raw)) return raw;
	if (typeof raw === 'string') {
		const parsed = Number(raw);
		if (Number.isFinite(parsed)) return parsed;
	}
	return null;
}

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

export function isAiTimeBlockCalendarItem(item: CalendarItem): boolean {
	if (item.kind !== 'event') return false;
	const metadata = item.node.metadata ?? {};
	return metadataString(metadata, TIME_BLOCK_TASK_ID_KEY) !== null;
}

function calendarBusyWindows(items: CalendarItem[]): TimeBlockBusyWindow[] {
	return items.map((item) => ({
		start: item.start,
		end: item.end ?? null
	}));
}

function buildExistingTimeBlocks(
	items: CalendarItem[],
	defaultBlockMinutes: number
): ExistingTimeBlock[] {
	const blocks: ExistingTimeBlock[] = [];
	for (const item of items) {
		if (!isAiTimeBlockCalendarItem(item)) continue;
		const metadata = item.node.metadata ?? {};
		const taskId = metadataString(metadata, TIME_BLOCK_TASK_ID_KEY);
		if (!taskId) continue;
		const start = toIsoStringOrNull(item.start);
		const end = toIsoStringOrNull(item.end);
		if (!start || !end) continue;
		blocks.push({
			eventId: item.id,
			taskId,
			taskTitle:
				metadataString(metadata, TIME_BLOCK_TASK_TITLE_KEY) ??
				(item.title || 'Time Block'),
			start,
			end,
			score: metadataNumber(metadata, TIME_BLOCK_SCORE_KEY) ?? 0.5,
			reason: metadataString(metadata, TIME_BLOCK_REASON_KEY),
			durationMinutes: normalizeDurationMinutes(start, end, defaultBlockMinutes),
			dueAt: metadataString(metadata, TIME_BLOCK_DUE_AT_KEY)
		});
	}
	return blocks;
}

export function computeRescheduleSuggestions(
	existingBlocks: ExistingTimeBlock[],
	busyWindows: TimeBlockBusyWindow[],
	options: RescheduleOptions
): TimeBlockRescheduleSuggestion[] {
	const candidates = existingBlocks.map((block) => ({
		id: block.eventId,
		title: block.taskTitle,
		score: block.score,
		estimateMinutes: block.durationMinutes,
		dueAt: block.dueAt,
		reason: block.reason
	}));

	const suggestionByEvent = new Map(
		existingBlocks.map((block) => [block.eventId, block] as const)
	);
	const planned = computeTimeBlockSuggestions(candidates, busyWindows, {
		date: options.date,
		limit: options.limit ?? existingBlocks.length,
		workdayStartHour: options.workdayStartHour ?? 9,
		workdayEndHour: options.workdayEndHour ?? 18,
		defaultBlockMinutes: options.defaultBlockMinutes ?? 45
	});

	return planned
		.map((entry) => {
			const source = suggestionByEvent.get(entry.taskId);
			if (!source) return null;
			return {
				eventId: source.eventId,
				taskId: source.taskId,
				taskTitle: source.taskTitle,
				score: entry.score,
				reason: entry.reason,
				previousStart: source.start,
				previousEnd: source.end,
				start: entry.start,
				end: entry.end,
				durationMinutes: entry.durationMinutes,
				dueAt: source.dueAt
			} satisfies TimeBlockRescheduleSuggestion;
		})
		.filter((entry): entry is TimeBlockRescheduleSuggestion => Boolean(entry))
		.filter((entry) => entry.start !== entry.previousStart || entry.end !== entry.previousEnd);
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
			include_tasks: false,
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

	return computeTimeBlockSuggestions(candidates, calendarBusyWindows(calendar.items), {
		date: params.date,
		limit: params.limit ?? 6,
		workdayStartHour: params.workdayStartHour ?? 9,
		workdayEndHour: params.workdayEndHour ?? 18,
		defaultBlockMinutes: params.defaultBlockMinutes ?? 45
	});
}

export async function suggestRescheduledTimeBlocks(
	params: SuggestTimeBlocksParams
): Promise<TimeBlockRescheduleSuggestion[]> {
	const calendar = await getCalendarItems({
		date: params.date,
		view: 'day',
		include_tasks: false,
		namespace: params.namespace ?? undefined
	});
	const defaultBlockMinutes = params.defaultBlockMinutes ?? 45;
	const existingBlocks = buildExistingTimeBlocks(calendar.items, defaultBlockMinutes);
	if (existingBlocks.length === 0) {
		return [];
	}
	const movableEventIds = new Set(existingBlocks.map((block) => block.eventId));
	const fixedBusy = calendar
		.items
		.filter((item) => !movableEventIds.has(item.id));

	return computeRescheduleSuggestions(existingBlocks, calendarBusyWindows(fixedBusy), {
		date: params.date,
		limit: params.limit ?? existingBlocks.length,
		workdayStartHour: params.workdayStartHour ?? 9,
		workdayEndHour: params.workdayEndHour ?? 18,
		defaultBlockMinutes
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
				[TIME_BLOCK_TASK_ID_KEY]: suggestion.taskId,
				[TIME_BLOCK_TASK_TITLE_KEY]: suggestion.taskTitle,
				[TIME_BLOCK_SCORE_KEY]: suggestion.score,
				[TIME_BLOCK_REASON_KEY]: suggestion.reason ?? null,
				[TIME_BLOCK_DUE_AT_KEY]: suggestion.dueAt ?? null
			}
		});
		ids.push(created.id);

		// Link the generated time block back to its source task.
		try {
			await addRelationship(created.id, suggestion.taskId, 'references');
		} catch (err) {
			console.warn('Failed to create time block relationship', err);
		}
	}
	return { created: ids.length, ids };
}

async function updateCalendarEventWindow(
	eventId: string,
	start: string,
	end: string
): Promise<void> {
	const existing = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${eventId}`);
	await fetchJson<KnowledgeNode>(`/api/v1/nodes/${eventId}`, {
		method: 'PUT',
		body: JSON.stringify({
			...existing,
			metadata: {
				...(existing.metadata ?? {}),
				event_start_at: start,
				event_end_at: end,
				time_block_rescheduled_at: new Date().toISOString(),
				time_block_previous_start_at: metadataString(
					existing.metadata ?? {},
					'event_start_at'
				),
				time_block_previous_end_at: metadataString(
					existing.metadata ?? {},
					'event_end_at'
				)
			}
		})
	});
}

export async function applyRescheduledTimeBlocks(
	suggestions: TimeBlockRescheduleSuggestion[]
): Promise<{ updated: number; ids: string[] }> {
	const ids: string[] = [];
	for (const suggestion of suggestions) {
		await updateCalendarEventWindow(suggestion.eventId, suggestion.start, suggestion.end);
		ids.push(suggestion.eventId);
	}
	return { updated: ids.length, ids };
}
