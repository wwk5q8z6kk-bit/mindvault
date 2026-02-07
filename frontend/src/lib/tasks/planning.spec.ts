import { describe, expect, it } from 'vitest';
import { buildPlanningSnapshot, proposeDependencyDueDates } from './planning';

describe('task planning', () => {
	it('marks blocked and ready tasks from dependencies', () => {
		const tasks = [
			{
				id: 'a',
				title: 'A',
				status: 'in_progress',
				priority: 2,
				estimate_min: 60,
				due_at: null,
				dependencies: []
			},
			{
				id: 'b',
				title: 'B',
				status: 'planned',
				priority: 3,
				estimate_min: 30,
				due_at: null,
				dependencies: ['a']
			}
		] as any[];
		const snapshot = buildPlanningSnapshot(tasks);
		expect(snapshot.readyTaskIds).toContain('a');
		expect(snapshot.blockedTaskIds).toContain('b');
		expect(snapshot.blockedReasonByTask['b']).toContain('Waiting on');
	});

	it('detects dependency cycles', () => {
		const tasks = [
			{ id: 'a', title: 'A', status: 'planned', priority: 3, estimate_min: 30, due_at: null, dependencies: ['c'] },
			{ id: 'b', title: 'B', status: 'planned', priority: 3, estimate_min: 30, due_at: null, dependencies: ['a'] },
			{ id: 'c', title: 'C', status: 'planned', priority: 3, estimate_min: 30, due_at: null, dependencies: ['b'] }
		] as any[];
		const snapshot = buildPlanningSnapshot(tasks);
		expect(snapshot.cycles.length).toBeGreaterThan(0);
		expect(proposeDependencyDueDates(tasks, new Date('2026-02-06T00:00:00Z'))).toEqual({});
	});

	it('builds critical path and proposes due dates', () => {
		const now = new Date('2026-02-06T00:00:00Z');
		const tasks = [
			{
				id: 'setup',
				title: 'Setup',
				status: 'planned',
				priority: 3,
				estimate_min: 60,
				due_at: null,
				dependencies: []
			},
			{
				id: 'build',
				title: 'Build',
				status: 'planned',
				priority: 3,
				estimate_min: 120,
				due_at: null,
				dependencies: ['setup']
			},
			{
				id: 'review',
				title: 'Review',
				status: 'planned',
				priority: 3,
				estimate_min: 30,
				due_at: null,
				dependencies: ['build']
			}
		] as any[];
		const snapshot = buildPlanningSnapshot(tasks);
		expect(snapshot.criticalPathTaskIds).toEqual(['setup', 'build', 'review']);
		expect(snapshot.criticalPathMinutes).toBe(210);

		const updates = proposeDependencyDueDates(tasks, now);
		expect(Object.keys(updates)).toEqual(['setup', 'build', 'review']);
		expect(new Date(updates['setup']).getTime()).toBeGreaterThan(now.getTime());
		expect(new Date(updates['review']).getTime()).toBeGreaterThan(new Date(updates['build']).getTime());
	});
});
