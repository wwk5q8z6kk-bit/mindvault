import { writable } from 'svelte/store';

export type ApiHealthStatus = 'unknown' | 'healthy' | 'degraded' | 'offline';
export type ApiFailureKind = 'network' | 'timeout' | 'server' | 'invalid_response';
const STORAGE_KEY = 'mv_api_health_state';
const BASE_OFFLINE_COOLDOWN_MS = 2500;
const MAX_OFFLINE_COOLDOWN_MS = 60000;
const PERSISTED_STATE_TTL_MS = 5 * 60 * 1000;

export interface ApiHealthState {
	status: ApiHealthStatus;
	consecutiveFailures: number;
	lastSuccessAt: number | null;
	lastFailureAt: number | null;
	lastPath: string | null;
	lastError: string | null;
}

const defaultState: ApiHealthState = {
	status: 'unknown',
	consecutiveFailures: 0,
	lastSuccessAt: null,
	lastFailureAt: null,
	lastPath: null,
	lastError: null
};

function isFiniteNumber(value: unknown): value is number {
	return typeof value === 'number' && Number.isFinite(value);
}

function sanitizeFailureCount(value: unknown): number {
	if (!isFiniteNumber(value)) return 0;
	if (value < 0) return 0;
	return Math.floor(value);
}

function isExpired(state: ApiHealthState): boolean {
	const now = Date.now();
	const referenceTs = Math.max(state.lastFailureAt ?? 0, state.lastSuccessAt ?? 0);
	if (!referenceTs) return false;
	return now - referenceTs > PERSISTED_STATE_TTL_MS;
}

function readStoredState(): ApiHealthState {
	if (typeof localStorage === 'undefined') return defaultState;
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return defaultState;
		const parsed = JSON.parse(raw) as Partial<ApiHealthState>;
		if (parsed && typeof parsed === 'object') {
			const status = parsed.status;
			const allowed = status === 'unknown' || status === 'healthy' || status === 'degraded' || status === 'offline';
			const state: ApiHealthState = {
				status: allowed ? status : defaultState.status,
				consecutiveFailures: sanitizeFailureCount(parsed.consecutiveFailures),
				lastSuccessAt: isFiniteNumber(parsed.lastSuccessAt) ? parsed.lastSuccessAt : null,
				lastFailureAt: isFiniteNumber(parsed.lastFailureAt) ? parsed.lastFailureAt : null,
				lastPath: typeof parsed.lastPath === 'string' ? parsed.lastPath : null,
				lastError: typeof parsed.lastError === 'string' ? parsed.lastError : null
			};
			return isExpired(state) ? defaultState : state;
		}
		return defaultState;
	} catch {
		return defaultState;
	}
}

function persistState(state: ApiHealthState): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
	} catch {
		// Ignore quota/storage errors.
	}
}

export const apiHealth = writable<ApiHealthState>(readStoredState());

apiHealth.subscribe((state) => {
	persistState(state);
});

export function getOfflineCooldownMs(failures: number): number {
	const failureCount = sanitizeFailureCount(failures);
	if (failureCount <= 0) return BASE_OFFLINE_COOLDOWN_MS;
	const exponent = Math.min(failureCount - 1, 8);
	return Math.min(MAX_OFFLINE_COOLDOWN_MS, BASE_OFFLINE_COOLDOWN_MS * 2 ** exponent);
}

export function markApiSuccess(path: string): void {
	apiHealth.update((state) => ({
		...state,
		status: 'healthy',
		consecutiveFailures: 0,
		lastSuccessAt: Date.now(),
		lastPath: path,
		lastError: null
	}));
}

export function markApiFailure(kind: ApiFailureKind, path: string, detail?: string): void {
	apiHealth.update((state) => ({
		...state,
		status: kind === 'network' || kind === 'timeout' ? 'offline' : 'degraded',
		consecutiveFailures: state.consecutiveFailures + 1,
		lastFailureAt: Date.now(),
		lastPath: path,
		lastError: detail ?? kind
	}));
}
