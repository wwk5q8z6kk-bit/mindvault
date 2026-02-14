import { writable, get } from 'svelte/store';
import { STORAGE_KEYS } from '$lib/constants/storage-keys';

export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://127.0.0.1:9470';

// ---------------------------------------------------------------------------
// API health tracking (moved from stores/api-health.ts)
// ---------------------------------------------------------------------------

export type ApiHealthStatus = 'unknown' | 'healthy' | 'degraded' | 'offline';
export type ApiFailureKind = 'network' | 'timeout' | 'server' | 'invalid_response';

export interface ApiHealthState {
	status: ApiHealthStatus;
	consecutiveFailures: number;
	lastSuccessAt: number | null;
	lastFailureAt: number | null;
	lastPath: string | null;
	lastError: string | null;
}

const defaultHealthState: ApiHealthState = {
	status: 'unknown',
	consecutiveFailures: 0,
	lastSuccessAt: null,
	lastFailureAt: null,
	lastPath: null,
	lastError: null
};

const HEALTH_STORAGE_KEY = STORAGE_KEYS.API_HEALTH_STATE;
const BASE_OFFLINE_COOLDOWN_MS = 2500;
const MAX_OFFLINE_COOLDOWN_MS = 60000;
const PERSISTED_STATE_TTL_MS = 5 * 60 * 1000;

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
	if (typeof localStorage === 'undefined') return defaultHealthState;
	try {
		const raw = localStorage.getItem(HEALTH_STORAGE_KEY);
		if (!raw) return defaultHealthState;
		const parsed = JSON.parse(raw) as Partial<ApiHealthState>;
		if (parsed && typeof parsed === 'object') {
			const status = parsed.status;
			const allowed = status === 'unknown' || status === 'healthy' || status === 'degraded' || status === 'offline';
			const state: ApiHealthState = {
				status: allowed ? status : defaultHealthState.status,
				consecutiveFailures: sanitizeFailureCount(parsed.consecutiveFailures),
				lastSuccessAt: isFiniteNumber(parsed.lastSuccessAt) ? parsed.lastSuccessAt : null,
				lastFailureAt: isFiniteNumber(parsed.lastFailureAt) ? parsed.lastFailureAt : null,
				lastPath: typeof parsed.lastPath === 'string' ? parsed.lastPath : null,
				lastError: typeof parsed.lastError === 'string' ? parsed.lastError : null
			};
			return isExpired(state) ? defaultHealthState : state;
		}
		return defaultHealthState;
	} catch {
		return defaultHealthState;
	}
}

function persistState(state: ApiHealthState): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(HEALTH_STORAGE_KEY, JSON.stringify(state));
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

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

const HEALTH_PROBE_PATH = '/api/v1/nodes?limit=1';

let reachabilityProbePromise: Promise<boolean> | null = null;

async function probeApiReachability(timeoutMs: number): Promise<boolean> {
	if (reachabilityProbePromise) {
		return reachabilityProbePromise;
	}

	reachabilityProbePromise = (async () => {
		const controller = new AbortController();
		const timer = setTimeout(() => controller.abort(), Math.min(timeoutMs, 3000));
		try {
			const res = await fetch(`${API_BASE_URL}${HEALTH_PROBE_PATH}`, {
				method: 'GET',
				headers: { Accept: 'application/json' },
				signal: controller.signal
			});
			if (res.status >= 500) {
				markApiFailure('server', HEALTH_PROBE_PATH, `HTTP ${res.status}`);
			} else {
				// Any non-5xx response means the backend is reachable.
				markApiSuccess(HEALTH_PROBE_PATH);
			}
			return true;
		} catch (err) {
			const kind = err instanceof Error && err.name === 'AbortError' ? 'timeout' : 'network';
			const detail = err instanceof Error ? err.message : 'Probe failed';
			markApiFailure(kind, HEALTH_PROBE_PATH, detail);
			return false;
		} finally {
			clearTimeout(timer);
			reachabilityProbePromise = null;
		}
	})();

	return reachabilityProbePromise;
}

export class ApiError extends Error {
	status: number;
	body?: unknown;

	constructor(message: string, status: number, body?: unknown) {
		super(message);
		this.status = status;
		this.body = body;
	}
}

export async function fetchJson<T>(
	path: string,
	options: RequestInit = {},
	{
		timeoutMs = 10000,
		bypassOfflineCircuit = false
	}: { timeoutMs?: number; bypassOfflineCircuit?: boolean } = {}
): Promise<T> {
	if (!bypassOfflineCircuit) {
		let health = get(apiHealth);
		if (health.status === 'unknown') {
			const reachable = await probeApiReachability(timeoutMs);
			if (!reachable) {
				throw new ApiError('Backend unavailable', 0, {
					reason: 'offline_probe'
				});
			}
			health = get(apiHealth);
		}

		if (health.status === 'offline' && typeof health.lastFailureAt === 'number') {
			const elapsed = Date.now() - health.lastFailureAt;
			const cooldownMs = getOfflineCooldownMs(health.consecutiveFailures);
			if (elapsed < cooldownMs) {
				throw new ApiError('Backend unavailable', 0, {
					reason: 'offline_cooldown',
					cooldown_ms: cooldownMs,
					retry_in_ms: cooldownMs - elapsed,
					failures: health.consecutiveFailures
				});
			}

			const reachable = await probeApiReachability(timeoutMs);
			if (!reachable) {
				throw new ApiError('Backend unavailable', 0, {
					reason: 'offline_probe'
				});
			}
		}
	}

	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), timeoutMs);

	try {
		let res: Response;
		try {
			res = await fetch(`${API_BASE_URL}${path}`, {
				...options,
				headers: {
					'Content-Type': 'application/json',
					...(options.headers ?? {})
				},
				signal: controller.signal
			});
		} catch (err) {
			const kind = err instanceof Error && err.name === 'AbortError' ? 'timeout' : 'network';
			const detail = err instanceof Error ? err.message : 'Request failed';
			markApiFailure(kind, path, detail);
			throw new ApiError(
				kind === 'timeout' ? `Request timed out (${timeoutMs}ms)` : 'Network unavailable',
				0,
				detail
			);
		}

		if (!res.ok) {
			let body: unknown = undefined;
			try {
				body = await res.json();
			} catch {
				body = await res.text();
			}
			if (res.status >= 500) {
				markApiFailure('server', path, `HTTP ${res.status}`);
			} else {
				// 4xx responses still prove the backend is reachable.
				markApiSuccess(path);
			}
			throw new ApiError(`Request failed (${res.status})`, res.status, body);
		}

		if (res.status === 204 || res.status === 205) {
			markApiSuccess(path);
			return undefined as T;
		}

		const text = await res.text();
		if (!text) {
			markApiSuccess(path);
			return undefined as T;
		}

		try {
			const data = JSON.parse(text) as T;
			markApiSuccess(path);
			return data;
		} catch {
			markApiFailure('invalid_response', path, 'Invalid JSON response');
			throw new ApiError('Invalid JSON response', res.status, text);
		}
	} finally {
		clearTimeout(timer);
	}
}
