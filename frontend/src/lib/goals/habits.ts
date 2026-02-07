const DATE_ONLY_RE = /^\d{4}-\d{2}-\d{2}$/;

export function toDateOnly(date: Date): string {
	return date.toISOString().slice(0, 10);
}

export function normalizeDateList(dates: string[]): string[] {
	const unique = new Set<string>();
	for (const date of dates) {
		if (DATE_ONLY_RE.test(date)) {
			unique.add(date);
		}
	}
	return [...unique].sort();
}

export function toggleCheckinDate(dates: string[], targetDate: string): string[] {
	const normalized = normalizeDateList(dates);
	if (!DATE_ONLY_RE.test(targetDate)) return normalized;
	if (normalized.includes(targetDate)) {
		return normalized.filter((date) => date !== targetDate);
	}
	return normalizeDateList([...normalized, targetDate]);
}

export function latestCheckinDate(dates: string[]): string | null {
	const normalized = normalizeDateList(dates);
	if (normalized.length === 0) return null;
	return normalized[normalized.length - 1];
}

function dateDiffDays(fromDate: string, toDate: string): number {
	const start = new Date(`${fromDate}T00:00:00.000Z`).getTime();
	const end = new Date(`${toDate}T00:00:00.000Z`).getTime();
	return Math.floor((end - start) / 86_400_000);
}

export function calculateCurrentStreak(dates: string[], now: Date = new Date()): number {
	const normalized = normalizeDateList(dates);
	if (normalized.length === 0) return 0;

	const latest = normalized[normalized.length - 1];
	const today = toDateOnly(now);
	const gapFromToday = dateDiffDays(latest, today);
	if (gapFromToday > 1) {
		return 0;
	}

	let streak = 1;
	for (let idx = normalized.length - 1; idx > 0; idx -= 1) {
		const prev = normalized[idx - 1];
		const current = normalized[idx];
		if (dateDiffDays(prev, current) === 1) {
			streak += 1;
		} else {
			break;
		}
	}
	return streak;
}

export function calculateBestStreak(dates: string[]): number {
	const normalized = normalizeDateList(dates);
	if (normalized.length === 0) return 0;

	let best = 1;
	let current = 1;

	for (let idx = 1; idx < normalized.length; idx += 1) {
		const prev = normalized[idx - 1];
		const value = normalized[idx];
		if (dateDiffDays(prev, value) === 1) {
			current += 1;
			best = Math.max(best, current);
		} else {
			current = 1;
		}
	}

	return best;
}
