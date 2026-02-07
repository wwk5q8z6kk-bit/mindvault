import type { Task } from '$lib/api/tasks';

type TaskLike = Pick<Task, 'id' | 'title' | 'status' | 'priority' | 'estimate_min' | 'due_at' | 'dependencies'>;

export type PlanningSnapshot = {
	blockedTaskIds: string[];
	readyTaskIds: string[];
	dependencyIssues: Array<{
		taskId: string;
		missing: string[];
		incomplete: string[];
	}>;
	blockedReasonByTask: Record<string, string>;
	dependentsByTask: Record<string, string[]>;
	cycles: string[][];
	recommendedOrder: string[];
	criticalPathTaskIds: string[];
	criticalPathMinutes: number;
};

function durationMinutes(task: TaskLike): number {
	if (task.estimate_min && task.estimate_min > 0) return task.estimate_min;
	switch (task.priority) {
		case 1:
			return 180;
		case 2:
			return 120;
		case 3:
			return 60;
		case 4:
			return 30;
		default:
			return 15;
	}
}

function formatBlockedReason(missing: string[], incomplete: string[]): string {
	if (missing.length > 0 && incomplete.length > 0) {
		return `Missing: ${missing.length}, waiting on: ${incomplete.length}`;
	}
	if (missing.length > 0) {
		return `Missing dependencies: ${missing.length}`;
	}
	return `Waiting on ${incomplete.length} task${incomplete.length === 1 ? '' : 's'}`;
}

function detectCycles(
	nodeIds: string[],
	adj: Record<string, string[]>
): string[][] {
	const visiting = new Set<string>();
	const visited = new Set<string>();
	const stack: string[] = [];
	const cycles: string[][] = [];
	const seenCycleKey = new Set<string>();

	function dfs(nodeId: string) {
		if (visited.has(nodeId)) return;
		visiting.add(nodeId);
		stack.push(nodeId);

		for (const next of adj[nodeId] ?? []) {
			if (!visiting.has(next) && !visited.has(next)) {
				dfs(next);
				continue;
			}
			if (visiting.has(next)) {
				const index = stack.lastIndexOf(next);
				if (index >= 0) {
					const cycle = stack.slice(index);
					const normalized = [...cycle].sort().join('|');
					if (!seenCycleKey.has(normalized)) {
						seenCycleKey.add(normalized);
						cycles.push(cycle);
					}
				}
			}
		}

		stack.pop();
		visiting.delete(nodeId);
		visited.add(nodeId);
	}

	for (const nodeId of nodeIds) {
		if (!visited.has(nodeId)) dfs(nodeId);
	}

	return cycles;
}

