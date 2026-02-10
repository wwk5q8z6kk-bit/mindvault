import { writable } from 'svelte/store';

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

const initialState: ApiHealthState = {
	status: 'unknown',
	consecutiveFailures: 0,
	lastSuccessAt: null,
	lastFailureAt: null,
	lastPath: null,
	lastError: null
};

export const apiHealth = writable<ApiHealthState>(initialState);

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
