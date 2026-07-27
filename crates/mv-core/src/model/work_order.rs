//! Governed agent execution graph: Work Orders, node contracts, typed edges,
//! Agent Runs, write leases, gate evidence, and artifacts.
//!
//! Contract: `docs/architecture/WORK_ORDER_MODEL.md`.
//! Isolation contract: `docs/architecture/EXECUTION_ISOLATION_MODEL.md`.
//! Decision: `docs/adr/012-governed-agent-execution-graph.md`.
//!
//! The binding rule of this module is that a node contract's declared write
//! scope is the AuthorityGrant target set. Scope is never enforced by
//! instruction text, because `PROTOCOL_BOUNDARIES.md` treats external tool
//! descriptions as untrusted and a compromised executor ignores prompts.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::str::FromStr;
use uuid::Uuid;

use super::interoperability::interoperability_string_enum;
use super::{ProvenanceReference, RetentionClass, SchemaReference, Sensitivity, StableUri};
#[cfg(test)]
use super::ProvenanceRelation;

/// Maximum write-lease duration, matching the outbox dispatch bound in
/// `ACTION_RECEIPT_MODEL.md`. A longer lease would let a crashed run hold a
/// target indefinitely.
pub const MAX_WRITE_LEASE_SECS: i64 = 3600;

// ---------------------------------------------------------------------------
// String enums
// ---------------------------------------------------------------------------

interoperability_string_enum! {
    /// Lifecycle of one unit of governed intent.
    pub enum WorkOrderStatus {
        Draft => "draft",
        Admitted => "admitted",
        Running => "running",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
        Rejected => "rejected",
        BudgetExhausted => "budget_exhausted",
    }
}

impl WorkOrderStatus {
    pub const fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Draft, Self::Admitted)
                | (Self::Draft, Self::Rejected)
                | (Self::Draft, Self::Cancelled)
                | (Self::Admitted, Self::Running)
                | (Self::Admitted, Self::Cancelled)
                | (Self::Running, Self::Completed)
                | (Self::Running, Self::Failed)
                | (Self::Running, Self::Cancelled)
                | (Self::Running, Self::BudgetExhausted)
        )
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Failed
                | Self::Cancelled
                | Self::Rejected
                | Self::BudgetExhausted
        )
    }
}

interoperability_string_enum! {
    pub enum WorkOrderNodeStatus {
        Pending => "pending",
        Ready => "ready",
        Active => "active",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
    }
}

interoperability_string_enum! {
    /// Lifecycle of one bounded execution attempt of one node contract.
    pub enum AgentRunStatus {
        Ready => "ready",
        Leased => "leased",
        Running => "running",
        AwaitingApproval => "awaiting_approval",
        Gated => "gated",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
        BudgetExhausted => "budget_exhausted",
    }
}

impl AgentRunStatus {
    pub const fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Ready, Self::Leased)
                // The autonomy gate is consulted when a run starts, not only
                // mid-execution. A run the owner has not authorized parks
                // before it takes any write lease, so an unbounded approval
                // wait never holds a target URI.
                | (Self::Ready, Self::AwaitingApproval)
                | (Self::Ready, Self::Cancelled)
                | (Self::Ready, Self::BudgetExhausted)
                | (Self::Leased, Self::Running)
                | (Self::Leased, Self::Ready)
                | (Self::Leased, Self::Cancelled)
                | (Self::Leased, Self::BudgetExhausted)
                | (Self::Running, Self::AwaitingApproval)
                | (Self::Running, Self::Gated)
                | (Self::Running, Self::Failed)
                | (Self::Running, Self::Cancelled)
                | (Self::Running, Self::BudgetExhausted)
                // Approval is unbounded and returns to the ready set, where the
                // run re-acquires leases under a fresh conflict check.
                | (Self::AwaitingApproval, Self::Ready)
                | (Self::AwaitingApproval, Self::Cancelled)
                | (Self::AwaitingApproval, Self::BudgetExhausted)
                | (Self::Gated, Self::Completed)
                | (Self::Gated, Self::Failed)
                | (Self::Gated, Self::Ready)
                | (Self::Gated, Self::Cancelled)
                | (Self::Gated, Self::BudgetExhausted)
        )
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::BudgetExhausted
        )
    }

    /// A run may hold write leases only while it is actually executing.
    /// `AwaitingApproval` is unbounded, so holding leases across it would
    /// deadlock every other run touching the same targets.
    pub const fn may_hold_write_leases(self) -> bool {
        matches!(self, Self::Leased | Self::Running | Self::Gated)
    }
}

interoperability_string_enum! {
    pub enum ExecutorKind {
        Engine => "engine",
        LocalModel => "local_model",
        Owner => "owner",
        ExternalAgent => "external_agent",
    }
}

interoperability_string_enum! {
    pub enum RiskTier {
        Low => "low",
        Standard => "standard",
        High => "high",
    }
}

interoperability_string_enum! {
    /// A typed constraint between node contracts. Chronological narration is
    /// not a dependency; only these eight relations are.
    pub enum EdgeKind {
        Data => "data",
        Artifact => "artifact",
        State => "state",
        Conflict => "conflict",
        Resource => "resource",
        Policy => "policy",
        Verification => "verification",
        Temporal => "temporal",
    }
}

