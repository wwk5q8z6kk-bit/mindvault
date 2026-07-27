import { API_BASE_URL, fetchJson } from './client';

export type WorkOrderStatus =
	| 'draft'
	| 'admitted'
	| 'running'
	| 'completed'
	| 'failed'
	| 'cancelled'
	| 'rejected'
	| 'budget_exhausted';

export type AgentRunStatus =
	| 'ready'
	| 'leased'
	| 'running'
	| 'awaiting_approval'
	| 'gated'
	| 'verified'
	| 'completed'
	| 'failed'
	| 'cancelled'
	| 'budget_exhausted';

export type GateId = 'g0' | 'g1' | 'g2' | 'g3' | 'g4' | 'g5' | 'g6' | 'g7';
export type GateOutcome = 'pass' | 'fail' | 'waived';
export type RiskTier = 'low' | 'standard' | 'high';
export type EdgeKind =
	| 'data'
	| 'artifact'
	| 'state'
	| 'conflict'
	| 'resource'
	| 'policy'
	| 'verification'
	| 'temporal';

/**
 * A run parked on owner approval is the system working correctly, not stalled.
 * Approval has no deadline: quiet hours or a week away are expected, so this
 * state must never be rendered as an error or a timeout.
 */
export const AWAITING_APPROVAL: AgentRunStatus = 'awaiting_approval';

export interface WorkOrderBudget {
	wall_clock_secs: number;
	run_attempts: number;
	model_tokens: number;
	effect_actions: number;
}

export interface WorkOrderSummary {
	work_order_id: string;
	revision: number;
	work_order_uri: string;
	goal: string;
	non_goals: string[];
	success_criteria: string[];
	status: WorkOrderStatus;
	status_reason: string | null;
	budget: WorkOrderBudget;
	remaining: WorkOrderBudget;
	created_at: string;
	updated_at: string;
}

export interface AgentRunView {
	run_id: string;
	run_uri: string;
	work_order_id: string;
	node_id: string;
	attempt_no: number;
	status: AgentRunStatus;
	failure_class: string | null;
	actor: string;
	started_at: string | null;
	ended_at: string | null;
}

export interface GateResultView {
	gate: GateId;
	outcome: GateOutcome;
	evaluator_actor: string;
	evidence_digest: string;
	detail: string | null;
	evaluated_at: string;
}

export interface ArtifactProvenanceView {
	relation: string;
	resource: string;
}

export interface ArtifactView {
	artifact_id: string;
	artifact_uri: string;
	run_id: string;
	artifact_kind: string;
	content_digest: string;
	sensitivity: string;
	retention: string;
	provenance: ArtifactProvenanceView[];
	created_at: string;
}

export interface RunReadinessView {
	run_id: string;
	ready: boolean;
	unsatisfied_edges: { kind: EdgeKind; from_node_id: string; detail: string | null }[];
	outstanding_gates: GateId[];
}

export interface CompleteRunResponse {
	completed: boolean;
	run: AgentRunView | null;
	outstanding_gates: GateId[];
}

export interface ProposedNodeRequest {
	purpose: string;
	executor_kind: string;
	risk_tier: RiskTier;
	read_scope?: string[];
	write_scope: string[];
	timeout_secs: number;
	max_attempts: number;
}

export interface CreateWorkOrderRequest {
	goal: string;
	non_goals?: string[];
	success_criteria: string[];
	prohibited_outcomes?: string[];
	budget: WorkOrderBudget;
	idempotency_key: string;
	nodes: ProposedNodeRequest[];
	edges?: { from: number; to: number; kind: EdgeKind }[];
}

export interface StartRunResponse {
	run: AgentRunView;
	/** Digests, not plaintext URIs: a sealed vault must not expose scope here. */
	lease_target_digests: string[];
	/** True when the owner must authorise before the run proceeds. Not an error. */
	awaiting_approval: boolean;
}

export interface RestoreSummary {
	work_order_id: string;
	nodes: number;
	edges: number;
	runs: number;
	gate_results: number;
	artifacts: number;
	/**
	 * Always false. An export moves the record, never the authority: a restored
	 * contract carries no grant and must be re-authorized locally before any new
	 * run can pass gate G0.
	 */
	authority_restored: boolean;
}

const BASE = '/api/v1/work-orders';

export function listWorkOrders(status?: WorkOrderStatus): Promise<WorkOrderSummary[]> {
	const params = new URLSearchParams();
	if (status) params.set('status', status);
	const query = params.toString();
	return fetchJson<WorkOrderSummary[]>(`${BASE}${query ? `?${query}` : ''}`);
}

export function getWorkOrder(workOrderId: string): Promise<WorkOrderSummary> {
	return fetchJson<WorkOrderSummary>(`${BASE}/${encodeURIComponent(workOrderId)}`);
}

export function createWorkOrder(request: CreateWorkOrderRequest): Promise<WorkOrderSummary> {
	return fetchJson<WorkOrderSummary>(BASE, {
		method: 'POST',
		body: JSON.stringify(request)
	});
}

