export type TimeBlockTaskCandidate = {
	id: string;
	title: string;
	score: number;
	estimateMinutes?: number | null;
	dueAt?: string | null;
	reason?: string | null;
};

export type TimeBlockBusyWindow = {
	start: string;
	end?: string | null;
};

export type TimeBlockSuggestion = {
	taskId: string;
	taskTitle: string;
	score: number;
	reason: string | null;
	start: string;
	end: string;
	durationMinutes: number;
	dueAt?: string | null;
};

export type TimeBlockPlannerOptions = {
	date: string;
	workdayStartHour?: number;
	workdayEndHour?: number;
	defaultBlockMinutes?: number;
	limit?: number;
};

type FreeWindow = {
	startMs: number;
	endMs: number;
};

const MIN_BLOCK_MINUTES = 15;
const MAX_BLOCK_MINUTES = 180;

function clamp(value: number, min: number, max: number): number {
	return Math.max(min, Math.min(max, value));
}

function parseLocalDate(date: string): Date {
	const [yearRaw, monthRaw, dayRaw] = date.split('-').map((value) => Number(value));
	const year = Number.isFinite(yearRaw) ? yearRaw : new Date().getFullYear();
	const month = Number.isFinite(monthRaw) ? monthRaw : 1;
	const day = Number.isFinite(dayRaw) ? dayRaw : 1;
	return new Date(Date.UTC(year, month - 1, day, 0, 0, 0, 0));
}

function toWindowMs(
	window: TimeBlockBusyWindow,
	workdayStartMs: number,
	workdayEndMs: number
): FreeWindow | null {
	const startMs = new Date(window.start).getTime();
	if (!Number.isFinite(startMs)) return null;
	const rawEndMs = window.end ? new Date(window.end).getTime() : startMs + 30 * 60 * 1000;
	const endMs = Number.isFinite(rawEndMs) ? rawEndMs : startMs + 30 * 60 * 1000;
	const clampedStart = Math.max(workdayStartMs, startMs);
	const clampedEnd = Math.min(workdayEndMs, Math.max(endMs, clampedStart));
	if (clampedEnd <= clampedStart) return null;
	return {
		startMs: clampedStart,
		endMs: clampedEnd
	};
}

function normalizeDurationMinutes(
	estimateMinutes: number | null | undefined,
	defaultBlockMinutes: number
): number {
	const fallback = clamp(Math.round(defaultBlockMinutes), MIN_BLOCK_MINUTES, MAX_BLOCK_MINUTES);
	if (estimateMinutes == null || !Number.isFinite(estimateMinutes)) return fallback;
	return clamp(Math.round(estimateMinutes), MIN_BLOCK_MINUTES, MAX_BLOCK_MINUTES);
}

function carveBusyWindows(initial: FreeWindow[], busy: FreeWindow[]): FreeWindow[] {
	let free = [...initial];
	for (const busyWindow of busy) {
		const next: FreeWindow[] = [];
		for (const slot of free) {
			if (busyWindow.endMs <= slot.startMs || busyWindow.startMs >= slot.endMs) {
				next.push(slot);
				continue;
			}
			if (busyWindow.startMs > slot.startMs) {
				next.push({ startMs: slot.startMs, endMs: busyWindow.startMs });
			}
			if (busyWindow.endMs < slot.endMs) {
				next.push({ startMs: busyWindow.endMs, endMs: slot.endMs });
			}
		}
		free = next.filter((slot) => slot.endMs - slot.startMs >= MIN_BLOCK_MINUTES * 60 * 1000);
	}
	return free.sort((a, b) => a.startMs - b.startMs);
}

function sortCandidates(candidates: TimeBlockTaskCandidate[]): TimeBlockTaskCandidate[] {
	return [...candidates].sort((a, b) => {
		const scoreDiff = (b.score ?? 0) - (a.score ?? 0);
		if (Math.abs(scoreDiff) > 1e-6) return scoreDiff;
		const dueA = a.dueAt ? new Date(a.dueAt).getTime() : Number.POSITIVE_INFINITY;
		const dueB = b.dueAt ? new Date(b.dueAt).getTime() : Number.POSITIVE_INFINITY;
		return dueA - dueB;
	});
}

export function computeTimeBlockSuggestions(
	candidates: TimeBlockTaskCandidate[],
	busyWindows: TimeBlockBusyWindow[],
	options: TimeBlockPlannerOptions
): TimeBlockSuggestion[] {
	const targetDate = parseLocalDate(options.date);
	const startHour = clamp(Math.round(options.workdayStartHour ?? 9), 0, 23);
	const endHour = clamp(Math.round(options.workdayEndHour ?? 18), startHour + 1, 24);
	const defaultBlockMinutes = options.defaultBlockMinutes ?? 45;
	const limit = clamp(Math.round(options.limit ?? 6), 1, 24);

	const workdayStart = new Date(targetDate);
	workdayStart.setUTCHours(startHour, 0, 0, 0);
	const workdayEnd = new Date(targetDate);
	workdayEnd.setUTCHours(endHour, 0, 0, 0);

	const workdayStartMs = workdayStart.getTime();
	const workdayEndMs = workdayEnd.getTime();
	if (workdayEndMs <= workdayStartMs) return [];

	const normalizedBusy = busyWindows
		.map((window) => toWindowMs(window, workdayStartMs, workdayEndMs))
		.filter((item): item is FreeWindow => Boolean(item))
		.sort((a, b) => a.startMs - b.startMs);

	let freeSlots = carveBusyWindows(
		[{ startMs: workdayStartMs, endMs: workdayEndMs }],
		normalizedBusy
	);
	const suggestions: TimeBlockSuggestion[] = [];

	for (const task of sortCandidates(candidates)) {
		if (suggestions.length >= limit || freeSlots.length === 0) break;
		const durationMinutes = normalizeDurationMinutes(task.estimateMinutes, defaultBlockMinutes);
		const durationMs = durationMinutes * 60 * 1000;
		const slotIndex = freeSlots.findIndex((slot) => slot.endMs - slot.startMs >= durationMs);
		if (slotIndex === -1) continue;

		const slot = freeSlots[slotIndex];
		const startMs = slot.startMs;
		const endMs = startMs + durationMs;
		suggestions.push({
			taskId: task.id,
			taskTitle: task.title,
			score: task.score,
			reason: task.reason ?? null,
			start: new Date(startMs).toISOString(),
			end: new Date(endMs).toISOString(),
			durationMinutes,
			dueAt: task.dueAt ?? null
		});

		const replacement: FreeWindow[] = [];
		if (endMs < slot.endMs) {
			replacement.push({ startMs: endMs, endMs: slot.endMs });
		}
		freeSlots = [
			...freeSlots.slice(0, slotIndex),
			...replacement,
			...freeSlots.slice(slotIndex + 1)
		].sort((a, b) => a.startMs - b.startMs);
	}

	return suggestions;
}
