import { fetchJson } from './client';
import type { KnowledgeNode } from './types';
import { nodeToTask, taskToNodePayload, taskPatchToNode } from './mappers';

export type TaskStatus = 'inbox' | 'planned' | 'in_progress' | 'waiting' | 'review' | 'done';

export interface Task {
	id: string;
	title: string;
	description?: string | null;
	status: TaskStatus;
	priority: number;
	due_at?: string | null;
	estimate_min?: number | null;
	labels: string[];
	assignee?: string | null;
	dependencies: string[];
	recurrence?: string | null;
	created_at: string;
	updated_at: string;
	completed_at?: string | null;
	metadata: Record<string, unknown>;
}

export interface TaskCreatePayload {
	title: string;
	description?: string | null;
	status?: TaskStatus;
	priority?: number;
	due_at?: string | null;
	estimate_min?: number | null;
	labels?: string[];
	assignee?: string | null;
	dependencies?: string[];
	recurrence?: string | null;
	metadata?: Record<string, unknown>;
}

export interface TaskPatchPayload {
	title?: string;
	description?: string | null;
	status?: TaskStatus;
	priority?: number;
	due_at?: string | null;
	estimate_min?: number | null;
	labels?: string[];
	assignee?: string | null;
	dependencies?: string[];
	recurrence?: string | null;
	metadata?: Record<string, unknown>;
}

export async function listTasks(status?: TaskStatus, namespace?: string | null): Promise<Task[]> {
	const params = new URLSearchParams({ kind: 'task', limit: '200' });
	if (namespace) params.set('namespace', namespace);
	const nodes = await fetchJson<KnowledgeNode[]>(`/api/v1/nodes?${params.toString()}`);
	let tasks = nodes.map(nodeToTask);
	if (status) {
		tasks = tasks.filter((t) => t.status === status);
	}
	return tasks;
}

