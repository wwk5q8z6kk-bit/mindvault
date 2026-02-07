import type { PrioritizedTaskItem } from '$lib/api/ai';
import type { TaskPatchPayload, TaskStatus } from '$lib/api/tasks';
import type { TaskRecord } from '$lib/db';

export interface InboxTriageSuggestion {
	taskId: string;
	score: number;
	rank: number;
	reason: string | null;
	suggestedPriority: number;
	suggestedStatus: TaskStatus;
}

export function scoreToTaskPriority(score: number): number {
	const normalized = Number.isFinite(score) ? score : 0;
	if (normalized >= 0.85) return 1;
	if (normalized >= 0.65) return 2;
	if (normalized >= 0.45) return 3;
	if (normalized >= 0.25) return 4;
	return 5;
}

export function recommendInboxStatus(rank: number, priority: number): TaskStatus {
	if (rank <= 1 && priority <= 2) return 'in_progress';
	if (rank <= 5 || priority <= 2) return 'planned';
	return 'inbox';
}

export function buildInboxTriageSuggestions(items: PrioritizedTaskItem[]): Map<string, InboxTriageSuggestion> {
	const suggestions = new Map<string, InboxTriageSuggestion>();
	for (const item of items) {
		const priority = scoreToTaskPriority(item.score);
		suggestions.set(item.task.id, {
			taskId: item.task.id,
			score: item.score,
			rank: item.rank,
			reason: item.reason ?? null,
			suggestedPriority: priority,
			suggestedStatus: recommendInboxStatus(item.rank, priority)
		});
	}
	return suggestions;
}

export function buildInboxTriagePatch(
	task: Pick<TaskRecord, 'status' | 'priority'>,
	suggestion: InboxTriageSuggestion
): TaskPatchPayload | null {
	const patch: TaskPatchPayload = {};
	if (task.priority !== suggestion.suggestedPriority) {
		patch.priority = suggestion.suggestedPriority;
	}
	// Keep inbox untouched when recommendation is to keep triaging later.
	if (task.status !== suggestion.suggestedStatus && suggestion.suggestedStatus !== 'inbox') {
		patch.status = suggestion.suggestedStatus;
	}
	return Object.keys(patch).length ? patch : null;
}
