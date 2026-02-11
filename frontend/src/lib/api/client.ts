import { get } from 'svelte/store';
import { apiHealth, getOfflineCooldownMs, markApiFailure, markApiSuccess } from '$lib/stores/api-health';
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://127.0.0.1:9470';
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