export async function getTask(taskId: string): Promise<Task | null> {
	const node = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${taskId}`);
	return nodeToTask(node);
}

export async function createTask(payload: TaskCreatePayload): Promise<Task> {
	const body = taskToNodePayload(payload);
	const node = await fetchJson<KnowledgeNode>('/api/v1/nodes', {
		method: 'POST',
		body: JSON.stringify(body)
	});
	return nodeToTask(node);
}

export async function updateTask(taskId: string, payload: TaskPatchPayload): Promise<Task | null> {
	const existing = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${taskId}`);
	const merged = taskPatchToNode(existing, payload);
	const updated = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${taskId}`, {
		method: 'PUT',
		body: JSON.stringify(merged)
	});
	return nodeToTask(updated);
}

export async function deleteTask(taskId: string): Promise<void> {
	await fetchJson<{ deleted: boolean }>(`/api/v1/nodes/${taskId}`, { method: 'DELETE' });
}

// ============================================================================
// Task-Specific Actions (dedicated backend endpoints)
// ============================================================================

export interface DueTasksParams {
	/** Filter tasks due before this date (ISO) */
	due_before?: string;
	/** Filter tasks due after this date (ISO) */
	due_after?: string;
	/** Include overdue tasks */
	include_overdue?: boolean;
	/** Include completed tasks */
	include_completed?: boolean;
	/** Limit results */
	limit?: number;
}

export interface DueTasksResponse {
	tasks: Task[];
	overdue_count: number;
	due_today_count: number;
	due_this_week_count: number;
}

/**
 * Get tasks with due dates, sorted by urgency.
 */
export async function listDueTasks(params: DueTasksParams = {}): Promise<DueTasksResponse> {
	const searchParams = new URLSearchParams();
	if (params.due_before) searchParams.set('due_before', params.due_before);
	if (params.due_after) searchParams.set('due_after', params.due_after);
	if (params.include_overdue !== undefined)
		searchParams.set('include_overdue', String(params.include_overdue));
	if (params.include_completed !== undefined)
		searchParams.set('include_completed', String(params.include_completed));
	if (params.limit !== undefined) searchParams.set('limit', String(params.limit));

	const query = searchParams.toString();
	const path = `/api/v1/tasks/due${query ? `?${query}` : ''}`;

	const response = await fetchJson<{ tasks: KnowledgeNode[]; overdue_count: number; due_today_count: number; due_this_week_count: number }>(path);

	return {
		tasks: response.tasks.map(nodeToTask),
		overdue_count: response.overdue_count,
		due_today_count: response.due_today_count,
		due_this_week_count: response.due_this_week_count
	};
}

/**
 * Mark a task as complete using the dedicated endpoint.
 * More efficient than updateTask for simple completion.
 */
export async function completeTask(taskId: string): Promise<Task> {
	const node = await fetchJson<KnowledgeNode>(`/api/v1/tasks/${taskId}/complete`, {
		method: 'POST'
	});
	return nodeToTask(node);
}

/**
 * Reopen a completed task.
 */
export async function reopenTask(taskId: string): Promise<Task> {
	const node = await fetchJson<KnowledgeNode>(`/api/v1/tasks/${taskId}/reopen`, {
		method: 'POST'
	});
	return nodeToTask(node);
}

export interface SnoozeTaskParams {
	/** Snooze until this datetime (ISO) */
	snooze_until: string;
	/** Optional reason for snoozing */
	reason?: string;
}

/**
 * Snooze task reminders until a specific time.
 */
export async function snoozeTask(taskId: string, params: SnoozeTaskParams): Promise<Task> {
	const node = await fetchJson<KnowledgeNode>(`/api/v1/tasks/${taskId}/snooze`, {
		method: 'POST',
		body: JSON.stringify(params)
	});
	return nodeToTask(node);
}

/**
 * Snooze until tomorrow morning (9am local).
 */
export async function snoozeTillTomorrow(taskId: string): Promise<Task> {
	const tomorrow = new Date();
	tomorrow.setDate(tomorrow.getDate() + 1);
	tomorrow.setHours(9, 0, 0, 0);
	return snoozeTask(taskId, { snooze_until: tomorrow.toISOString() });
}

/**
 * Snooze until next week (Monday 9am local).
 */
export async function snoozeTillNextWeek(taskId: string): Promise<Task> {
	const nextWeek = new Date();
	const daysUntilMonday = (8 - nextWeek.getDay()) % 7 || 7;
	nextWeek.setDate(nextWeek.getDate() + daysUntilMonday);
	nextWeek.setHours(9, 0, 0, 0);
	return snoozeTask(taskId, { snooze_until: nextWeek.toISOString() });
}

/** Client-side quick-add: parse text for title/priority/date, then create a task. */
export async function quickAddTask(text: string): Promise<{ task: Task }> {
	const parsed = parseQuickAdd(text);
	const task = await createTask(parsed);
	return { task };
}

/** Preview parser result type */
export interface QuickAddPreview {
	title: string;
	priority: number;
	due_at: string | null;
	labels: string[];
	estimate_min: number | null;
	assignee: string | null;
	recurrence: string | null;
}

/** Parse quick add text and return preview (for UI) */
export function parseQuickAddPreview(text: string): QuickAddPreview {
	return parseQuickAdd(text) as QuickAddPreview;
}

function parseQuickAdd(text: string): TaskCreatePayload {
	let title = text;
	let priority = 3;
	let due_at: string | null = null;
	let estimate_min: number | null = null;
	let assignee: string | null = null;
	let recurrence: string | null = null;
	const labels: string[] = [];

	// Extract priority markers: !1 .. !5 or p1 .. p5
	const prioMatch = title.match(/[!p]([1-5])(?:\s|$)/i);
	if (prioMatch) {
		priority = parseInt(prioMatch[1]);
		title = title.replace(prioMatch[0], '').trim();
	}

	// Extract #tags
	const tagRegex = /#(\w[\w-]*)/g;
	let tagMatch: RegExpExecArray | null;
	while ((tagMatch = tagRegex.exec(title)) !== null) {
		labels.push(tagMatch[1]);
	}
	title = title.replace(tagRegex, '').trim();

	// Extract @assignee
	const assigneeMatch = title.match(/@(\w+)/);
	if (assigneeMatch) {
		assignee = assigneeMatch[1];
		title = title.replace(assigneeMatch[0], '').trim();
	}

	// Extract time estimates: 30m, 1h, 1.5h, 2h30m
	const estimateMatch = title.match(/(\d+(?:\.\d+)?)\s*([hm])/i);
	if (estimateMatch) {
		const value = parseFloat(estimateMatch[1]);
		const unit = estimateMatch[2].toLowerCase();
		estimate_min = unit === 'h' ? Math.round(value * 60) : value;
		title = title.replace(estimateMatch[0], '').trim();
	}

	// Extract recurrence: "every day", "every week", "every month"
	const recurrenceMatch = title.match(/every\s+(day|week|month|weekday)/i);
	if (recurrenceMatch) {
		recurrence = recurrenceMatch[1].toLowerCase();
		title = title.replace(recurrenceMatch[0], '').trim();
	}

	// Extract natural language dates: "tomorrow", "next monday", "5pm", etc.
	const datePatterns = [
		{ pattern: /\btomorrow\b/i, compute: () => { const d = new Date(); d.setDate(d.getDate() + 1); return d; } },
		{ pattern: /\btoday\b/i, compute: () => new Date() },
		{ pattern: /\bnext\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b/i, compute: (m: RegExpMatchArray) => {
			const days = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];
			const targetDay = days.indexOf(m[1].toLowerCase());
			const d = new Date();
			const currentDay = d.getDay();
			const daysAhead = ((targetDay - currentDay + 7) % 7) || 7;
			d.setDate(d.getDate() + daysAhead);
			return d;
		}},
		{ pattern: /(\d{1,2})(?::(\d{2}))?\s*(am|pm)/i, compute: (m: RegExpMatchArray) => {
			const d = new Date();
			let hours = parseInt(m[1]);
			const minutes = m[2] ? parseInt(m[2]) : 0;
			const meridiem = m[3].toLowerCase();
			if (meridiem === 'pm' && hours < 12) hours += 12;
			if (meridiem === 'am' && hours === 12) hours = 0;
			d.setHours(hours, minutes, 0, 0);
			return d;
		}}
	];

	for (const { pattern, compute } of datePatterns) {
		const match = title.match(pattern);
		if (match) {
			const date = compute(match);
			due_at = date.toISOString();
			title = title.replace(match[0], '').trim();
			break;
		}
	}

	// Extract "due:YYYY-MM-DD" or "due:today" / "due:tomorrow" (explicit format)
	const dueMatch = title.match(/due:(\S+)/i);
	if (dueMatch) {
		const raw = dueMatch[1].toLowerCase();
		if (raw === 'today') {
			due_at = new Date().toISOString().slice(0, 10);
		} else if (raw === 'tomorrow') {
			const d = new Date();
			d.setDate(d.getDate() + 1);
			due_at = d.toISOString().slice(0, 10);
		} else {
			due_at = raw;
		}
		title = title.replace(dueMatch[0], '').trim();
	}

	// Clean up extra whitespace
	title = title.replace(/\s+/g, ' ').trim();

	return { title, priority, due_at, labels, estimate_min, assignee, recurrence };
}