impl EdgeKind {
    /// Conflict edges are derived from intersecting write scopes at admission,
    /// never authored, so an author cannot omit one by not noticing it.
    pub const fn is_derived_only(self) -> bool {
        matches!(self, Self::Conflict)
    }
}

interoperability_string_enum! {
    pub enum GateId {
        G0 => "g0",
        G1 => "g1",
        G2 => "g2",
        G3 => "g3",
        G4 => "g4",
        G5 => "g5",
        G6 => "g6",
        G7 => "g7",
    }
}

interoperability_string_enum! {
    pub enum GateOutcome {
        Pass => "pass",
        Fail => "fail",
        Blocked => "blocked",
    }
}

interoperability_string_enum! {
    /// Failure is classified before any retry. Blind repetition is not recovery.
    pub enum RunFailureClass {
        Transient => "transient",
        Deterministic => "deterministic",
        Specification => "specification",
        Authorization => "authorization",
        Budget => "budget",
        Conflict => "conflict",
    }
}

impl RunFailureClass {
    /// Only transient infrastructure failure justifies repeating the same
    /// action. Everything else needs new evidence or a different method.
    pub const fn permits_identical_retry(self) -> bool {
        matches!(self, Self::Transient)
    }
}

interoperability_string_enum! {
    pub enum LeaseReleaseReason {
        Completed => "completed",
        AwaitingApproval => "awaiting_approval",
        Cancelled => "cancelled",
        Failed => "failed",
        ExpiredReplaced => "expired_replaced",
    }
}

impl RiskTier {
    /// Gates required at this tier.
    ///
    /// G0 (scope boundary) and G6 (ceiling boundary) are required at every
    /// tier and are never waived. G5 is required only at `High`, because ADR
    /// 002 establishes a single-owner vault with no independent third-party
    /// reviewer to import; see `WORK_ORDER_MODEL.md`.
    pub const fn required_gates(self) -> &'static [GateId] {
        match self {
            Self::Low => &[GateId::G0, GateId::G1, GateId::G2, GateId::G6],
            Self::Standard => &[
                GateId::G0,
                GateId::G1,
                GateId::G2,
                GateId::G3,
                GateId::G4,
                GateId::G6,
            ],
            Self::High => &[
                GateId::G0,
                GateId::G1,
                GateId::G2,
                GateId::G3,
                GateId::G4,
                GateId::G5,
                GateId::G6,
                GateId::G7,
            ],
        }
    }

    pub fn requires(self, gate: GateId) -> bool {
        self.required_gates().contains(&gate)
    }
}

// ---------------------------------------------------------------------------
// Target digests
// ---------------------------------------------------------------------------

/// SHA-256 digest of a stable target URI, lowercase hex.
///
/// Write scope is stored and indexed as a digest so that lease exclusivity and
/// conflict derivation work on a sealed vault without exposing scope detail.
/// Grants permit only exact stable URI equality — never wildcards or prefixes —
/// so digest equality is exactly equivalent to target equality.
pub fn target_digest(target: &StableUri) -> String {
    let mut hasher = Sha256::new();
    hasher.update(target.as_str().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// ---------------------------------------------------------------------------
// Budgets
// ---------------------------------------------------------------------------

/// Enforced ceilings for one Work Order. Ceilings are immutable after
/// admission; remaining balances only decrease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkOrderBudget {
    pub wall_clock_secs: u64,
    pub run_attempts: u64,
    pub model_tokens: u64,
    pub effect_actions: u64,
}

impl WorkOrderBudget {
    pub const fn zero() -> Self {
        Self {
            wall_clock_secs: 0,
            run_attempts: 0,
            model_tokens: 0,
            effect_actions: 0,
        }
    }

    pub const fn is_exhausted(&self) -> bool {
        self.wall_clock_secs == 0
            || self.run_attempts == 0
            || self.model_tokens == 0
            || self.effect_actions == 0
    }

    /// Subtract a spend, failing closed on exhaustion.
    ///
    /// Callers must apply the result inside the same immediate transaction as
    /// the run state transition and event emission. A `CHECK` constraint
    /// prevents a negative row; only the transaction prevents a double spend.
    pub fn checked_spend(&self, spend: &WorkOrderSpend) -> Result<Self, String> {
        Ok(Self {
            wall_clock_secs: self
                .wall_clock_secs
                .checked_sub(spend.wall_clock_secs)
                .ok_or("work-order wall-clock budget exhausted")?,
            run_attempts: self
                .run_attempts
                .checked_sub(spend.run_attempts)
                .ok_or("work-order run-attempt budget exhausted")?,
            model_tokens: self
                .model_tokens
                .checked_sub(spend.model_tokens)
                .ok_or("work-order model-token budget exhausted")?,
            effect_actions: self
                .effect_actions
                .checked_sub(spend.effect_actions)
                .ok_or("work-order effect-action budget exhausted")?,
        })
    }

    fn within(&self, ceiling: &Self) -> bool {
        self.wall_clock_secs <= ceiling.wall_clock_secs
            && self.run_attempts <= ceiling.run_attempts
            && self.model_tokens <= ceiling.model_tokens
            && self.effect_actions <= ceiling.effect_actions
    }
}

/// One increment of budget consumption.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkOrderSpend {
    pub wall_clock_secs: u64,
    pub run_attempts: u64,
    pub model_tokens: u64,
    pub effect_actions: u64,
}

