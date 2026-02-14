import { STORAGE_KEYS } from '$lib/constants/storage-keys';

const STORAGE_KEY = STORAGE_KEYS.COMMAND_PALETTE_USAGE;

type UsageEntry = { count: number; lastUsed: number };

function loadUsage(): Record<string, UsageEntry> {
	if (typeof localStorage === 'undefined') return {};
	try {
		return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
	} catch {
		return {};
	}
}

function saveUsage(map: Record<string, UsageEntry>) {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify(map));
}

export function recordUsage(id: string) {
	const usage = loadUsage();
	const entry = usage[id] ?? { count: 0, lastUsed: 0 };
	usage[id] = { count: entry.count + 1, lastUsed: Date.now() };
	saveUsage(usage);
}

export function getUsage(id: string) {
	const usage = loadUsage();
	return usage[id] ?? { count: 0, lastUsed: 0 };
}

export function getUsageMap() {
	return loadUsage();
}
