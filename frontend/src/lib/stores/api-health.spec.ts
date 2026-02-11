import { describe, expect, it, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	apiHealth,
	markApiSuccess,
	markApiFailure,
	getOfflineCooldownMs,
	type ApiHealthState
} from './api-health';

const defaultState: ApiHealthState = {
	status: 'unknown',
	consecutiveFailures: 0,
	lastSuccessAt: null,
	lastFailureAt: null,
	lastPath: null,
	lastError: null
};

beforeEach(() => {
	apiHealth.set({ ...defaultState });
});

describe('markApiSuccess', () => {
	it('sets status to healthy and resets failures', () => {
		markApiFailure('network', '/api/test');
		markApiSuccess('/api/ok');
		const state = get(apiHealth);
		expect(state.status).toBe('healthy');
		expect(state.consecutiveFailures).toBe(0);
		expect(state.lastPath).toBe('/api/ok');
		expect(state.lastError).toBeNull();
		expect(state.lastSuccessAt).toBeTypeOf('number');
	});
});

describe('markApiFailure', () => {
	it('sets status to offline for network errors', () => {
		markApiFailure('network', '/api/test', 'ECONNREFUSED');
		const state = get(apiHealth);
		expect(state.status).toBe('offline');
		expect(state.consecutiveFailures).toBe(1);
		expect(state.lastPath).toBe('/api/test');
		expect(state.lastError).toBe('ECONNREFUSED');
	});

	it('sets status to offline for timeout errors', () => {
		markApiFailure('timeout', '/api/slow');
		expect(get(apiHealth).status).toBe('offline');
	});

	it('sets status to degraded for server errors', () => {
		markApiFailure('server', '/api/broken', '500 Internal');
		const state = get(apiHealth);
		expect(state.status).toBe('degraded');
		expect(state.lastError).toBe('500 Internal');
	});

	it('sets status to degraded for invalid_response', () => {
		markApiFailure('invalid_response', '/api/bad');
		expect(get(apiHealth).status).toBe('degraded');
	});

	it('increments consecutive failures', () => {
		markApiFailure('server', '/api/1');
		markApiFailure('server', '/api/2');
		markApiFailure('server', '/api/3');
		expect(get(apiHealth).consecutiveFailures).toBe(3);
	});
});

describe('getOfflineCooldownMs', () => {
	it('returns base cooldown for zero failures', () => {
		expect(getOfflineCooldownMs(0)).toBe(2500);
	});

	it('returns base cooldown for first failure', () => {
		expect(getOfflineCooldownMs(1)).toBe(2500);
	});

	it('doubles for subsequent failures', () => {
		expect(getOfflineCooldownMs(2)).toBe(5000);
		expect(getOfflineCooldownMs(3)).toBe(10000);
	});

	it('caps at max cooldown', () => {
		expect(getOfflineCooldownMs(100)).toBe(60000);
	});

	it('handles negative values safely', () => {
		expect(getOfflineCooldownMs(-1)).toBe(2500);
	});
});
