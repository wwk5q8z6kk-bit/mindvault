import { fetchJson } from './client';
import { createNode, deleteNode, listNodes } from './nodes';
import type { KnowledgeNode, StoreNodeRequest } from './types';
import {
	calculateBestStreak,
	calculateCurrentStreak,
	latestCheckinDate,
	normalizeDateList
} from '$lib/goals/habits';

export type GoalStatus = 'active' | 'on_track' | 'at_risk' | 'paused' | 'completed';
export type HabitFrequency = 'daily' | 'weekdays' | 'weekly';

export interface Goal {
	id: string;
	title: string;
	description: string | null;
	status: GoalStatus;
	progress: number;
	target_date: string | null;
	priority: number;
	tags: string[];
	namespace: string | null;
	created_at: string;
	updated_at: string;
	metadata: Record<string, unknown>;
}

export interface Habit {
	id: string;
	name: string;
	description: string | null;
	frequency: HabitFrequency;
	target_per_period: number;
	enabled: boolean;
	checkins: string[];
	current_streak: number;
	best_streak: number;
	last_checkin_at: string | null;
	tags: string[];
	namespace: string | null;
	created_at: string;
	updated_at: string;
	metadata: Record<string, unknown>;
}

export interface GoalCreatePayload {
	title: string;
	description?: string | null;
	status?: GoalStatus;
	progress?: number;
	target_date?: string | null;
	priority?: number;
	tags?: string[];
	namespace?: string | null;
	metadata?: Record<string, unknown>;
}

export interface GoalPatchPayload {
	title?: string;
	description?: string | null;
	status?: GoalStatus;
	progress?: number;
	target_date?: string | null;
	priority?: number;
	tags?: string[];
	namespace?: string | null;
	metadata?: Record<string, unknown>;
}

export interface HabitCreatePayload {
	name: string;
	description?: string | null;
	frequency?: HabitFrequency;
	target_per_period?: number;
	enabled?: boolean;
	checkins?: string[];
	tags?: string[];
	namespace?: string | null;
	metadata?: Record<string, unknown>;
}

export interface HabitPatchPayload {
	name?: string;
	description?: string | null;
	frequency?: HabitFrequency;
	target_per_period?: number;
	enabled?: boolean;
	checkins?: string[];
	tags?: string[];
	namespace?: string | null;
	metadata?: Record<string, unknown>;
}

const GOAL_KIND = 'project';
const OBJECT_TYPE_KEY = 'mv_object_type';
const OBJECT_TYPE_GOAL = 'goal';
const OBJECT_TYPE_HABIT = 'habit';

const GOAL_STATUS_KEY = 'goal_status';
const GOAL_PROGRESS_KEY = 'goal_progress';
const GOAL_TARGET_DATE_KEY = 'goal_target_date';
const GOAL_PRIORITY_KEY = 'goal_priority';

const HABIT_FREQUENCY_KEY = 'habit_frequency';
const HABIT_TARGET_KEY = 'habit_target_per_period';
const HABIT_ENABLED_KEY = 'habit_enabled';
const HABIT_CHECKINS_KEY = 'habit_checkins';
const HABIT_BEST_STREAK_KEY = 'habit_best_streak';
const HABIT_LAST_CHECKIN_KEY = 'habit_last_checkin_at';

function importanceToPriority(importance: number): number {
	return Math.max(1, Math.min(5, Math.round(importance * 4) + 1));
}

function priorityToImportance(priority: number): number {
	return Math.max(0, Math.min(1, (priority - 1) / 4));
}

function asString(value: unknown): string | null {
	return typeof value === 'string' && value.trim().length > 0 ? value : null;
}