/**
 * Start the next attempt for a node contract.
 *
 * Executes nothing: it records the governed run, spends a budgeted attempt, and
 * takes write leases only if the owner's autonomy policy allows. An
 * unconfigured vault parks the run for approval — `awaiting_approval` is the
 * human-led default working correctly, not a failure, and it has no deadline.
 */
export function startRun(
	workOrderId: string,
	nodeId: string,
	actor: string,
	confidence = 0
): Promise<StartRunResponse> {
	return fetchJson<StartRunResponse>(
		`${BASE}/${encodeURIComponent(workOrderId)}/nodes/${encodeURIComponent(nodeId)}/runs`,
		{ method: 'POST', body: JSON.stringify({ actor, confidence }) }
	);
}

export function listRuns(workOrderId: string): Promise<AgentRunView[]> {
	return fetchJson<AgentRunView[]>(`${BASE}/${encodeURIComponent(workOrderId)}/runs`);
}

export function getRunReadiness(workOrderId: string, runId: string): Promise<RunReadinessView> {
	return fetchJson<RunReadinessView>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/readiness`
	);
}

export function listGates(workOrderId: string, runId: string): Promise<GateResultView[]> {
	return fetchJson<GateResultView[]>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/gates`
	);
}

export function recordGate(
	workOrderId: string,
	runId: string,
	request: {
		gate: GateId;
		outcome: GateOutcome;
		evaluator_actor: string;
		evidence_digest: string;
		detail?: string;
	}
): Promise<GateResultView> {
	return fetchJson<GateResultView>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/gates`,
		{ method: 'POST', body: JSON.stringify(request) }
	);
}

export function approveRun(workOrderId: string, runId: string): Promise<AgentRunView> {
	return fetchJson<AgentRunView>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/approve`,
		{ method: 'POST' }
	);
}

export function completeRun(workOrderId: string, runId: string): Promise<CompleteRunResponse> {
	return fetchJson<CompleteRunResponse>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/complete`,
		{ method: 'POST' }
	);
}

export function failRun(
	workOrderId: string,
	runId: string,
	failureClass: string
): Promise<AgentRunView> {
	return fetchJson<AgentRunView>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/fail`,
		{ method: 'POST', body: JSON.stringify({ failure_class: failureClass }) }
	);
}

/**
 * Record an artifact produced by a run.
 *
 * The server derives the content digest from the bytes; there is deliberately
 * no digest parameter, because a caller-supplied one would let gate G2 verify a
 * claim against itself. Provenance is required — derived knowledge never erases
 * the authority of its evidence.
 */
export function recordArtifact(
	workOrderId: string,
	runId: string,
	request: {
		artifact_kind: string;
		content_base64: string;
		provenance: ArtifactProvenanceView[];
	}
): Promise<ArtifactView> {
	return fetchJson<ArtifactView>(
		`${BASE}/${encodeURIComponent(workOrderId)}/runs/${encodeURIComponent(runId)}/artifacts`,
		{ method: 'POST', body: JSON.stringify(request) }
	);
}

export function listArtifacts(workOrderId: string): Promise<ArtifactView[]> {
	return fetchJson<ArtifactView[]>(`${BASE}/${encodeURIComponent(workOrderId)}/artifacts`);
}

/**
 * Direct link to an artifact's bytes.
 *
 * Served as an opaque octet stream with `nosniff`: agent output is not trusted
 * markup, so it is downloaded rather than rendered inline.
 */
export function artifactContentUrl(workOrderId: string, artifactId: string): string {
	return `${API_BASE_URL}${BASE}/${encodeURIComponent(workOrderId)}/artifacts/${encodeURIComponent(
		artifactId
	)}/content`;
}

export function exportWorkOrder(workOrderId: string): Promise<unknown> {
	return fetchJson<unknown>(`${BASE}/${encodeURIComponent(workOrderId)}/export`);
}

export function restoreWorkOrder(exported: unknown): Promise<RestoreSummary> {
	return fetchJson<RestoreSummary>(`${BASE}/restore`, {
		method: 'POST',
		body: JSON.stringify(exported)
	});
}

/**
 * Gates required at each risk tier, mirroring `RiskTier::required_gates` in
 * `crates/mv-core/src/model/work_order.rs`.
 *
 * Used only to label a run's outstanding work in the UI. The server decides
 * what is actually required; nothing here is enforcement.
 */
export const REQUIRED_GATES: Record<RiskTier, GateId[]> = {
	low: ['g0', 'g1', 'g2', 'g6'],
	standard: ['g0', 'g1', 'g2', 'g3', 'g4', 'g6'],
	high: ['g0', 'g1', 'g2', 'g3', 'g4', 'g5', 'g6', 'g7']
};

export const GATE_LABELS: Record<GateId, string> = {
	g0: 'Scope within grant',
	g1: 'Schema registered',
	g2: 'Outputs verified',
	g3: 'Provenance resolves',
	g4: 'Evidence preserved',
	g5: 'Independent review',
	g6: 'Ceilings honoured',
	g7: 'Owner approval'
};
