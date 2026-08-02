import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchJson = vi.fn();

vi.mock('./client', () => ({
	API_BASE_URL: 'http://127.0.0.1:9470',
	fetchJson: (...args: unknown[]) => fetchJson(...args)
}));

import {
	approveRun,
	artifactContentUrl,
	completeRun,
	createWorkOrder,
	exportWorkOrder,
	failRun,
	getRunReadiness,
	getWorkOrder,
	listArtifacts,
	listGates,
	listRuns,
	listWorkOrders,
	recordArtifact,
	recordGate,
	restoreWorkOrder,
	startRun,
	AWAITING_APPROVAL,
	REQUIRED_GATES
} from './workOrders';

describe('work order api client', () => {
	beforeEach(() => {
		fetchJson.mockReset();
		fetchJson.mockResolvedValue({});
	});

	it('lists all work orders or filters by lifecycle status', async () => {
		await listWorkOrders();
		await listWorkOrders('admitted');

		expect(fetchJson).toHaveBeenNthCalledWith(1, '/api/v1/work-orders');
		expect(fetchJson).toHaveBeenNthCalledWith(2, '/api/v1/work-orders?status=admitted');
	});

	it('encodes identifiers so a malformed id cannot alter the path', async () => {
		await getWorkOrder('../../admin');
		expect(fetchJson).toHaveBeenCalledWith('/api/v1/work-orders/..%2F..%2Fadmin');

		await listGates('a/b', 'c d');
		expect(fetchJson).toHaveBeenCalledWith('/api/v1/work-orders/a%2Fb/runs/c%20d/gates');
	});

	it('sends a work order proposal with its declared write scope', async () => {
		await createWorkOrder({
			goal: 'extract candidate decisions',
			success_criteria: ['decisions carry provenance'],
			budget: {
				wall_clock_secs: 3600,
				run_attempts: 5,
				model_tokens: 100000,
				effect_actions: 10
			},
			idempotency_key: 'extract-decisions',
			nodes: [
				{
					purpose: 'extract',
					executor_kind: 'engine',
					risk_tier: 'low',
					write_scope: ['mindvault://schemas/decisions'],
					timeout_secs: 600,
					max_attempts: 3
				}
			]
		});

		const [path, init] = fetchJson.mock.calls[0];
		expect(path).toBe('/api/v1/work-orders');
		expect(init.method).toBe('POST');
		expect(JSON.parse(init.body).nodes[0].write_scope).toEqual([
			'mindvault://schemas/decisions'
		]);
	});

	it('reads runs, readiness, gates, and artifacts through the query surface', async () => {
		await listRuns('wo-1');
		await getRunReadiness('wo-1', 'run-1');
		await listArtifacts('wo-1');

		expect(fetchJson).toHaveBeenNthCalledWith(1, '/api/v1/work-orders/wo-1/runs');
		expect(fetchJson).toHaveBeenNthCalledWith(
			2,
			'/api/v1/work-orders/wo-1/runs/run-1/readiness'
		);
		expect(fetchJson).toHaveBeenNthCalledWith(3, '/api/v1/work-orders/wo-1/artifacts');
	});

	it('records gate evidence with an attributable evaluator', async () => {
		await recordGate('wo-1', 'run-1', {
			gate: 'g5',
			outcome: 'pass',
			evaluator_actor: 'mindvault://node/identity/principal/owner',
			evidence_digest: 'a'.repeat(64)
		});

		const [path, init] = fetchJson.mock.calls[0];
		expect(path).toBe('/api/v1/work-orders/wo-1/runs/run-1/gates');
		expect(init.method).toBe('POST');
		// Evidence is attributable, never anonymous — the server refuses a run
		// that names itself as its own G5 evaluator.
		expect(JSON.parse(init.body).evaluator_actor).toBe(
			'mindvault://node/identity/principal/owner'
		);
	});

	it('starts a run without claiming autonomy it was not given', async () => {
		await startRun('wo-1', 'node-1', 'mindvault://node/identity/principal/agent');

		const [path, init] = fetchJson.mock.calls[0];
		expect(path).toBe('/api/v1/work-orders/wo-1/nodes/node-1/runs');
		expect(init.method).toBe('POST');
		// Confidence defaults to zero — the value least likely to clear an
		// auto-apply threshold, so an omitted argument cannot buy autonomy.
		expect(JSON.parse(init.body)).toEqual({
			actor: 'mindvault://node/identity/principal/agent',
			confidence: 0
		});
	});

	it('drives the run lifecycle commands', async () => {
		await approveRun('wo-1', 'run-1');
		await completeRun('wo-1', 'run-1');
		await failRun('wo-1', 'run-1', 'deterministic');

		expect(fetchJson.mock.calls[0][0]).toBe('/api/v1/work-orders/wo-1/runs/run-1/approve');
		expect(fetchJson.mock.calls[1][0]).toBe('/api/v1/work-orders/wo-1/runs/run-1/complete');
		expect(fetchJson.mock.calls[2][0]).toBe('/api/v1/work-orders/wo-1/runs/run-1/fail');
		// Blind retry is not recovery: a failure must be classified.
		expect(JSON.parse(fetchJson.mock.calls[2][1].body)).toEqual({
			failure_class: 'deterministic'
		});
	});

	it('records an artifact without supplying its own digest', async () => {
		await recordArtifact('wo-1', 'run-1', {
			artifact_kind: 'decision-summary',
			content_base64: 'ZGVjaXNpb25z',
			provenance: [
				{ relation: 'WasDerivedFrom', resource: 'mindvault://schemas/meeting' }
			]
		});

		const [path, init] = fetchJson.mock.calls[0];
		expect(path).toBe('/api/v1/work-orders/wo-1/runs/run-1/artifacts');
		expect(init.method).toBe('POST');
		const sent = JSON.parse(init.body);
		// No digest field: the server derives it from the bytes, so a client
		// cannot make G2 check a claim against itself.
		expect(sent.content_digest).toBeUndefined();
		expect(sent.provenance).toHaveLength(1);
	});

	it('exports and restores the portable graph', async () => {
		await exportWorkOrder('wo-1');
		expect(fetchJson).toHaveBeenCalledWith('/api/v1/work-orders/wo-1/export');

		await restoreWorkOrder({ format_version: 'work-order-export-v1' });
		const [path, init] = fetchJson.mock.calls[1];
		expect(path).toBe('/api/v1/work-orders/restore');
		expect(init.method).toBe('POST');
	});

	it('builds an absolute artifact content url for download', () => {
		expect(artifactContentUrl('wo-1', 'art-1')).toBe(
			'http://127.0.0.1:9470/api/v1/work-orders/wo-1/artifacts/art-1/content'
		);
	});

	it('mirrors the risk-scaled gate requirements', () => {
		// The scope gate and the ceiling gate are never waived at any tier.
		for (const tier of ['low', 'standard', 'high'] as const) {
			expect(REQUIRED_GATES[tier]).toContain('g0');
			expect(REQUIRED_GATES[tier]).toContain('g6');
		}
		// Independent review and owner approval apply only at the high tier.
		expect(REQUIRED_GATES.low).not.toContain('g5');
		expect(REQUIRED_GATES.standard).not.toContain('g5');
		expect(REQUIRED_GATES.high).toContain('g5');
		expect(REQUIRED_GATES.high).toContain('g7');
	});

	it('names the approval state so it is never rendered as a failure', () => {
		expect(AWAITING_APPROVAL).toBe('awaiting_approval');
	});
});