function asNumber(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function asBoolean(value: unknown, fallback: boolean): boolean {
	return typeof value === 'boolean' ? value : fallback;
}

function asDateList(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	return normalizeDateList(value.filter((item): item is string => typeof item === 'string'));
}

function mapGoal(node: KnowledgeNode): Goal {
	const meta = node.metadata;
	return {
		id: node.id,
		title: node.title,
		description: node.content ?? null,
		status: (asString(meta[GOAL_STATUS_KEY]) as GoalStatus | null) ?? 'active',
		progress: Math.max(0, Math.min(100, asNumber(meta[GOAL_PROGRESS_KEY]) ?? 0)),
		target_date: asString(meta[GOAL_TARGET_DATE_KEY]),
		priority: Math.max(1, Math.min(5, asNumber(meta[GOAL_PRIORITY_KEY]) ?? importanceToPriority(node.importance))),
		tags: node.tags,
		namespace: node.namespace ?? null,
		created_at: node.temporal.created_at,
		updated_at: node.temporal.updated_at,
		metadata: meta
	};
}

function mapHabit(node: KnowledgeNode): Habit {
	const meta = node.metadata;
	const checkins = asDateList(meta[HABIT_CHECKINS_KEY]);
	const bestFromMeta = asNumber(meta[HABIT_BEST_STREAK_KEY]) ?? 0;
	const bestStreak = Math.max(bestFromMeta, calculateBestStreak(checkins));

	return {
		id: node.id,
		name: node.title,
		description: node.content ?? null,
		frequency: (asString(meta[HABIT_FREQUENCY_KEY]) as HabitFrequency | null) ?? 'daily',
		target_per_period: Math.max(1, Math.min(31, asNumber(meta[HABIT_TARGET_KEY]) ?? 1)),
		enabled: asBoolean(meta[HABIT_ENABLED_KEY], true),
		checkins,
		current_streak: calculateCurrentStreak(checkins),
		best_streak: bestStreak,
		last_checkin_at: asString(meta[HABIT_LAST_CHECKIN_KEY]) ?? latestCheckinDate(checkins),
		tags: node.tags,
		namespace: node.namespace ?? null,
		created_at: node.temporal.created_at,
		updated_at: node.temporal.updated_at,
		metadata: meta
	};
}

function isGoalNode(node: KnowledgeNode): boolean {
	return node.metadata[OBJECT_TYPE_KEY] === OBJECT_TYPE_GOAL;
}

function isHabitNode(node: KnowledgeNode): boolean {
	return node.metadata[OBJECT_TYPE_KEY] === OBJECT_TYPE_HABIT;
}

export async function listGoals(limit = 300, namespace?: string | null): Promise<Goal[]> {
	const nodes = await listNodes({ kind: GOAL_KIND, limit, namespace: namespace ?? undefined });
	return nodes.filter(isGoalNode).map(mapGoal);
}

export async function listHabits(limit = 300, namespace?: string | null): Promise<Habit[]> {
	const nodes = await listNodes({ kind: GOAL_KIND, limit, namespace: namespace ?? undefined });
	return nodes.filter(isHabitNode).map(mapHabit);
}

export async function createGoal(payload: GoalCreatePayload): Promise<Goal> {
	const metadata: Record<string, unknown> = {
		...(payload.metadata ?? {}),
		[OBJECT_TYPE_KEY]: OBJECT_TYPE_GOAL,
		[GOAL_STATUS_KEY]: payload.status ?? 'active',
		[GOAL_PROGRESS_KEY]: payload.progress ?? 0,
		[GOAL_PRIORITY_KEY]: payload.priority ?? 3
	};
	if (payload.target_date) {
		metadata[GOAL_TARGET_DATE_KEY] = payload.target_date;
	}

	const request: StoreNodeRequest = {
		kind: GOAL_KIND,
		title: payload.title,
		content: payload.description ?? undefined,
		namespace: payload.namespace ?? undefined,
		tags: payload.tags ?? [],
		importance: priorityToImportance(payload.priority ?? 3),
		metadata
	};
	const created = await createNode(request);
	return mapGoal(created);
}

export async function updateGoal(goalId: string, patch: GoalPatchPayload): Promise<Goal> {
	const existing = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${goalId}`);
	const metadata: Record<string, unknown> = {
		...existing.metadata,
		...(patch.metadata ?? {}),
		[OBJECT_TYPE_KEY]: OBJECT_TYPE_GOAL
	};

	if (patch.status !== undefined) metadata[GOAL_STATUS_KEY] = patch.status;
	if (patch.progress !== undefined) metadata[GOAL_PROGRESS_KEY] = Math.max(0, Math.min(100, patch.progress));
	if (patch.target_date !== undefined) metadata[GOAL_TARGET_DATE_KEY] = patch.target_date;
	if (patch.priority !== undefined) metadata[GOAL_PRIORITY_KEY] = patch.priority;

	const updated = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${goalId}`, {
		method: 'PUT',
		body: JSON.stringify({
			...existing,
			title: patch.title ?? existing.title,
			content: patch.description !== undefined ? patch.description : existing.content,
			tags: patch.tags ?? existing.tags,
			namespace: patch.namespace ?? existing.namespace,
			importance:
				patch.priority !== undefined
					? priorityToImportance(patch.priority)
					: existing.importance,
			metadata
		})
	});
	return mapGoal(updated);
}

