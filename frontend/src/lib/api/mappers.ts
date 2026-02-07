import type { Task, TaskStatus, TaskCreatePayload, TaskPatchPayload } from './tasks';
import type { Note } from './notes';
import type { KnowledgeNode, StoreNodeRequest } from './types';
import {
	TASK_DUE_AT,
	TASK_COMPLETED,
	TASK_COMPLETED_AT,
	TASK_STATUS,
	TASK_RECURRENCE,
	TASK_ESTIMATE_MIN,
	TASK_ASSIGNEE,
	TASK_DEPENDENCIES,
	NOTE_PINNED
} from './types';

// ---- helpers ----

function importanceToTaskPriority(importance: number): number {
	// importance is 0..1, priority is 1..5
	return Math.max(1, Math.min(5, Math.round(importance * 4) + 1));
}

function taskPriorityToImportance(priority: number): number {
	return Math.max(0, Math.min(1, (priority - 1) / 4));
}

function resolveTaskStatus(node: KnowledgeNode): TaskStatus {
	if (node.metadata[TASK_COMPLETED] === true) return 'done';
	const raw = node.metadata[TASK_STATUS] as string | undefined;
	const valid: TaskStatus[] = ['inbox', 'planned', 'in_progress', 'waiting', 'review', 'done'];
	if (raw && valid.includes(raw as TaskStatus)) return raw as TaskStatus;
	return 'inbox';
}

// ---- Node → Task ----

export function nodeToTask(node: KnowledgeNode): Task {
	const meta = node.metadata;
	return {
		id: node.id,
		title: node.title,
		description: node.content ?? null,
		status: resolveTaskStatus(node),
		priority: importanceToTaskPriority(node.importance),
		due_at: (meta[TASK_DUE_AT] as string) ?? null,
		estimate_min: (meta[TASK_ESTIMATE_MIN] as number) ?? null,
		labels: node.tags,
		assignee: (meta[TASK_ASSIGNEE] as string) ?? null,
		dependencies: (meta[TASK_DEPENDENCIES] as string[]) ?? [],
		recurrence: (meta[TASK_RECURRENCE] as string) ?? null,
		created_at: node.temporal.created_at,
		updated_at: node.temporal.updated_at,
		completed_at: (meta[TASK_COMPLETED_AT] as string) ?? null,
		metadata: meta
	};
}

// ---- Task → Node (create) ----

export function taskToNodePayload(task: TaskCreatePayload): StoreNodeRequest {
	const metadata: Record<string, unknown> = { ...(task.metadata ?? {}) };
	if (task.status) metadata[TASK_STATUS] = task.status;
	if (task.due_at) metadata[TASK_DUE_AT] = task.due_at;
	if (task.recurrence) metadata[TASK_RECURRENCE] = task.recurrence;
	if (task.estimate_min != null) metadata[TASK_ESTIMATE_MIN] = task.estimate_min;
	if (task.assignee) metadata[TASK_ASSIGNEE] = task.assignee;
	if (task.dependencies?.length) metadata[TASK_DEPENDENCIES] = task.dependencies;
	if (task.status === 'done') metadata[TASK_COMPLETED] = true;

	return {
		kind: 'task',
		title: task.title,
		content: task.description ?? undefined,
		tags: task.labels,
		importance: taskPriorityToImportance(task.priority ?? 3),
		metadata
	};
}

// ---- Task patch → Node (update) ----

export function taskPatchToNode(existing: KnowledgeNode, patch: TaskPatchPayload): KnowledgeNode {
	const meta = { ...existing.metadata };
	if (patch.metadata) {
		Object.assign(meta, patch.metadata);
	}

	if (patch.status !== undefined) {
		meta[TASK_STATUS] = patch.status;
		if (patch.status === 'done') {
			meta[TASK_COMPLETED] = true;
			meta[TASK_COMPLETED_AT] = new Date().toISOString();
		} else {
			meta[TASK_COMPLETED] = false;
			delete meta[TASK_COMPLETED_AT];
		}
	}
	if (patch.due_at !== undefined) meta[TASK_DUE_AT] = patch.due_at;
	if (patch.recurrence !== undefined) meta[TASK_RECURRENCE] = patch.recurrence;
	if (patch.estimate_min !== undefined) meta[TASK_ESTIMATE_MIN] = patch.estimate_min;
	if (patch.assignee !== undefined) meta[TASK_ASSIGNEE] = patch.assignee;
	if (patch.dependencies !== undefined) meta[TASK_DEPENDENCIES] = patch.dependencies;

	return {
		...existing,
		title: patch.title ?? existing.title,
		content: patch.description !== undefined ? patch.description : existing.content,
		tags: patch.labels ?? existing.tags,
		importance:
			patch.priority !== undefined
				? taskPriorityToImportance(patch.priority)
				: existing.importance,
		metadata: meta
	};
}

// ---- Node → Note ----

export function nodeToNote(node: KnowledgeNode): Note {
	return {
		id: node.id,
		markdown: node.content ?? '',
		title: node.title || null,
		namespace: node.namespace ?? null,
		tags: node.tags,
		backlinks: [],
		pinned: node.metadata[NOTE_PINNED] === true,
		created_at: node.temporal.created_at,
		updated_at: node.temporal.updated_at,
		metadata: node.metadata
	};
}

// ---- Note → Node (create) ----

export function noteToNodePayload(markdown: string, title?: string): StoreNodeRequest {
	return {
		kind: 'fact',
		title: title || extractTitleFromMarkdown(markdown),
		content: markdown
	};
}

// ---- Note patch → Node (update) ----

export function notePatchToNode(
	existing: KnowledgeNode,
	patch: { markdown?: string; title?: string; pinned?: boolean }
): KnowledgeNode {
	const meta = { ...existing.metadata };
	if (patch.pinned !== undefined) meta[NOTE_PINNED] = patch.pinned;

	return {
		...existing,
		title: patch.title ?? existing.title,
		content: patch.markdown !== undefined ? patch.markdown : existing.content,
		metadata: meta
	};
}

// ---- Utility ----

function extractTitleFromMarkdown(md: string): string {
	const firstLine = md.split('\n')[0]?.trim() ?? '';
	const heading = firstLine.match(/^#+\s+(.*)$/);
	if (heading) return heading[1];
	return firstLine.slice(0, 80) || 'Untitled';
}
