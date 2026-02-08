import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CapturedIntent, ProactiveInsight } from './types';

const fetchJsonMock = vi.fn();

vi.mock('$app/environment', () => ({
	browser: false
}));

vi.mock('./client', () => ({
	API_BASE_URL: 'http://127.0.0.1:9470',
	fetchJson: fetchJsonMock
}));

let agentApi: typeof import('./agent');

describe('agent api client', () => {
	beforeAll(async () => {
		agentApi = await import('./agent');
	}, 15_000);

	beforeEach(() => {
		fetchJsonMock.mockReset();
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
});
