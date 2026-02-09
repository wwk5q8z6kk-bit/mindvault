import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { goalsStore, habitsStore, goalsFilter, filteredGoals, activeHabits } from './goals';
import type { GoalRecord, HabitRecord } from '$lib/db';

function makeGoal(overrides: Partial<GoalRecord> = {}): GoalRecord {
	return {
		id: crypto.randomUUID(),
		title: 'Test Goal',
		description: null,
		status: 'active',
		progress: 0,
		target_date: null,
		priority: 3,
		tags: [],
		namespace: 'default',
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString(),
		metadata: {},
		...overrides
	};
}

function makeHabit(overrides: Partial<HabitRecord> = {}): HabitRecord {
	return {
		id: crypto.randomUUID(),
		name: 'Test Habit',
		description: null,
		frequency: 'daily',
		target_per_period: 1,
		enabled: true,
		checkins: [],
		current_streak: 0,
		best_streak: 0,
		last_checkin_at: null,
		tags: [],
		namespace: 'default',
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString(),
		metadata: {},
		...overrides
	};
}

beforeEach(() => {
	goalsStore.set([]);
	habitsStore.set([]);
	goalsFilter.set({ query: '', status: 'all' });
});

describe('goalsStore', () => {
	it('starts empty', () => {
		expect(get(goalsStore)).toEqual([]);
	});

	it('holds goals when set', () => {
		const goal = makeGoal();
		goalsStore.set([goal]);
		expect(get(goalsStore)).toHaveLength(1);
	});
});

describe('filteredGoals', () => {
	it('returns all goals with default filter', () => {
		goalsStore.set([makeGoal({ title: 'A' }), makeGoal({ title: 'B' })]);
		expect(get(filteredGoals)).toHaveLength(2);
	});

	it('filters by status', () => {
		goalsStore.set([
			makeGoal({ title: 'Active', status: 'active' }),
			makeGoal({ title: 'Completed', status: 'completed' })
		]);
		goalsFilter.update((f) => ({ ...f, status: 'completed' }));
		expect(get(filteredGoals)).toHaveLength(1);
		expect(get(filteredGoals)[0].title).toBe('Completed');
	});

	it('filters by query text in title', () => {
		goalsStore.set([makeGoal({ title: 'Learn Rust' }), makeGoal({ title: 'Cook dinner' })]);
		goalsFilter.update((f) => ({ ...f, query: 'rust' }));
		expect(get(filteredGoals)).toHaveLength(1);
		expect(get(filteredGoals)[0].title).toBe('Learn Rust');
	});

	it('filters by query text in description', () => {
		goalsStore.set([
			makeGoal({ title: 'Goal 1', description: 'This is about Rust' }),
			makeGoal({ title: 'Goal 2', description: 'This is about cooking' })
		]);
		goalsFilter.update((f) => ({ ...f, query: 'rust' }));
		expect(get(filteredGoals)).toHaveLength(1);
	});

	it('filters by query text in tags', () => {
		goalsStore.set([
			makeGoal({ title: 'Goal 1', tags: ['programming'] }),
			makeGoal({ title: 'Goal 2', tags: ['health'] })
		]);
		goalsFilter.update((f) => ({ ...f, query: 'programming' }));
		expect(get(filteredGoals)).toHaveLength(1);
	});

	it('sorts by target_date ascending, null dates last', () => {
		const sooner = makeGoal({ title: 'Sooner', target_date: '2025-03-01' });
		const later = makeGoal({ title: 'Later', target_date: '2025-09-01' });
		const noDate = makeGoal({ title: 'NoDate', target_date: null });
		goalsStore.set([noDate, later, sooner]);
		const result = get(filteredGoals);
		expect(result[0].title).toBe('Sooner');
		expect(result[1].title).toBe('Later');
		expect(result[2].title).toBe('NoDate');
	});

	it('sorts by updated_at descending for same target_date', () => {
		const older = makeGoal({
			title: 'Older',
			target_date: '2025-06-01',
			updated_at: '2025-01-01T00:00:00Z'
		});
		const newer = makeGoal({
			title: 'Newer',
			target_date: '2025-06-01',
			updated_at: '2025-02-01T00:00:00Z'
		});
		goalsStore.set([older, newer]);
		const result = get(filteredGoals);
		expect(result[0].title).toBe('Newer');
		expect(result[1].title).toBe('Older');
	});
});

describe('habitsStore', () => {
	it('starts empty', () => {
		expect(get(habitsStore)).toEqual([]);
	});
});

describe('activeHabits', () => {
	it('only includes enabled habits', () => {
		habitsStore.set([
			makeHabit({ name: 'Active', enabled: true }),
			makeHabit({ name: 'Disabled', enabled: false })
		]);
		const result = get(activeHabits);
		expect(result).toHaveLength(1);
		expect(result[0].name).toBe('Active');
	});

	it('sorts by current_streak descending', () => {
		habitsStore.set([
			makeHabit({ name: 'Low', current_streak: 2 }),
			makeHabit({ name: 'High', current_streak: 10 })
		]);
		const result = get(activeHabits);
		expect(result[0].name).toBe('High');
		expect(result[1].name).toBe('Low');
	});

	it('sorts by name alphabetically for same streak', () => {
		habitsStore.set([
			makeHabit({ name: 'Yoga', current_streak: 5 }),
			makeHabit({ name: 'Reading', current_streak: 5 })
		]);
		const result = get(activeHabits);
		expect(result[0].name).toBe('Reading');
		expect(result[1].name).toBe('Yoga');
	});
});