impl WorkOrderSpend {
    pub const fn one_run_attempt() -> Self {
        Self {
            wall_clock_secs: 0,
            run_attempts: 1,
            model_tokens: 0,
            effect_actions: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Work Order
// ---------------------------------------------------------------------------

/// A unit of governed intent.
///
/// Non-goals are recorded because a run that reaches its goal by violating a
/// non-goal has failed. Anchors are recorded because derived knowledge never
/// erases the authority of its evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkOrder {
    pub work_order_id: Uuid,
    pub revision: u64,
    pub work_order_uri: StableUri,
    pub principal: StableUri,
    pub actor: StableUri,
    pub governing_node: StableUri,
    pub goal: String,
    pub non_goals: Vec<String>,
    pub anchors: Vec<StableUri>,
    pub success_criteria: Vec<String>,
    pub prohibited_outcomes: Vec<String>,
    pub budget: WorkOrderBudget,
    pub remaining: WorkOrderBudget,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub idempotency_key: String,
    pub status: WorkOrderStatus,
    pub status_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkOrder {
    pub fn validate(&self) -> Result<(), String> {
        if self.revision == 0 {
            return Err("work-order revision must be positive".into());
        }
        if self.goal.trim().is_empty() {
            return Err("a work order must state a bounded goal".into());
        }
        if self.success_criteria.is_empty() {
            return Err("a work order must state at least one success criterion".into());
        }
        if self.idempotency_key.trim().is_empty() {
            return Err("a work order must carry an idempotency key".into());
        }
        if !self.remaining.within(&self.budget) {
            return Err("remaining budget cannot exceed the declared ceiling".into());
        }
        if self.updated_at < self.created_at {
            return Err("work-order update time cannot precede creation".into());
        }
        let mut seen = BTreeSet::new();
        for anchor in &self.anchors {
            if !seen.insert(anchor.as_str()) {
                return Err("work-order anchors must be unique".into());
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Node contract
// ---------------------------------------------------------------------------

/// One bounded contract within a Work Order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkOrderNode {
    pub node_id: Uuid,
    pub work_order_id: Uuid,
    pub node_uri: StableUri,
    pub purpose: String,
    pub executor_kind: ExecutorKind,
    pub risk_tier: RiskTier,
    pub status: WorkOrderNodeStatus,
    /// Exact stable URIs this contract may read. Resolved against a Context Grant.
    pub read_scope: Vec<StableUri>,
    /// Exact stable URIs this contract may write. Resolved against a Tool Grant.
    pub write_scope: Vec<StableUri>,
    pub inputs: Vec<StableUri>,
    pub timeout_secs: u32,
    pub max_attempts: u32,
    pub authorizing_grant_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkOrderNode {
    pub fn validate(&self) -> Result<(), String> {
        if self.purpose.trim().is_empty() {
            return Err("a node contract must state a single purpose".into());
        }
        if self.timeout_secs == 0 {
            return Err("a node contract must declare a positive timeout".into());
        }
        if self.max_attempts == 0 {
            return Err("a node contract must permit at least one attempt".into());
        }
        let mut writes = BTreeSet::new();
        for target in &self.write_scope {
            if !writes.insert(target.as_str()) {
                return Err("declared write scope must not repeat a target".into());
            }
        }
        let mut reads = BTreeSet::new();
        for target in &self.read_scope {
            if !reads.insert(target.as_str()) {
                return Err("declared read scope must not repeat a target".into());
            }
        }
        Ok(())
    }

    /// Digests of the declared write scope, in stable order.
    pub fn write_target_digests(&self) -> Vec<String> {
        let mut digests: Vec<String> = self.write_scope.iter().map(target_digest).collect();
        digests.sort();
        digests.dedup();
        digests
    }

    /// True when every declared write target is present in the grant's exact
    /// target set. This is gate G0's containment test.
    pub fn write_scope_within(&self, grant_targets: &[StableUri]) -> bool {
        let permitted: BTreeSet<&str> = grant_targets.iter().map(StableUri::as_str).collect();
        self.write_scope
            .iter()
            .all(|target| permitted.contains(target.as_str()))
    }

    /// Write targets that fall outside the grant's target set, for a precise
    /// rejection reason rather than a bare denial.
    pub fn write_scope_violations(&self, grant_targets: &[StableUri]) -> Vec<StableUri> {
        let permitted: BTreeSet<&str> = grant_targets.iter().map(StableUri::as_str).collect();
        self.write_scope
            .iter()
            .filter(|target| !permitted.contains(target.as_str()))
            .cloned()
            .collect()
    }

    /// True when this contract's write scope intersects another's, which
    /// requires a derived conflict edge between them.
    pub fn write_scope_intersects(&self, other: &Self) -> bool {
        let mine: BTreeSet<&str> = self.write_scope.iter().map(StableUri::as_str).collect();
        other
            .write_scope
            .iter()
            .any(|target| mine.contains(target.as_str()))
    }
}

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkOrderEdge {
    pub edge_id: Uuid,
    pub work_order_id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub kind: EdgeKind,
    pub derived: bool,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WorkOrderEdge {
    pub fn validate(&self) -> Result<(), String> {
        if self.from_node_id == self.to_node_id {
            return Err("a node contract cannot depend on itself".into());
        }
        if self.kind.is_derived_only() && !self.derived {
            return Err("conflict edges are derived from write scope, not authored".into());
        }
        Ok(())
    }
}

/// Derive the conflict edges implied by intersecting write scopes.
///
/// Every unordered pair of contracts whose write scopes intersect acquires an
/// edge, whether or not the author noticed the overlap. This is the primary
/// defense against two runs corrupting one target in the owner's vault.
pub fn derive_conflict_edges(
    work_order_id: Uuid,
    nodes: &[WorkOrderNode],
    now: DateTime<Utc>,
) -> Vec<WorkOrderEdge> {
    let mut edges = Vec::new();
    for (index, left) in nodes.iter().enumerate() {
        for right in nodes.iter().skip(index + 1) {
            if left.write_scope_intersects(right) {
                edges.push(WorkOrderEdge {
                    edge_id: Uuid::now_v7(),
                    work_order_id,
                    from_node_id: left.node_id,
                    to_node_id: right.node_id,
                    kind: EdgeKind::Conflict,
                    derived: true,
                    detail: Some("intersecting declared write scope".into()),
                    created_at: now,
                });
            }
        }
    }
    edges
}

/// Detect a dependency cycle over non-conflict edges.
///
/// Conflict edges are mutual-exclusion constraints rather than ordering
/// constraints, so they are excluded: two contracts that conflict are
/// serialized by leases, not by precedence, and treating the pair as a cycle
/// would reject every legitimate overlapping Work Order.
pub fn find_dependency_cycle(
    nodes: &[WorkOrderNode],
    edges: &[WorkOrderEdge],
) -> Option<Vec<Uuid>> {
    use std::collections::HashMap;

    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for node in nodes {
        adjacency.entry(node.node_id).or_default();
    }
    for edge in edges.iter().filter(|e| e.kind != EdgeKind::Conflict) {
        adjacency
            .entry(edge.from_node_id)
            .or_default()
            .push(edge.to_node_id);
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        InProgress,
        Done,
    }

    let mut marks: HashMap<Uuid, Mark> =
        adjacency.keys().map(|id| (*id, Mark::Unvisited)).collect();
    let mut stack = Vec::new();

    fn visit(
        current: Uuid,
        adjacency: &HashMap<Uuid, Vec<Uuid>>,
        marks: &mut HashMap<Uuid, Mark>,
        stack: &mut Vec<Uuid>,
    ) -> Option<Vec<Uuid>> {
        marks.insert(current, Mark::InProgress);
        stack.push(current);
        for next in adjacency.get(&current).into_iter().flatten() {
            match marks.get(next).copied().unwrap_or(Mark::Unvisited) {
                Mark::InProgress => {
                    let start = stack.iter().position(|id| id == next).unwrap_or(0);
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(*next);
                    return Some(cycle);
                }
                Mark::Unvisited => {
                    if let Some(cycle) = visit(*next, adjacency, marks, stack) {
                        return Some(cycle);
                    }
                }
                Mark::Done => {}
            }
        }
        stack.pop();
        marks.insert(current, Mark::Done);
        None
    }

    let ids: Vec<Uuid> = adjacency.keys().copied().collect();
    for id in ids {
        if marks.get(&id).copied().unwrap_or(Mark::Unvisited) == Mark::Unvisited {
            if let Some(cycle) = visit(id, &adjacency, &mut marks, &mut stack) {
                return Some(cycle);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Agent run
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRun {
    pub run_id: Uuid,
    pub run_uri: StableUri,
    pub work_order_id: Uuid,
    pub node_id: Uuid,
    pub attempt_no: u32,
    pub status: AgentRunStatus,
    pub failure_class: Option<RunFailureClass>,
    pub principal: StableUri,
    /// The acting actor. Gate G5 requires an evaluator distinct from this.
    pub actor: StableUri,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentRun {
    pub fn validate(&self) -> Result<(), String> {
        if self.attempt_no == 0 {
            return Err("agent-run attempt numbers start at one".into());
        }
        if self.status.is_terminal() && self.ended_at.is_none() {
            return Err("a terminal run must record an end time".into());
        }
        if matches!(
            self.status,
            AgentRunStatus::Failed | AgentRunStatus::BudgetExhausted
        ) && self.failure_class.is_none()
        {
            return Err("a failed run must record its failure class".into());
        }
        if self.updated_at < self.created_at {
            return Err("agent-run update time cannot precede creation".into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Write leases
// ---------------------------------------------------------------------------

/// An exclusive claim on one write target, held only while a run executes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WriteLease {
    pub lease_id: Uuid,
    pub run_id: Uuid,
    pub work_order_id: Uuid,
    pub target_digest: String,
    pub governing_node: StableUri,
    pub attempt_no: u32,
    pub claimed_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub release_reason: Option<LeaseReleaseReason>,
    pub created_at: DateTime<Utc>,
}

impl WriteLease {
    pub fn validate(&self) -> Result<(), String> {
        if !is_sha256_hex(&self.target_digest) {
            return Err("write-lease target digest must be lowercase SHA-256 hex".into());
        }
        if self.attempt_no == 0 {
            return Err("write-lease claim attempts start at one".into());
        }
        if self.lease_expires_at <= self.claimed_at {
            return Err("write-lease interval must be non-empty".into());
        }
        if self.lease_expires_at - self.claimed_at > Duration::seconds(MAX_WRITE_LEASE_SECS) {
            return Err("write leases are bounded to one hour".into());
        }
        if self.released_at.is_some() != self.release_reason.is_some() {
            return Err("a released write lease must record its reason".into());
        }
        if let Some(released_at) = self.released_at {
            if released_at < self.claimed_at {
                return Err("write-lease release cannot precede the claim".into());
            }
        }
        Ok(())
    }

    pub const fn is_active(&self) -> bool {
        self.released_at.is_none()
    }

    /// The interval is half-open: completion at or after expiry fails closed.
    pub fn is_expired_at(&self, at: DateTime<Utc>) -> bool {
        at >= self.lease_expires_at
    }

    pub fn covers(&self, at: DateTime<Utc>) -> bool {
        self.is_active() && at >= self.claimed_at && at < self.lease_expires_at
    }
}

// ---------------------------------------------------------------------------
// Gate evidence
// ---------------------------------------------------------------------------

/// Immutable evidence that one gate was evaluated, not a boolean flag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateResult {
    pub result_id: Uuid,
    pub run_id: Uuid,
    pub work_order_id: Uuid,
    pub gate: GateId,
    pub outcome: GateOutcome,
    pub evaluator_actor: StableUri,
    pub evidence_digest: String,
    pub detail: Option<String>,
    pub evaluated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl GateResult {
    pub fn validate(&self, run: &AgentRun) -> Result<(), String> {
        if !is_sha256_hex(&self.evidence_digest) {
            return Err("gate evidence digest must be lowercase SHA-256 hex".into());
        }
        if self.run_id != run.run_id {
            return Err("gate result must reference its own run".into());
        }
        // Holds at every risk tier: independence is narrower than "a third
        // party reviewed it", but it is absolute.
        if self.gate == GateId::G5 && self.evaluator_actor == run.actor {
            return Err("a run cannot satisfy its own G5".into());
        }
        Ok(())
    }
}

/// Gates still outstanding for a run at its contract's risk tier.
pub fn missing_gates(tier: RiskTier, results: &[GateResult]) -> Vec<GateId> {
    tier.required_gates()
        .iter()
        .copied()
        .filter(|gate| {
            !results
                .iter()
                .any(|result| result.gate == *gate && result.outcome == GateOutcome::Pass)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunArtifact {
    pub artifact_id: Uuid,
    pub artifact_uri: StableUri,
    pub run_id: Uuid,
    pub work_order_id: Uuid,
    pub artifact_kind: String,
    pub content_digest: String,
    pub schema: Option<SchemaReference>,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub provenance: Vec<ProvenanceReference>,
    pub created_at: DateTime<Utc>,
}

/// Upper bound on provenance references attached to one run artifact.
///
/// Keeps a single record_artifact request from amplifying into an unbounded
/// number of provenance rows in the same transaction.
pub const MAX_ARTIFACT_PROVENANCE: usize = 32;

impl RunArtifact {
    pub fn validate(&self) -> Result<(), String> {
        if !is_sha256_hex(&self.content_digest) {
            return Err("artifact content digest must be lowercase SHA-256 hex".into());
        }
        if self.artifact_kind.trim().is_empty() {
            return Err("an artifact must declare its kind".into());
        }
        // Derived knowledge never erases the authority of its evidence.
        if self.provenance.is_empty() {
            return Err("an artifact must reference the evidence it derives from".into());
        }
        if self.provenance.len() > MAX_ARTIFACT_PROVENANCE {
            return Err(format!(
                "an artifact may cite at most {MAX_ARTIFACT_PROVENANCE} provenance references"
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Idempotent commit results
// ---------------------------------------------------------------------------

/// Result of an atomic Work Order commit and its event.
#[derive(Debug, Clone, PartialEq)]
pub struct IdempotentWorkOrderCommit {
    pub work_order: WorkOrder,
    pub event: super::EventEnvelope,
    pub replayed: bool,
}

/// Result of an atomic Agent Run commit and its event.
#[derive(Debug, Clone, PartialEq)]
pub struct IdempotentAgentRunCommit {
    pub run: AgentRun,
    pub event: super::EventEnvelope,
    pub replayed: bool,
}

// ---------------------------------------------------------------------------
// Portable export
// ---------------------------------------------------------------------------

/// Wire format version for a governed execution-graph export.
pub const WORK_ORDER_EXPORT_FORMAT_V1: &str = "work-order-export-v1";

/// An artifact together with the bytes it attests to.
///
/// The content travels with the record. An export that carried only digests
/// would let a restored vault hold provenance pointing at content that no
/// longer exists anywhere — a portability guarantee in name only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportedArtifact {
    pub artifact: RunArtifact,
    /// Base64-encoded content. Verified against `artifact.content_digest` on
    /// both export and restore.
    pub content_base64: String,
}

/// A complete, portable Work Order: the order, its contracts and typed edges,
/// every run attempt, all gate evidence, and every artifact with its bytes and
/// provenance.
///
/// Constitutional law 11 requires that a user can leave with their data. For
/// this capability that means the whole governed graph, not a summary: gate
/// evidence without the runs it judged, or provenance without the artifacts it
/// describes, would restore into something that cannot be audited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkOrderExport {
    pub format_version: String,
    pub exported_at: DateTime<Utc>,
    pub work_order: WorkOrder,
    pub nodes: Vec<WorkOrderNode>,
    pub edges: Vec<WorkOrderEdge>,
    pub runs: Vec<AgentRun>,
    pub gate_results: Vec<GateResult>,
    pub artifacts: Vec<ExportedArtifact>,
}

impl WorkOrderExport {
    /// Check the export is internally consistent before it is trusted.
    ///
    /// Applied on restore as well as export, because an export file is
    /// untrusted input on the way back in: it may have been edited, truncated,
    /// or produced by a different implementation.
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != WORK_ORDER_EXPORT_FORMAT_V1 {
            return Err(format!(
                "unsupported export format: {}",
                self.format_version
            ));
        }
        let order_id = self.work_order.work_order_id;

        for node in &self.nodes {
            if node.work_order_id != order_id {
                return Err("a node contract belongs to a different work order".into());
            }
        }
        let node_ids: Vec<_> = self.nodes.iter().map(|node| node.node_id).collect();
        for edge in &self.edges {
            if edge.work_order_id != order_id {
                return Err("an edge belongs to a different work order".into());
            }
            // A dangling edge would restore a graph whose readiness can never
            // be computed.
            if !node_ids.contains(&edge.from_node_id) || !node_ids.contains(&edge.to_node_id) {
                return Err("an edge references a node contract not present in the export".into());
            }
        }
        for run in &self.runs {
            if run.work_order_id != order_id {
                return Err("a run belongs to a different work order".into());
            }
            if !node_ids.contains(&run.node_id) {
                return Err("a run references a node contract not present in the export".into());
            }
        }

        let run_ids: Vec<_> = self.runs.iter().map(|run| run.run_id).collect();
        for result in &self.gate_results {
            if !run_ids.contains(&result.run_id) {
                return Err("gate evidence references a run not present in the export".into());
            }
            // The independence invariant must survive the round trip; a
            // restored vault must not be able to hold evidence that a run
            // reviewed itself.
            if let Some(run) = self.runs.iter().find(|run| run.run_id == result.run_id) {
                result.validate(run)?;
            }
        }

        for exported in &self.artifacts {
            if exported.artifact.work_order_id != order_id {
                return Err("an artifact belongs to a different work order".into());
            }
            if !run_ids.contains(&exported.artifact.run_id) {
                return Err("an artifact references a run not present in the export".into());
            }
            exported.artifact.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(value: &str) -> StableUri {
        StableUri::parse(value).expect("valid stable URI")
    }

    fn node(id: Uuid, writes: &[&str], tier: RiskTier) -> WorkOrderNode {
        let now = Utc::now();
        WorkOrderNode {
            node_id: id,
            work_order_id: Uuid::nil(),
            node_uri: uri("mindvault://schemas/placeholder"),
            purpose: "port one module".into(),
            executor_kind: ExecutorKind::Engine,
            risk_tier: tier,
            status: WorkOrderNodeStatus::Pending,
            read_scope: Vec::new(),
            write_scope: writes.iter().map(|value| uri(value)).collect(),
            inputs: Vec::new(),
            timeout_secs: 600,
            max_attempts: 3,
            authorizing_grant_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn write_scope_outside_the_grant_is_rejected_with_the_offending_targets() {
        let contract = node(
            Uuid::now_v7(),
            &["mindvault://schemas/alpha", "mindvault://schemas/beta"],
            RiskTier::Standard,
        );
        let granted = vec![uri("mindvault://schemas/alpha")];

        assert!(!contract.write_scope_within(&granted));
        let violations = contract.write_scope_violations(&granted);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].as_str(), "mindvault://schemas/beta");

        let full = vec![
            uri("mindvault://schemas/alpha"),
            uri("mindvault://schemas/beta"),
        ];
        assert!(contract.write_scope_within(&full));
        assert!(contract.write_scope_violations(&full).is_empty());
    }

    #[test]
    fn intersecting_write_scopes_always_derive_a_conflict_edge() {
        let left = node(
            Uuid::now_v7(),
            &["mindvault://schemas/shared"],
            RiskTier::Low,
        );
        let right = node(
            Uuid::now_v7(),
            &["mindvault://schemas/shared", "mindvault://schemas/other"],
            RiskTier::Low,
        );
        let apart = node(
            Uuid::now_v7(),
            &["mindvault://schemas/apart"],
            RiskTier::Low,
        );

        let edges = derive_conflict_edges(Uuid::nil(), &[left, right, apart], Utc::now());
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Conflict);
        assert!(
            edges[0].derived,
            "conflict edges are derived, never authored"
        );
        assert!(edges[0].validate().is_ok());
    }

    #[test]
    fn an_authored_conflict_edge_is_rejected() {
        let mut edge = WorkOrderEdge {
            edge_id: Uuid::now_v7(),
            work_order_id: Uuid::nil(),
            from_node_id: Uuid::now_v7(),
            to_node_id: Uuid::now_v7(),
            kind: EdgeKind::Conflict,
            derived: false,
            detail: None,
            created_at: Utc::now(),
        };
        assert!(edge.validate().is_err());
        edge.derived = true;
        assert!(edge.validate().is_ok());
    }

    #[test]
    fn conflict_edges_do_not_count_as_dependency_cycles() {
        let left_id = Uuid::now_v7();
        let right_id = Uuid::now_v7();
        let left = node(left_id, &["mindvault://schemas/shared"], RiskTier::Low);
        let right = node(right_id, &["mindvault://schemas/shared"], RiskTier::Low);
        let nodes = vec![left, right];

        // Mutual conflict edges are exclusion, not precedence.
        let mut edges = derive_conflict_edges(Uuid::nil(), &nodes, Utc::now());
        edges.push(WorkOrderEdge {
            edge_id: Uuid::now_v7(),
            work_order_id: Uuid::nil(),
            from_node_id: right_id,
            to_node_id: left_id,
            kind: EdgeKind::Conflict,
            derived: true,
            detail: None,
            created_at: Utc::now(),
        });
        assert!(find_dependency_cycle(&nodes, &edges).is_none());

        // A genuine data cycle is still caught.
        let cyclic = vec![
            WorkOrderEdge {
                edge_id: Uuid::now_v7(),
                work_order_id: Uuid::nil(),
                from_node_id: left_id,
                to_node_id: right_id,
                kind: EdgeKind::Data,
                derived: false,
                detail: None,
                created_at: Utc::now(),
            },
            WorkOrderEdge {
                edge_id: Uuid::now_v7(),
                work_order_id: Uuid::nil(),
                from_node_id: right_id,
                to_node_id: left_id,
                kind: EdgeKind::Data,
                derived: false,
                detail: None,
                created_at: Utc::now(),
            },
        ];
        assert!(find_dependency_cycle(&nodes, &cyclic).is_some());
    }

    #[test]
    fn approval_is_unbounded_and_holds_no_leases() {
        assert!(!AgentRunStatus::AwaitingApproval.may_hold_write_leases());
        assert!(AgentRunStatus::Running.may_hold_write_leases());
        // Approval returns to the ready set so leases are re-acquired under a
        // fresh conflict check, rather than resuming with a stale write set.
        assert!(AgentRunStatus::AwaitingApproval.can_transition_to(AgentRunStatus::Ready));
        assert!(!AgentRunStatus::AwaitingApproval.can_transition_to(AgentRunStatus::Running));
        assert!(!AgentRunStatus::AwaitingApproval.is_terminal());
    }

    #[test]
    fn write_leases_are_bounded_to_one_hour_and_half_open() {
        let claimed = Utc::now();
        let mut lease = WriteLease {
            lease_id: Uuid::now_v7(),
            run_id: Uuid::now_v7(),
            work_order_id: Uuid::nil(),
            target_digest: target_digest(&uri("mindvault://schemas/alpha")),
            governing_node: uri("mindvault://schemas/placeholder"),
            attempt_no: 1,
            claimed_at: claimed,
            lease_expires_at: claimed + Duration::minutes(60),
            released_at: None,
            release_reason: None,
            created_at: claimed,
        };
        assert!(lease.validate().is_ok());

        lease.lease_expires_at = claimed + Duration::minutes(61);
        assert!(lease.validate().is_err());

        lease.lease_expires_at = claimed + Duration::minutes(30);
        assert!(lease.validate().is_ok());
        assert!(lease.covers(claimed));
        // Half-open: the expiry instant itself is outside the lease.
        assert!(!lease.covers(lease.lease_expires_at));
        assert!(lease.is_expired_at(lease.lease_expires_at));
    }

    #[test]
    fn g0_and_g6_are_required_at_every_tier_and_g5_only_at_high() {
        for tier in [RiskTier::Low, RiskTier::Standard, RiskTier::High] {
            assert!(tier.requires(GateId::G0), "G0 is the scope boundary");
            assert!(tier.requires(GateId::G6), "G6 is the ceiling boundary");
        }
        assert!(!RiskTier::Low.requires(GateId::G5));
        assert!(!RiskTier::Standard.requires(GateId::G5));
        assert!(RiskTier::High.requires(GateId::G5));
        assert!(RiskTier::High.requires(GateId::G7));
    }

    #[test]
    fn a_run_cannot_satisfy_its_own_g5() {
        let now = Utc::now();
        let run = AgentRun {
            run_id: Uuid::now_v7(),
            run_uri: uri("mindvault://schemas/placeholder"),
            work_order_id: Uuid::nil(),
            node_id: Uuid::now_v7(),
            attempt_no: 1,
            status: AgentRunStatus::Gated,
            failure_class: None,
            principal: uri("mindvault://schemas/owner"),
            actor: uri("mindvault://schemas/agent-a"),
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            started_at: Some(now),
            ended_at: None,
            created_at: now,
            updated_at: now,
        };
        let mut result = GateResult {
            result_id: Uuid::now_v7(),
            run_id: run.run_id,
            work_order_id: Uuid::nil(),
            gate: GateId::G5,
            outcome: GateOutcome::Pass,
            evaluator_actor: uri("mindvault://schemas/agent-a"),
            evidence_digest: "a".repeat(64),
            detail: None,
            evaluated_at: now,
            created_at: now,
        };
        assert!(result.validate(&run).is_err());

        result.evaluator_actor = uri("mindvault://schemas/owner");
        assert!(result.validate(&run).is_ok());
    }

    #[test]
    fn budget_exhaustion_fails_closed_rather_than_reducing_scope() {
        let budget = WorkOrderBudget {
            wall_clock_secs: 60,
            run_attempts: 1,
            model_tokens: 100,
            effect_actions: 0,
        };
        assert!(
            budget.is_exhausted(),
            "a zero effect-action ceiling is exhausted"
        );

        let spend = WorkOrderSpend::one_run_attempt();
        let after = budget.checked_spend(&spend).expect("first attempt fits");
        assert_eq!(after.run_attempts, 0);
        // The second attempt must fail rather than silently proceed.
        assert!(after.checked_spend(&spend).is_err());
    }

    #[test]
    fn only_transient_failure_permits_an_identical_retry() {
        assert!(RunFailureClass::Transient.permits_identical_retry());
        for class in [
            RunFailureClass::Deterministic,
            RunFailureClass::Specification,
            RunFailureClass::Authorization,
            RunFailureClass::Budget,
            RunFailureClass::Conflict,
        ] {
            assert!(!class.permits_identical_retry());
        }
    }

    #[test]
    fn target_digests_are_exact_and_leak_no_scope() {
        let alpha = target_digest(&uri("mindvault://schemas/alpha"));
        let beta = target_digest(&uri("mindvault://schemas/beta"));
        assert_ne!(alpha, beta);
        assert_eq!(alpha, target_digest(&uri("mindvault://schemas/alpha")));
        assert!(is_sha256_hex(&alpha));
        assert!(!alpha.contains("alpha"));
    }

    #[test]
    fn missing_gates_reports_only_unsatisfied_requirements() {
        let now = Utc::now();
        let passing = |gate: GateId| GateResult {
            result_id: Uuid::now_v7(),
            run_id: Uuid::nil(),
            work_order_id: Uuid::nil(),
            gate,
            outcome: GateOutcome::Pass,
            evaluator_actor: uri("mindvault://schemas/owner"),
            evidence_digest: "b".repeat(64),
            detail: None,
            evaluated_at: now,
            created_at: now,
        };

        let results = vec![
            passing(GateId::G0),
            passing(GateId::G1),
            passing(GateId::G2),
        ];
        assert_eq!(missing_gates(RiskTier::Low, &results), vec![GateId::G6]);

        // A failing result does not satisfy a requirement.
        let mut failed = passing(GateId::G6);
        failed.outcome = GateOutcome::Fail;
        let mut with_failure = results.clone();
        with_failure.push(failed);
        assert_eq!(
            missing_gates(RiskTier::Low, &with_failure),
            vec![GateId::G6]
        );
    }

    #[test]
    fn artifact_provenance_is_required_and_bounded() {
        let mut artifact = RunArtifact {
            artifact_id: Uuid::nil(),
            artifact_uri: uri("mindvault://schemas/artifact"),
            run_id: Uuid::nil(),
            work_order_id: Uuid::nil(),
            artifact_kind: "summary".into(),
            content_digest: "a".repeat(64),
            schema: None,
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: Vec::new(),
            created_at: Utc::now(),
        };
        assert!(
            artifact.validate().unwrap_err().contains("evidence"),
            "empty provenance must fail closed"
        );

        artifact.provenance = (0..=MAX_ARTIFACT_PROVENANCE)
            .map(|i| ProvenanceReference {
                resource: uri(&format!("mindvault://schemas/source-{i}")),
                relation: ProvenanceRelation::WasDerivedFrom,
            })
            .collect();
        assert!(
            artifact.validate().unwrap_err().contains("at most"),
            "provenance above the ceiling must be refused"
        );

        artifact.provenance.truncate(MAX_ARTIFACT_PROVENANCE);
        artifact.validate().expect("ceiling inclusive");
    }
}