export function buildPlanningSnapshot(tasks: TaskLike[]): PlanningSnapshot {
	const taskById = new Map(tasks.map((task) => [task.id, task]));
	const activeTasks = tasks.filter((task) => task.status !== 'done');
	const activeIds = new Set(activeTasks.map((task) => task.id));

	const dependentsByTask: Record<string, string[]> = {};
	for (const task of tasks) {
		dependentsByTask[task.id] = [];
	}

	const blockedTaskIds: string[] = [];
	const readyTaskIds: string[] = [];
	const dependencyIssues: PlanningSnapshot['dependencyIssues'] = [];
	const blockedReasonByTask: Record<string, string> = {};

	for (const task of activeTasks) {
		const deps = [...new Set(task.dependencies ?? [])].filter((depId) => depId !== task.id);
		const missing = deps.filter((depId) => !taskById.has(depId));
		const incomplete = deps.filter((depId) => {
			const depTask = taskById.get(depId);
			return depTask !== undefined && depTask.status !== 'done';
		});
		for (const depId of deps) {
			if (taskById.has(depId)) {
				dependentsByTask[depId].push(task.id);
			}
		}
		if (missing.length > 0 || incomplete.length > 0) {
			blockedTaskIds.push(task.id);
			dependencyIssues.push({ taskId: task.id, missing, incomplete });
			blockedReasonByTask[task.id] = formatBlockedReason(missing, incomplete);
		} else {
			readyTaskIds.push(task.id);
		}
	}

	const adj: Record<string, string[]> = {};
	const indegree = new Map<string, number>();
	for (const task of activeTasks) {
		adj[task.id] = [];
		indegree.set(task.id, 0);
	}
	for (const task of activeTasks) {
		for (const depId of task.dependencies ?? []) {
			if (!activeIds.has(depId) || depId === task.id) continue;
			adj[depId].push(task.id);
			indegree.set(task.id, (indegree.get(task.id) ?? 0) + 1);
		}
	}

	const queue = activeTasks
		.map((task) => task.id)
		.filter((taskId) => (indegree.get(taskId) ?? 0) === 0)
		.sort((a, b) => {
			const taskA = taskById.get(a)!;
			const taskB = taskById.get(b)!;
			const dueA = taskA.due_at ? Date.parse(taskA.due_at) : Infinity;
			const dueB = taskB.due_at ? Date.parse(taskB.due_at) : Infinity;
			if (dueA !== dueB) return dueA - dueB;
			return taskA.priority - taskB.priority;
		});

	const recommendedOrder: string[] = [];
	const queueWork = [...queue];
	while (queueWork.length > 0) {
		const nodeId = queueWork.shift()!;
		recommendedOrder.push(nodeId);
		for (const nextId of adj[nodeId] ?? []) {
			const nextInDegree = (indegree.get(nextId) ?? 0) - 1;
			indegree.set(nextId, nextInDegree);
			if (nextInDegree === 0) queueWork.push(nextId);
		}
		queueWork.sort((a, b) => {
			const taskA = taskById.get(a)!;
			const taskB = taskById.get(b)!;
			const dueA = taskA.due_at ? Date.parse(taskA.due_at) : Infinity;
			const dueB = taskB.due_at ? Date.parse(taskB.due_at) : Infinity;
			if (dueA !== dueB) return dueA - dueB;
			return taskA.priority - taskB.priority;
		});
	}

	const cycles = detectCycles(activeTasks.map((task) => task.id), adj);

	const dist = new Map<string, number>();
	const prev = new Map<string, string | null>();
	for (const nodeId of recommendedOrder) {
		const base = dist.get(nodeId) ?? durationMinutes(taskById.get(nodeId)!);
		dist.set(nodeId, base);
		prev.set(nodeId, prev.get(nodeId) ?? null);
		for (const nextId of adj[nodeId] ?? []) {
			const candidate = base + durationMinutes(taskById.get(nextId)!);
			if (candidate > (dist.get(nextId) ?? 0)) {
				dist.set(nextId, candidate);
				prev.set(nextId, nodeId);
			}
		}
	}

	let criticalEndId: string | null = null;
	let criticalPathMinutes = 0;
	for (const [nodeId, minutes] of dist.entries()) {
		if (minutes > criticalPathMinutes) {
			criticalPathMinutes = minutes;
			criticalEndId = nodeId;
		}
	}

	const criticalPathTaskIds: string[] = [];
	if (criticalEndId) {
		let cursor: string | null = criticalEndId;
		while (cursor) {
			criticalPathTaskIds.push(cursor);
			cursor = prev.get(cursor) ?? null;
		}
		criticalPathTaskIds.reverse();
	}

	return {
		blockedTaskIds,
		readyTaskIds,
		dependencyIssues,
		blockedReasonByTask,
		dependentsByTask,
		cycles,
		recommendedOrder,
		criticalPathTaskIds,
		criticalPathMinutes
	};
}

export function proposeDependencyDueDates(
	tasks: TaskLike[],
	now: Date = new Date()
): Record<string, string> {
	const snapshot = buildPlanningSnapshot(tasks);
	if (snapshot.cycles.length > 0) return {};

	const taskById = new Map(tasks.map((task) => [task.id, task]));
	const activeTasks = tasks.filter((task) => task.status !== 'done');
	const activeIds = new Set(activeTasks.map((task) => task.id));
	const proposed = new Map<string, Date>();

	for (const taskId of snapshot.recommendedOrder) {
		const task = taskById.get(taskId);
		if (!task || task.status === 'done') continue;

		const dependencyEnds: number[] = [];
		for (const depId of task.dependencies ?? []) {
			if (!activeIds.has(depId) || depId === taskId) continue;
			const depProposed = proposed.get(depId);
			const depTask = taskById.get(depId);
			if (depProposed) {
				dependencyEnds.push(depProposed.getTime());
			} else if (depTask?.due_at) {
				dependencyEnds.push(Date.parse(depTask.due_at));
			}
		}

		const startAt = dependencyEnds.length > 0 ? Math.max(now.getTime(), ...dependencyEnds) : now.getTime();
		const candidate = new Date(startAt + durationMinutes(task) * 60_000);
		const existingDue = task.due_at ? new Date(task.due_at) : null;
		const resolved = existingDue && existingDue.getTime() > candidate.getTime() ? existingDue : candidate;
		proposed.set(taskId, resolved);
	}

	const updates: Record<string, string> = {};
	for (const task of activeTasks) {
		const nextDue = proposed.get(task.id);
		if (!nextDue) continue;
		const current = task.due_at ? Date.parse(task.due_at) : null;
		if (current === null || Math.abs(current - nextDue.getTime()) > 5 * 60_000) {
			updates[task.id] = nextDue.toISOString();
		}
	}

	return updates;
}
