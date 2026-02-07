import { derived, get, writable } from 'svelte/store';
import type {
	GoalCreatePayload,
	GoalPatchPayload,
	HabitCreatePayload,
	HabitPatchPayload
} from '$lib/api/goals';
import { createGoal, createHabit, deleteGoal, deleteHabit, listGoals, listHabits, updateGoal, updateHabit } from '$lib/api/goals';
import { db, type GoalRecord, type HabitRecord } from '$lib/db';
import { activeNamespace } from '$lib/stores/namespace';
import {
	calculateBestStreak,
	calculateCurrentStreak,
	latestCheckinDate,
	toDateOnly,
	toggleCheckinDate
} from '$lib/goals/habits';

export const goalsStore = writable<GoalRecord[]>([]);
export const habitsStore = writable<HabitRecord[]>([]);

export const goalsFilter = writable<{
	query: string;
	status: GoalRecord['status'] | 'all';
}>({
	query: '',
	status: 'all'
});

export const filteredGoals = derived([goalsStore, goalsFilter], ([$goals, $filter]) => {
	const query = $filter.query.trim().toLowerCase();
	return [...$goals]
		.filter((goal) => ($filter.status === 'all' ? true : goal.status === $filter.status))
		.filter((goal) => {
			if (!query) return true;
			const haystack = [goal.title, goal.description ?? '', ...goal.tags].join(' ').toLowerCase();
			return haystack.includes(query);
		})
		.sort((a, b) => {
			const aDate = a.target_date ? Date.parse(a.target_date) : Number.POSITIVE_INFINITY;
			const bDate = b.target_date ? Date.parse(b.target_date) : Number.POSITIVE_INFINITY;
			if (aDate !== bDate) return aDate - bDate;
			return b.updated_at.localeCompare(a.updated_at);
		});
});

export const activeHabits = derived(habitsStore, ($habits) =>
	[...$habits]
		.filter((habit) => habit.enabled)
		.sort((a, b) => {
			if (a.current_streak !== b.current_streak) return b.current_streak - a.current_streak;
			return a.name.localeCompare(b.name);
		})
);

export async function loadGoalsAndHabits(): Promise<void> {
	try {
		const namespace = get(activeNamespace);
		const [goals, habits] = await Promise.all([listGoals(300, namespace), listHabits(300, namespace)]);
		await db.goals.clear();
		await db.habits.clear();
		if (goals.length > 0) await db.goals.bulkPut(goals);
		if (habits.length > 0) await db.habits.bulkPut(habits);
		goalsStore.set(goals);
		habitsStore.set(habits);
	} catch {
		const [goals, habits] = await Promise.all([db.goals.toArray(), db.habits.toArray()]);
		goalsStore.set(goals);
		habitsStore.set(habits);
	}
}

export async function createGoalOptimistic(payload: GoalCreatePayload): Promise<GoalRecord> {
	const created = await createGoal({
		...payload,
		namespace: payload.namespace ?? get(activeNamespace)
	});
	await db.goals.put(created);
	goalsStore.update((items) => [created, ...items]);
	return created;
}

export async function updateGoalOptimistic(goalId: string, payload: GoalPatchPayload): Promise<void> {
	const previous = get(goalsStore);
	goalsStore.update((items) =>
		items.map((goal) =>
			goal.id === goalId ? { ...goal, ...payload, updated_at: new Date().toISOString() } : goal
		)
	);
	try {
		const updated = await updateGoal(goalId, payload);
		await db.goals.put(updated);
		goalsStore.update((items) => items.map((goal) => (goal.id === goalId ? updated : goal)));
	} catch {
		goalsStore.set(previous);
		throw new Error('Failed to update goal');
	}
}

export async function deleteGoalOptimistic(goalId: string): Promise<void> {
	const previous = get(goalsStore);
	goalsStore.update((items) => items.filter((goal) => goal.id !== goalId));
	await db.goals.delete(goalId);
	try {
		await deleteGoal(goalId);
	} catch {
		goalsStore.set(previous);
		throw new Error('Failed to delete goal');
	}
}

export async function createHabitOptimistic(payload: HabitCreatePayload): Promise<HabitRecord> {
	const created = await createHabit({
		...payload,
		namespace: payload.namespace ?? get(activeNamespace)
	});
	await db.habits.put(created);
	habitsStore.update((items) => [created, ...items]);
	return created;
}

export async function updateHabitOptimistic(habitId: string, payload: HabitPatchPayload): Promise<void> {
	const previous = get(habitsStore);
	habitsStore.update((items) =>
		items.map((habit) =>
			habit.id === habitId
				? {
						...habit,
						...payload,
						updated_at: new Date().toISOString(),
						current_streak:
							payload.checkins !== undefined
								? calculateCurrentStreak(payload.checkins)
								: habit.current_streak,
						best_streak:
							payload.checkins !== undefined
								? Math.max(habit.best_streak, calculateBestStreak(payload.checkins))
								: habit.best_streak,
						last_checkin_at:
							payload.checkins !== undefined
								? latestCheckinDate(payload.checkins)
								: habit.last_checkin_at
					}
				: habit
		)
	);
	try {
		const updated = await updateHabit(habitId, payload);
		await db.habits.put(updated);
		habitsStore.update((items) => items.map((habit) => (habit.id === habitId ? updated : habit)));
	} catch {
		habitsStore.set(previous);
		throw new Error('Failed to update habit');
	}
}

export async function deleteHabitOptimistic(habitId: string): Promise<void> {
	const previous = get(habitsStore);
	habitsStore.update((items) => items.filter((habit) => habit.id !== habitId));
	await db.habits.delete(habitId);
	try {
		await deleteHabit(habitId);
	} catch {
		habitsStore.set(previous);
		throw new Error('Failed to delete habit');
	}
}

export async function toggleHabitCompletionOptimistic(
	habitId: string,
	date = toDateOnly(new Date())
): Promise<void> {
	const habit = get(habitsStore).find((item) => item.id === habitId);
	if (!habit) return;

	const checkins = toggleCheckinDate(habit.checkins ?? [], date);
	const lastCheckin = latestCheckinDate(checkins);
	await updateHabitOptimistic(habitId, {
		checkins,
		metadata: {
			...(habit.metadata ?? {}),
			habit_last_checkin_at: lastCheckin
		}
	});
}
