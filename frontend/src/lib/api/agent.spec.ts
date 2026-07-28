import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CapturedIntent, ProactiveInsight } from './types';

const fetchJsonMock = vi.fn();

vi.mock('$app/environment', () => ({
	browser: false
}));

vi.mock('./client', () => ({
	API_BASE_URL: 'http://127.0.0.1:9470',
	fetchJson: (...args: unknown[]) => fetchJsonMock(...args)
}));

import * as agentApi from './agent';

describe('agent api client', () => {
	beforeEach(() => {
		fetchJsonMock.mockReset();
		agentApi.agentRunObservations.set(null);
	});

	it('calls versioned agent context endpoint', async () => {
		fetchJsonMock.mockResolvedValueOnce({
			executive_summary: 'summary',
			related_nodes: []
		});

		await agentApi.fetchAgentContext('node-1');

		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/agent/context?basis_node_id=node-1');
	});

	it('calls versioned intent endpoints', async () => {
		fetchJsonMock.mockResolvedValue([] as CapturedIntent[]);

		await agentApi.fetchIntents();
		await agentApi.fetchIntents('node-1', 'suggested');
		await agentApi.applyIntent('intent-1');
		await agentApi.dismissIntent('intent-2');

		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/agent/intents');
		expect(fetchJsonMock).toHaveBeenCalledWith(
			'/api/v1/agent/intents?node_id=node-1&status=suggested'
		);
		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/agent/intents/intent-1/apply', {
			method: 'POST'
		});
		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/agent/intents/intent-2/dismiss', {
			method: 'POST'
		});
	});

	it('calls versioned proactive and models endpoints', async () => {
		fetchJsonMock
			.mockResolvedValueOnce([] as ProactiveInsight[])
			.mockResolvedValueOnce([] as ProactiveInsight[])
			.mockResolvedValueOnce({
				embedding: { provider: 'openai', model: 'text-embedding-3-small' }
			});

		await agentApi.fetchInsights();
		await agentApi.generateInsights();
		await agentApi.fetchAiModels();

		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/proactive/insights');
		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/proactive/generate', {
			method: 'POST'
		});
		expect(fetchJsonMock).toHaveBeenCalledWith('/api/v1/agent/models');
	});

	it('dispatches agent run transitions onto the observation store', () => {
		const seen: unknown[] = [];
		const unsub = agentApi.agentRunObservations.subscribe((value) => {
			if (value) seen.push(value);
		});

		agentApi.dispatchAgentNotification({
			type: 'agent_run_transitioned',
			run_id: 'run-1',
			work_order_id: 'wo-1',
			status: 'awaiting_approval',
			failure_class: null
		});
		agentApi.dispatchAgentNotification({
			type: 'agent_run_gate_recorded',
			run_id: 'run-1',
			work_order_id: 'wo-1',
			gate: 'g2',
			outcome: 'pass'
		});
		// Malformed payloads must not poison the store.
		agentApi.dispatchAgentNotification({ type: 'agent_run_transitioned' });

		unsub();

		expect(seen).toHaveLength(2);
		expect(seen[0]).toMatchObject({
			event: {
				kind: 'transition',
				run_id: 'run-1',
				work_order_id: 'wo-1',
				status: 'awaiting_approval',
				failure_class: null
			}
		});
		expect(seen[1]).toMatchObject({
			event: {
				kind: 'gate',
				run_id: 'run-1',
				work_order_id: 'wo-1',
				gate: 'g2',
				outcome: 'pass'
			}
		});
		expect((seen[0] as { seq: number }).seq).toBeLessThan((seen[1] as { seq: number }).seq);
	});
});
