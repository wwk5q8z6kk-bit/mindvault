import { beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type {
	AgentRunView,
	ArtifactView,
	GateResultView,
	WorkOrderSummary
} from '$lib/api/workOrders';

const api = vi.hoisted(() => ({
	listWorkOrders: vi.fn<() => Promise<WorkOrderSummary[]>>(),
	listRuns: vi.fn<() => Promise<AgentRunView[]>>(),
	listArtifacts: vi.fn<() => Promise<ArtifactView[]>>(),
	listGates: vi.fn<() => Promise<GateResultView[]>>()
}));

vi.mock('$lib/api/agent', () => ({
	agentRunObservations: {
		subscribe: () => () => undefined
	}
}));

vi.mock('$lib/stores/toast', () => ({
	pushToast: vi.fn()
}));

vi.mock('$lib/api/workOrders', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/api/workOrders')>();
	return {
		...actual,
		listWorkOrders: api.listWorkOrders,
		listRuns: api.listRuns,
		listArtifacts: api.listArtifacts,
		listGates: api.listGates
	};
});

import WorkOrdersPage from './+page.svelte';

const order: WorkOrderSummary = {
	work_order_id: 'wo-1',
	revision: 3,
	work_order_uri: 'mindvault://work-order/wo-1',
	goal: 'Summarize the quarterly planning decisions',
	non_goals: ['Do not publish or notify participants'],
	success_criteria: ['Every decision links back to source evidence'],
	status: 'running',
	status_reason: null,
	budget: {
		wall_clock_secs: 3600,
		run_attempts: 3,
		model_tokens: 100_000,
		effect_actions: 2
	},
	remaining: {
		wall_clock_secs: 3400,
		run_attempts: 2,
		model_tokens: 91_000,
		effect_actions: 2
	},
	created_at: '2026-08-02T12:00:00.000Z',
	updated_at: '2026-08-02T12:05:00.000Z'
};

const run: AgentRunView = {
	run_id: 'run-1',
	run_uri: 'mindvault://agent-run/run-1',
	work_order_id: 'wo-1',
	node_id: 'node-1',
	attempt_no: 1,
	status: 'awaiting_approval',
	failure_class: null,
	actor: 'mindvault://node/identity/principal/research-agent',
	started_at: '2026-08-02T12:01:00.000Z',
	ended_at: null
};

describe('Work Orders review journey', () => {
	beforeEach(() => {
		api.listWorkOrders.mockReset().mockResolvedValue([order]);
		api.listRuns.mockReset().mockResolvedValue([run]);
		api.listArtifacts.mockReset().mockResolvedValue([]);
		api.listGates.mockReset().mockResolvedValue([]);
	});

	it('puts the declared outcome and human approval boundary ahead of run mechanics', async () => {
		render(WorkOrdersPage);

		await expect
			.element(
				page.getByRole('heading', { name: 'Review agent work before it becomes an outcome' })
			)
			.toBeInTheDocument();
		await expect
			.element(page.getByText('Every decision links back to source evidence'))
			.toBeInTheDocument();
		await expect
			.element(page.getByText('Do not publish or notify participants'))
			.toBeInTheDocument();
		await expect
			.element(page.getByText('Agent research agent', { exact: true }))
			.toBeInTheDocument();
		await expect.element(page.getByRole('button', { name: 'Approve run' })).toBeInTheDocument();
	});

	it('separates a service failure from a genuinely empty work queue', async () => {
		api.listWorkOrders.mockRejectedValue(new Error('offline'));

		render(WorkOrdersPage);

		await expect
			.element(page.getByRole('heading', { name: 'Governed work is unavailable' }))
			.toBeInTheDocument();
		await expect.element(page.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
		await expect
			.element(page.getByText('Your existing work remains unchanged.', { exact: false }))
			.toBeInTheDocument();
	});
});
