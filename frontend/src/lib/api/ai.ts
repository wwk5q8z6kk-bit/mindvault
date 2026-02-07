import { fetchJson } from './client';
import type { Task, TaskStatus } from './tasks';
import type { KnowledgeNode } from './types';
import { nodeToTask } from './mappers';

export type PrioritizedTaskItem = {
	task: Task;
	score: number;
	rank: number;
	reason?: string | null;
};

export type PrioritizeTasksResponse = {
	items: PrioritizedTaskItem[];
	provider?: string | null;
	generated_at: string;
	meta?: Record<string, unknown>;
};

export type PrioritizeTasksRequest = {
	limit?: number;
	include_done?: boolean;
	statuses?: TaskStatus[];
	persist?: boolean;
	provider?: string;
	namespace?: string;
};

type RawPrioritizedItem = {
	task: KnowledgeNode;
	score: number;
	rank: number;
	reason?: string | null;
};

type RawPrioritizeResponse = {
	items: RawPrioritizedItem[];
	provider?: string | null;
	generated_at: string;
	meta?: Record<string, unknown>;
};

export async function prioritizeTasks(
	payload: PrioritizeTasksRequest = {}
): Promise<PrioritizeTasksResponse> {
	const raw = await fetchJson<RawPrioritizeResponse>('/api/v1/tasks/prioritize', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
	return {
		...raw,
		items: raw.items.map((item) => ({
			...item,
			task: nodeToTask(item.task)
		}))
	};
}
