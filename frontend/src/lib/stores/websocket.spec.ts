import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { wsStatus } from './websocket';

beforeEach(() => {
	wsStatus.set('disconnected');
});

describe('wsStatus', () => {
	it('starts disconnected', () => {
		expect(get(wsStatus)).toBe('disconnected');
	});

	it('can be set to connecting', () => {
		wsStatus.set('connecting');
		expect(get(wsStatus)).toBe('connecting');
	});

	it('can be set to connected', () => {
		wsStatus.set('connected');
		expect(get(wsStatus)).toBe('connected');
	});

	it('can transition through states', () => {
		expect(get(wsStatus)).toBe('disconnected');
		wsStatus.set('connecting');
		expect(get(wsStatus)).toBe('connecting');
		wsStatus.set('connected');
		expect(get(wsStatus)).toBe('connected');
		wsStatus.set('disconnected');
		expect(get(wsStatus)).toBe('disconnected');
	});
});