export async function deleteGoal(goalId: string): Promise<void> {
	await deleteNode(goalId);
}

export async function createHabit(payload: HabitCreatePayload): Promise<Habit> {
	const checkins = normalizeDateList(payload.checkins ?? []);
	const metadata: Record<string, unknown> = {
		...(payload.metadata ?? {}),
		[OBJECT_TYPE_KEY]: OBJECT_TYPE_HABIT,
		[HABIT_FREQUENCY_KEY]: payload.frequency ?? 'daily',
		[HABIT_TARGET_KEY]: payload.target_per_period ?? 1,
		[HABIT_ENABLED_KEY]: payload.enabled ?? true,
		[HABIT_CHECKINS_KEY]: checkins,
		[HABIT_BEST_STREAK_KEY]: calculateBestStreak(checkins),
		[HABIT_LAST_CHECKIN_KEY]: latestCheckinDate(checkins)
	};

	const request: StoreNodeRequest = {
		kind: GOAL_KIND,
		title: payload.name,
		content: payload.description ?? undefined,
		namespace: payload.namespace ?? undefined,
		tags: payload.tags ?? [],
		importance: 0.5,
		metadata
	};
	const created = await createNode(request);
	return mapHabit(created);
}

export async function updateHabit(habitId: string, patch: HabitPatchPayload): Promise<Habit> {
	const existing = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${habitId}`);
	const metadata: Record<string, unknown> = {
		...existing.metadata,
		...(patch.metadata ?? {}),
		[OBJECT_TYPE_KEY]: OBJECT_TYPE_HABIT
	};

	if (patch.frequency !== undefined) metadata[HABIT_FREQUENCY_KEY] = patch.frequency;
	if (patch.target_per_period !== undefined) metadata[HABIT_TARGET_KEY] = patch.target_per_period;
	if (patch.enabled !== undefined) metadata[HABIT_ENABLED_KEY] = patch.enabled;
	if (patch.checkins !== undefined) {
		const checkins = normalizeDateList(patch.checkins);
		metadata[HABIT_CHECKINS_KEY] = checkins;
		metadata[HABIT_BEST_STREAK_KEY] = Math.max(
			asNumber(metadata[HABIT_BEST_STREAK_KEY]) ?? 0,
			calculateBestStreak(checkins)
		);
		metadata[HABIT_LAST_CHECKIN_KEY] = latestCheckinDate(checkins);
	}

	const updated = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${habitId}`, {
		method: 'PUT',
		body: JSON.stringify({
			...existing,
			title: patch.name ?? existing.title,
			content: patch.description !== undefined ? patch.description : existing.content,
			tags: patch.tags ?? existing.tags,
			namespace: patch.namespace ?? existing.namespace,
			metadata
		})
	});
	return mapHabit(updated);
}

export async function deleteHabit(habitId: string): Promise<void> {
	await deleteNode(habitId);
}
