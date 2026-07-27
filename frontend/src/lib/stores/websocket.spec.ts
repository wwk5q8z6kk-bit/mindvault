import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { handleChangeMessage, workspaceReconciliation, wsStatus } from './websocket';

beforeEach(() => {
	wsStatus.set('disconnected');
	workspaceReconciliation.set(null);
});

describe('workspaceReconciliation', () => {
	it('routes workspace reconciliation messages to Files views', async () => {
		await handleChangeMessage({
			type: 'change',
			node_id: '01962ee0-2230-7000-8000-000000000001',
			operation: 'workspace_reconciled',
			namespace: 'personal',
			timestamp: '2026-07-26T12:00:00Z'
		});

		expect(get(workspaceReconciliation)).toEqual({
			workspaceId: '01962ee0-2230-7000-8000-000000000001',
			namespace: 'personal',
			timestamp: '2026-07-26T12:00:00Z'
		});
	});
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
