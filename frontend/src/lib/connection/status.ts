import type { ApiHealthStatus } from '$lib/api/client';
import type { WsStatus } from '$lib/stores/websocket';

/** Unified connectivity signal for pills and banners. */
export type ConnectionUiStatus = 'live' | 'online' | 'degraded' | 'offline' | 'connecting';

export type DeriveConnectionInput = {
	browserOnline: boolean;
	apiStatus: ApiHealthStatus;
	wsStatus: WsStatus;
};

/**
 * Derive one trust signal from browser, REST health, and websocket state.
 *
 * Priority is fail-closed: never show Live/Online when the API is offline or degraded.
 */
export function deriveConnectionStatus(input: DeriveConnectionInput): ConnectionUiStatus {
	if (!input.browserOnline) {
		return 'offline';
	}

	switch (input.apiStatus) {
		case 'offline':
			return 'offline';
		case 'degraded':
			return 'degraded';
		case 'unknown':
			// Avoid claiming Live before REST health is proven.
			return 'connecting';
		case 'healthy':
			if (input.wsStatus === 'connected') return 'live';
			if (input.wsStatus === 'connecting') return 'connecting';
			return 'online';
		default: {
			const _exhaustive: never = input.apiStatus;
			return _exhaustive;
		}
	}
}

export function connectionStatusLabel(status: ConnectionUiStatus): string {
	switch (status) {
		case 'live':
			return 'Live';
		case 'online':
			return 'Online';
		case 'degraded':
			return 'Degraded';
		case 'offline':
			return 'Offline';
		case 'connecting':
			return 'Connecting';
		default: {
			const _exhaustive: never = status;
			return _exhaustive;
		}
	}
}

export function connectionStatusDescription(status: ConnectionUiStatus): string {
	switch (status) {
		case 'live':
			return 'API and realtime sync are connected.';
		case 'online':
			return 'API is reachable. Realtime sync is reconnecting.';
		case 'degraded':
			return 'The API is reachable but returning server errors. Some actions may fail.';
		case 'offline':
			return 'The API is unreachable. Local data remains available, but sync and AI features are temporarily paused.';
		case 'connecting':
			return 'Checking backend connectivity…';
		default: {
			const _exhaustive: never = status;
			return _exhaustive;
		}
	}
}

export function shouldShowConnectionBanner(status: ConnectionUiStatus): boolean {
	return status === 'offline' || status === 'degraded';
}
