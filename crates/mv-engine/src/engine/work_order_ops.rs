//! Governed agent execution graph operations.
//!
//! Contract: `docs/architecture/WORK_ORDER_MODEL.md`.
//! Isolation: `docs/architecture/EXECUTION_ISOLATION_MODEL.md`.
//! Decision: `docs/adr/012-governed-agent-execution-graph.md`.
//!
//! Admission resolves every declared write target against an effective Tool
//! Grant before any run can exist. Scope is enforced at the grant layer, not by
//! instruction text, because `PROTOCOL_BOUNDARIES.md` treats external tool
//! descriptions as untrusted.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use chrono::{Duration, Utc};
use mv_core::*;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use super::MindVaultEngine;
use crate::admission::{admit_effect, EffectOutcome, EffectRefusal, EffectRequest};

/// Upper bound on run attempts carried in one export.
///
/// This is the store's own maximum for a run query, not a policy choice here. A
/// Work Order's budget caps `run_attempts` far below it, so reaching this bound
/// means the data is inconsistent with its own budget — and an export that
/// quietly returned the first 1000 runs would present a partial history as a
/// complete one. [`MindVaultEngine::export_work_order`] refuses instead.
const WORK_ORDER_EXPORT_RUN_LIMIT: usize = 1000;

/// Outcome of starting a run: the record, any leases taken, and whether the
/// owner still has to authorize it.
///
/// `awaiting_approval` is surfaced explicitly so a caller cannot mistake a
/// parked run for a failed one. It is the human-led default working correctly.
#[derive(Debug, Clone)]
pub struct StartedRun {
    pub run: AgentRun,
    pub leases: Vec<WriteLease>,
    pub awaiting_approval: bool,
}

/// What a restore actually wrote, so a caller can verify rather than assume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSummary {
    pub work_order_id: Uuid,
    pub nodes: usize,
    pub edges: usize,
    pub runs: usize,
    pub gate_results: usize,
    pub artifacts: usize,
}

/// A node contract proposed for admission, before grants are resolved.
#[derive(Debug, Clone)]
pub struct ProposedNode {
    pub purpose: String,
    pub executor_kind: ExecutorKind,
    pub risk_tier: RiskTier,
    pub read_scope: Vec<StableUri>,
    pub write_scope: Vec<StableUri>,
    pub inputs: Vec<StableUri>,
    pub timeout_secs: u32,
    pub max_attempts: u32,
}

/// An authored (non-conflict) edge proposed for admission, by node index.
#[derive(Debug, Clone, Copy)]
pub struct ProposedEdge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

/// A Work Order proposed for admission.
#[derive(Debug, Clone)]
pub struct ProposedWorkOrder {
    pub goal: String,
    pub non_goals: Vec<String>,
    pub anchors: Vec<StableUri>,
    pub success_criteria: Vec<String>,
    pub prohibited_outcomes: Vec<String>,
    pub budget: WorkOrderBudget,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub idempotency_key: String,
    pub nodes: Vec<ProposedNode>,
    pub edges: Vec<ProposedEdge>,
}

/// Why a Work Order was refused admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRefusal {
    /// One or more contracts declared write targets outside their Tool Grant.
    /// This is gate G0. The offending targets are named so the caller can fix
    /// the request rather than guess.
    WriteScopeOutsideGrant {
        node_purpose: String,
        targets: Vec<String>,
    },
    /// A contract declared read targets outside its Context Grant.
    ReadScopeOutsideGrant {
        node_purpose: String,
        targets: Vec<String>,
    },
    /// The dependency graph is cyclic across non-conflict edges.
    DependencyCycle { contracts: usize },
    /// The proposal is malformed.
    Invalid { reason: String },
}

impl std::fmt::Display for AdmissionRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WriteScopeOutsideGrant {
                node_purpose,
                targets,
            } => write!(
                formatter,
                "contract '{node_purpose}' declares write targets outside its Tool Grant: {}",
                targets.join(", ")
            ),
            Self::ReadScopeOutsideGrant {
                node_purpose,
                targets,
            } => write!(
                formatter,
                "contract '{node_purpose}' declares read targets outside its Context Grant: {}",
                targets.join(", ")
            ),
            Self::DependencyCycle { contracts } => write!(
                formatter,
                "dependency graph is cyclic across {contracts} contracts"
            ),
            Self::Invalid { reason } => write!(formatter, "invalid work order: {reason}"),
        }
    }
}

/// Outcome of asking a run whether it may proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunReadiness {
    Ready,
    /// Another run holds a write lease on one of this run's targets.
    BlockedByConflict {
        targets: usize,
    },
    /// A predecessor has not satisfied every gate required of it.
    BlockedByVerification {
        predecessor: Uuid,
    },
    /// A predecessor has not completed.
    BlockedByPredecessor {
        predecessor: Uuid,
    },
    /// Owner approval is outstanding.
    BlockedByApproval,
}

impl RunReadiness {
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

impl MindVaultEngine {
    /// Admit a Work Order, resolving every declared scope against an effective
    /// grant before anything becomes schedulable.
    ///
    /// Returns `Ok(Err(refusal))` when the proposal is well-formed but not
    /// authorized: refusal is a normal governed outcome, not an exception.
    pub async fn admit_work_order(
        &self,
        principal: &StableUri,
        actor: &StableUri,
        proposal: &ProposedWorkOrder,
    ) -> MvResult<Result<WorkOrder, AdmissionRefusal>> {
        if proposal.nodes.is_empty() {
            return Ok(Err(AdmissionRefusal::Invalid {
                reason: "a work order must contain at least one node contract".into(),
            }));
        }

        let local_node_id = self.store.nodes.local_context_node_id().await?;
        let governing_node = StableUri::node(local_node_id);
        let now = Utc::now();
        let work_order_id = Uuid::now_v7();

        // Resolve each contract's declared scope. Gate G0 runs here, before any
        // AgentRun exists, so an over-scoped contract never becomes schedulable.
        let mut nodes = Vec::with_capacity(proposal.nodes.len());
        for proposed in &proposal.nodes {
            let node_id = Uuid::now_v7();
            let contract = WorkOrderNode {
                node_id,
                work_order_id,
                node_uri: StableUri::work_order_node(local_node_id, work_order_id, node_id),
                purpose: proposed.purpose.clone(),
                executor_kind: proposed.executor_kind,
                risk_tier: proposed.risk_tier,
                status: WorkOrderNodeStatus::Pending,
                read_scope: proposed.read_scope.clone(),
                write_scope: proposed.write_scope.clone(),
                inputs: proposed.inputs.clone(),
                timeout_secs: proposed.timeout_secs,
                max_attempts: proposed.max_attempts,
                authorizing_grant_id: None,
                created_at: now,
                updated_at: now,
            };
            if let Err(reason) = contract.validate() {
                return Ok(Err(AdmissionRefusal::Invalid { reason }));
            }

            match self
                .resolve_contract_scope(actor, &contract, proposal, now)
                .await?
            {
                Ok(grant_id) => nodes.push(WorkOrderNode {
                    authorizing_grant_id: grant_id,
                    ..contract
                }),
                Err(refusal) => return Ok(Err(refusal)),
            }
        }

        // Conflict edges are derived, never authored: an author who did not
        // notice an overlap still gets the guard.
        let mut edges = derive_conflict_edges(work_order_id, &nodes, now);
        for proposed in &proposal.edges {
            let (Some(from), Some(to)) = (nodes.get(proposed.from), nodes.get(proposed.to)) else {
                return Ok(Err(AdmissionRefusal::Invalid {
                    reason: "edge references a contract outside the work order".into(),
                }));
            };
            if proposed.kind.is_derived_only() {
                return Ok(Err(AdmissionRefusal::Invalid {
                    reason: "conflict edges are derived from write scope, not authored".into(),
                }));
            }
            let edge = WorkOrderEdge {
                edge_id: Uuid::now_v7(),
                work_order_id,
                from_node_id: from.node_id,
                to_node_id: to.node_id,
                kind: proposed.kind,
                derived: false,
                detail: None,
                created_at: now,
            };
            if let Err(reason) = edge.validate() {
                return Ok(Err(AdmissionRefusal::Invalid { reason }));
            }
            edges.push(edge);
        }

        if let Some(cycle) = find_dependency_cycle(&nodes, &edges) {
            return Ok(Err(AdmissionRefusal::DependencyCycle {
                contracts: cycle.len(),
            }));
        }

        let work_order = WorkOrder {
            work_order_id,
            revision: 1,
            work_order_uri: StableUri::work_order(local_node_id, work_order_id),
            principal: principal.clone(),
            actor: actor.clone(),
            governing_node,
            goal: proposal.goal.clone(),
            non_goals: proposal.non_goals.clone(),
            anchors: proposal.anchors.clone(),
            success_criteria: proposal.success_criteria.clone(),
            prohibited_outcomes: proposal.prohibited_outcomes.clone(),
            budget: proposal.budget,
            remaining: proposal.budget,
            sensitivity: proposal.sensitivity,
            retention: proposal.retention,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: proposal.idempotency_key.clone(),
            status: WorkOrderStatus::Admitted,
            status_reason: None,
            created_at: now,
            updated_at: now,
        };
        if let Err(reason) = work_order.validate() {
            return Ok(Err(AdmissionRefusal::Invalid { reason }));
        }

        let event =
            Self::work_order_admitted_event(local_node_id, &work_order, nodes.len(), edges.len())?;
        let commit = self
            .store
            .nodes
            .commit_work_order_with_event(&work_order, &nodes, &edges, &event)
            .await?;
        Ok(Ok(commit.work_order))
    }

    /// Resolve one contract's declared read and write scope against effective
    /// grants. Returns the authorizing Tool Grant when the write scope is
    /// non-empty.
    async fn resolve_contract_scope(
        &self,
        actor: &StableUri,
        contract: &WorkOrderNode,
        proposal: &ProposedWorkOrder,
        at: chrono::DateTime<Utc>,
    ) -> MvResult<Result<Option<Uuid>, AdmissionRefusal>> {
        let mut read_violations = Vec::new();
        for target in &contract.read_scope {
            let grant = self
                .store
                .nodes
                .find_authorizing_grant(GrantQuery {
                    grantee: actor,
                    kind: AuthorityGrantKind::Context,
                    target,
                    capability: ContextCapability::Read,
                    sensitivity: proposal.sensitivity,
                    retention: proposal.retention,
                    at,
                })
                .await?;
            if grant.is_none() {
                read_violations.push(target.as_str().to_string());
            }
        }
        if !read_violations.is_empty() {
            return Ok(Err(AdmissionRefusal::ReadScopeOutsideGrant {
                node_purpose: contract.purpose.clone(),
                targets: read_violations,
            }));
        }

        let mut write_violations = Vec::new();
        let mut authorizing = None;
        for target in &contract.write_scope {
            // A Tool Grant, never a Context Grant: reading never implies the
            // authority to mutate.
            let grant = self
                .store
                .nodes
                .find_authorizing_grant(GrantQuery {
                    grantee: actor,
                    kind: AuthorityGrantKind::Tool,
                    target,
                    capability: ContextCapability::Command,
                    sensitivity: proposal.sensitivity,
                    retention: proposal.retention,
                    at,
                })
                .await?;
            match grant {
                Some(grant) => authorizing.get_or_insert(grant.grant_id),
                None => {
                    write_violations.push(target.as_str().to_string());
                    continue;
                }
            };
        }
        if !write_violations.is_empty() {
            return Ok(Err(AdmissionRefusal::WriteScopeOutsideGrant {
                node_purpose: contract.purpose.clone(),
                targets: write_violations,
            }));
        }

        Ok(Ok(authorizing))
    }

    /// Evaluate whether a run's inbound edges are satisfied.
    pub async fn run_readiness(&self, run_id: Uuid) -> MvResult<RunReadiness> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;

        if run.status == AgentRunStatus::AwaitingApproval {
            return Ok(RunReadiness::BlockedByApproval);
        }

        let edges = self
            .store
            .nodes
            .list_work_order_edges(run.work_order_id)
            .await?;
        let nodes = self
            .store
            .nodes
            .list_work_order_nodes(run.work_order_id)
            .await?;
        let runs = self
            .store
            .nodes
            .list_agent_runs(run.work_order_id, 1000)
            .await?;

        for edge in edges.iter().filter(|edge| edge.to_node_id == run.node_id) {
            match edge.kind {
                // Mutual exclusion, evaluated through leases below rather than
                // as an ordering constraint.
                EdgeKind::Conflict => continue,
                EdgeKind::Verification => {
                    let Some(predecessor) = runs
                        .iter()
                        .filter(|candidate| candidate.node_id == edge.from_node_id)
                        .max_by_key(|candidate| candidate.attempt_no)
                    else {
                        return Ok(RunReadiness::BlockedByPredecessor {
                            predecessor: edge.from_node_id,
                        });
                    };
                    let tier = nodes
                        .iter()
                        .find(|node| node.node_id == edge.from_node_id)
                        .map(|node| node.risk_tier)
                        .unwrap_or(RiskTier::Standard);
                    let results = self
                        .store
                        .nodes
                        .list_gate_results(predecessor.run_id)
                        .await?;
                    if !missing_gates(tier, &results).is_empty() {
                        return Ok(RunReadiness::BlockedByVerification {
                            predecessor: predecessor.run_id,
                        });
                    }
                }
                _ => {
                    let completed = runs.iter().any(|candidate| {
                        candidate.node_id == edge.from_node_id
                            && candidate.status == AgentRunStatus::Completed
                    });
                    if !completed {
                        return Ok(RunReadiness::BlockedByPredecessor {
                            predecessor: edge.from_node_id,
                        });
                    }
                }
            }
        }

        // Conflict evaluation: does any other run hold a lease on a target this
        // run declares?
        let Some(contract) = nodes.iter().find(|node| node.node_id == run.node_id) else {
            return Err(MvError::NotFound("node contract not found".into()));
        };
        let digests = contract.write_target_digests();
        let conflicts = self
            .store
            .nodes
            .conflicting_write_targets(run_id, &digests, Utc::now())
            .await?;
        if !conflicts.is_empty() {
            return Ok(RunReadiness::BlockedByConflict {
                targets: conflicts.len(),
            });
        }

        Ok(RunReadiness::Ready)
    }

    /// Acquire the write leases a run needs in order to execute.
    ///
    /// All-or-nothing: a partial claim would let a run write part of its scope
    /// while another run holds the rest.
    pub async fn acquire_run_leases(&self, run_id: Uuid) -> MvResult<Vec<WriteLease>> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        let nodes = self
            .store
            .nodes
            .list_work_order_nodes(run.work_order_id)
            .await?;
        let contract = nodes
            .iter()
            .find(|node| node.node_id == run.node_id)
            .ok_or_else(|| MvError::NotFound("node contract not found".into()))?;

        let now = Utc::now();
        self.store
            .nodes
            .claim_write_leases(
                run_id,
                &contract.write_target_digests(),
                now,
                now + Duration::seconds(MAX_WRITE_LEASE_SECS),
            )
            .await
    }

    /// Start the next run attempt for one node contract.
    ///
    /// This is the only production path that creates an `AgentRun`. Without it
    /// the layer is auditable but not operable: every downstream endpoint —
    /// readiness, gates, approval, completion — has no subject to act on.
    ///
    /// **Admission and leasing only.** ADR 012 governs internal runs; no external
    /// dispatcher is invoked here. Starting a run spends a budgeted attempt,
    /// records the governed record and event atomically, and — only if autonomy
    /// allows — takes write leases. Call [`MindVaultEngine::execute_run`] to
    /// drive a leased run through artifact production and completion.
    ///
    /// The autonomy gate is consulted at the start, not after the fact. Because
    /// `AutonomyGate::evaluate` defers when no rule matches, an unconfigured
    /// vault parks the run for the owner rather than proceeding — the
    /// human-led default of System Principle 1. A parked run holds no leases,
    /// so an unbounded approval wait cannot block another run.
    pub async fn start_run(
        &self,
        work_order_id: Uuid,
        node_id: Uuid,
        actor: &StableUri,
        confidence: f32,
    ) -> MvResult<StartedRun> {
        let work_order = self
            .store
            .nodes
            .get_work_order(work_order_id)
            .await?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;
        if work_order.status.is_terminal() {
            return Err(MvError::Conflict(format!(
                "work order is {} and cannot start new runs",
                work_order.status.as_str()
            )));
        }

        let contracts = self
            .store
            .nodes
            .list_work_order_nodes(work_order_id)
            .await?;
        let contract = contracts
            .iter()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| MvError::NotFound("node contract not found".into()))?;

        // Attempt numbers advance exactly once per node, with no skips. Derived
        // from what is recorded rather than supplied by the caller, so a client
        // cannot fabricate a history.
        let existing = self
            .store
            .nodes
            .list_agent_runs(work_order_id, WORK_ORDER_EXPORT_RUN_LIMIT)
            .await?;
        let attempt_no = existing
            .iter()
            .filter(|run| run.node_id == node_id)
            .map(|run| run.attempt_no)
            .max()
            .unwrap_or(0)
            + 1;
        if attempt_no > contract.max_attempts {
            return Err(MvError::Conflict(format!(
                "node contract allows {} attempt(s); blind retry is not recovery",
                contract.max_attempts
            )));
        }

        let local_node_id = self.store.nodes.local_context_node_id().await?;
        let run_id = Uuid::now_v7();
        // One timestamp, not two: a new run must satisfy updated_at ==
        // created_at, and two `Utc::now()` calls can differ by a nanosecond.
        let created_at = Utc::now();
        let run = AgentRun {
            run_id,
            run_uri: StableUri::agent_run(local_node_id, run_id),
            work_order_id,
            node_id,
            attempt_no,
            status: AgentRunStatus::Ready,
            failure_class: None,
            principal: work_order.principal.clone(),
            actor: actor.clone(),
            correlation_id: work_order.correlation_id,
            causation_id: None,
            started_at: None,
            ended_at: None,
            created_at,
            updated_at: created_at,
        };

        let data = serde_json::json!({
            "run_id": run.run_id,
            "work_order_id": run.work_order_id,
            "node_id": run.node_id,
            "attempt_no": run.attempt_no,
            "record_digest": canonical_json_sha256(&serde_json::json!({
                "run_uri": run.run_uri.as_str(),
                "node_id": run.node_id,
                "attempt_no": run.attempt_no,
            })),
        });
        let event = EventEnvelope::new(NewEventEnvelope {
            event_type: AGENT_RUN_STARTED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: run.run_uri.clone(),
            schema: SchemaReference::new(StableUri::schema("agent-run-started").unwrap(), "1.0.0")
                .map_err(MvError::InvalidInput)?,
            principal: run.principal.clone(),
            actor: run.actor.clone(),
            correlation_id: run.correlation_id,
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(format!("start-run-{run_id}"))
                .map_err(MvError::InvalidInput)?,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: work_order.sensitivity,
            retention: work_order.retention,
            provenance: vec![ProvenanceReference {
                resource: run.run_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)?;

        // Budget decrement, run insertion, and event emission share one
        // immediate transaction. A run attempt is spent whether or not the
        // owner later authorizes it, because the attempt was made.
        let committed = self
            .store
            .nodes
            .commit_agent_run_with_event(&run, &WorkOrderSpend::one_run_attempt(), &event)
            .await?;

        // Only now consult autonomy: the attempt is on the record either way,
        // so a deferral is visible rather than silent.
        let request = EffectRequest::new(
            format!("work_order.run.{}", contract.executor_kind.as_str()),
            confidence,
        )
        .with_scope("work_order", work_order_id.to_string());

        match admit_effect(&self.autonomy, &request).await? {
            EffectOutcome::Admitted(_) => {
                let leases = self.acquire_run_leases(committed.run.run_id).await?;
                let run = self
                    .transition_run(committed.run.run_id, AgentRunStatus::Leased, None)
                    .await?;
                Ok(StartedRun {
                    run,
                    leases,
                    awaiting_approval: false,
                })
            }
            EffectOutcome::Refused(EffectRefusal::Blocked) => {
                // A blocked intent type is a standing policy decision, not a
                // pending one, so the attempt ends rather than waiting.
                let run = self
                    .fail_run(committed.run.run_id, RunFailureClass::Authorization)
                    .await?;
                Ok(StartedRun {
                    run,
                    leases: Vec::new(),
                    awaiting_approval: false,
                })
            }
            EffectOutcome::Refused(_) => {
                // Deferred or queued: the owner can still authorize this. Park
                // before taking any lease.
                let run = self.park_run_for_approval(committed.run.run_id).await?;
                Ok(StartedRun {
                    run,
                    leases: Vec::new(),
                    awaiting_approval: true,
                })
            }
        }
    }

    /// Park a run on owner approval.
    ///
    /// Approval is unbounded, so the run releases every write lease first. A
    /// parked run must never block another run on the owner's own vault.
    pub async fn park_run_for_approval(&self, run_id: Uuid) -> MvResult<AgentRun> {
        self.transition_run(run_id, AgentRunStatus::AwaitingApproval, None)
            .await
    }

    /// Resume an approved run.
    ///
    /// The run returns to the ready set rather than straight to execution: the
    /// write set must be re-checked, because approval authorizes the action,
    /// not a stale set of targets.
    ///
    /// SPACE-002 / ADR 012: the acting run actor cannot approve itself. The
    /// approver must be a distinct principal (typically the owner/reviewer).
    pub async fn resume_approved_run(
        &self,
        run_id: Uuid,
        approver: &StableUri,
    ) -> MvResult<AgentRun> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        if &run.actor == approver {
            return Err(MvError::AccessDenied(
                "a run cannot approve itself (SPACE-002 / ADR 012)".into(),
            ));
        }
        self.transition_run(run_id, AgentRunStatus::Ready, None)
            .await
    }

    /// Record gate evidence for a run.
    /// Record an artifact produced by a run, deriving its digest from the bytes.
    ///
    /// The digest is computed here rather than accepted from the caller. A
    /// client-supplied digest would make gate G2 verify a claim against itself,
    /// which is the failure the gate exists to catch — the same reason G2's
    /// verdict is computed server-side.
    ///
    /// Provenance is required: derived knowledge never erases the authority of
    /// its evidence, so an artifact that cites nothing is refused.
    pub async fn record_artifact(
        &self,
        run_id: Uuid,
        kind: impl Into<String>,
        payload: &[u8],
        provenance: Vec<ProvenanceReference>,
    ) -> MvResult<RunArtifact> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        // Evidence cannot be added to a closed attempt. Recovery is a new run,
        // not an amendment to a finished one.
        if run.status.is_terminal() {
            return Err(MvError::Conflict(format!(
                "run is {} and cannot record further artifacts",
                run.status.as_str()
            )));
        }
        let work_order = self
            .store
            .nodes
            .get_work_order(run.work_order_id)
            .await?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;

        let mut hasher = Sha256::new();
        hasher.update(payload);
        let content_digest = format!("{:x}", hasher.finalize());

        let local_node_id = self.store.nodes.local_context_node_id().await?;
        let artifact_id = Uuid::now_v7();
        let artifact = RunArtifact {
            artifact_id,
            artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
            run_id,
            work_order_id: run.work_order_id,
            artifact_kind: kind.into(),
            content_digest,
            schema: None,
            // Inherited from the governing Work Order rather than chosen by the
            // producer: a run must not be able to downgrade the handling class
            // of what it emits.
            sensitivity: work_order.sensitivity,
            retention: work_order.retention,
            provenance,
            created_at: Utc::now(),
        };
        artifact.validate().map_err(MvError::InvalidInput)?;
        self.store
            .nodes
            .record_run_artifact(&artifact, payload)
            .await
    }

    pub async fn record_run_gate(
        &self,
        run_id: Uuid,
        gate: GateId,
        outcome: GateOutcome,
        evaluator: &StableUri,
        evidence_digest: impl Into<String>,
        detail: Option<String>,
    ) -> MvResult<GateResult> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        let now = Utc::now();
        let result = GateResult {
            result_id: Uuid::now_v7(),
            run_id,
            work_order_id: run.work_order_id,
            gate,
            outcome,
            evaluator_actor: evaluator.clone(),
            evidence_digest: evidence_digest.into(),
            detail,
            evaluated_at: now,
            created_at: now,
        };
        // Enforces G5 independence (and digest shape) before persistence —
        // SPACE-002: a run cannot satisfy its own review gate.
        result.validate(&run).map_err(MvError::InvalidInput)?;
        self.store.nodes.record_gate_result(&result).await
    }

    /// Evaluate gate G2 — declared outputs exist and their content matches the
    /// recorded digests.
    ///
    /// The verification reads each artifact's stored bytes back and rehashes
    /// them. Comparing the recorded digest against itself would pass for an
    /// artifact whose content was never retrievable, which is precisely the
    /// failure the gate exists to catch.
    ///
    /// Returns the recorded evidence. A run that produced no artifacts fails
    /// G2 rather than passing vacuously: a node contract that declares outputs
    /// and produces none has not satisfied its contract.
    pub async fn verify_run_artifacts(
        &self,
        run_id: Uuid,
        evaluator: &StableUri,
    ) -> MvResult<GateResult> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        let artifacts: Vec<_> = self
            .store
            .nodes
            .list_run_artifacts(run.work_order_id)
            .await?
            .into_iter()
            .filter(|artifact| artifact.run_id == run_id)
            .collect();

        let mut verified = Vec::new();
        let mut failures = Vec::new();
        for artifact in &artifacts {
            match self
                .store
                .nodes
                .read_run_artifact_payload(artifact.artifact_id)
                .await
            {
                // The store re-verifies the digest on read, so a successful
                // read is proof the content matches.
                Ok(Some(_)) => verified.push(artifact.content_digest.as_str()),
                Ok(None) => failures.push(format!(
                    "artifact {} is recorded but its content is missing",
                    artifact.artifact_id
                )),
                Err(err) => failures.push(format!("artifact {}: {err}", artifact.artifact_id)),
            }
        }

        let (outcome, detail) = if artifacts.is_empty() {
            (
                GateOutcome::Fail,
                Some("the run recorded no artifacts to verify".to_string()),
            )
        } else if failures.is_empty() {
            (
                GateOutcome::Pass,
                Some(format!(
                    "{} artifact(s) verified against content",
                    verified.len()
                )),
            )
        } else {
            (GateOutcome::Fail, Some(failures.join("; ")))
        };

        // Evidence is the digest of the verified digest set, in a stable order,
        // so the gate result names exactly what was checked.
        verified.sort_unstable();
        let evidence_digest = canonical_json_sha256(&serde_json::json!({
            "gate": "g2",
            "run_id": run_id,
            "verified_digests": verified,
        }));

        self.record_run_gate(
            run_id,
            GateId::G2,
            outcome,
            evaluator,
            evidence_digest,
            detail,
        )
        .await
    }

    /// Export one Work Order's complete governed graph, artifact bytes included.
    ///
    /// Constitutional law 11 and `DATA_PORTABILITY_CONTRACT.md` require that a
    /// user can leave with their data. For this capability that means the whole
    /// graph — contracts, typed edges, every run attempt, all gate evidence,
    /// and artifact content with provenance — not a summary. Artifact bytes are
    /// read through the verifying read path, so an export can never carry a
    /// digest that describes content the vault could not produce.
    pub async fn export_work_order(&self, work_order_id: Uuid) -> MvResult<WorkOrderExport> {
        let work_order = self
            .store
            .nodes
            .get_work_order(work_order_id)
            .await?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;
        let nodes = self
            .store
            .nodes
            .list_work_order_nodes(work_order_id)
            .await?;
        let edges = self
            .store
            .nodes
            .list_work_order_edges(work_order_id)
            .await?;
        let runs = self
            .store
            .nodes
            .list_agent_runs(work_order_id, WORK_ORDER_EXPORT_RUN_LIMIT)
            .await?;
        // Silence is the failure mode to avoid: a truncated export looks like a
        // complete one to whoever restores it.
        if runs.len() >= WORK_ORDER_EXPORT_RUN_LIMIT {
            return Err(MvError::Storage(format!(
                "work order {work_order_id} has at least {WORK_ORDER_EXPORT_RUN_LIMIT} run \
                 attempts, which exceeds what one export can carry; refusing to emit a \
                 partial history"
            )));
        }

        let mut gate_results = Vec::new();
        for run in &runs {
            gate_results.extend(self.store.nodes.list_gate_results(run.run_id).await?);
        }

        let mut artifacts = Vec::new();
        for artifact in self.store.nodes.list_run_artifacts(work_order_id).await? {
            let content = self
                .store
                .nodes
                .read_run_artifact_payload(artifact.artifact_id)
                .await?
                .ok_or_else(|| {
                    MvError::Storage(format!(
                        "artifact {} is recorded but its content is missing; \
                         refusing to export an unrestorable digest",
                        artifact.artifact_id
                    ))
                })?;
            artifacts.push(ExportedArtifact {
                artifact,
                content_base64: BASE64_STANDARD.encode(&content),
            });
        }

        let export = WorkOrderExport {
            format_version: WORK_ORDER_EXPORT_FORMAT_V1.to_string(),
            exported_at: Utc::now(),
            work_order,
            nodes,
            edges,
            runs,
            gate_results,
            artifacts,
        };
        export.validate().map_err(MvError::InvalidInput)?;
        Ok(export)
    }

    /// Restore a previously exported Work Order into this vault.
    ///
    /// The export file is untrusted input: it may have been edited, truncated,
    /// or written by another implementation, so it is validated for internal
    /// consistency and every artifact digest is re-derived from its bytes
    /// before anything is written.
    ///
    /// Restore refuses when the Work Order already exists rather than merging.
    /// Two vaults' histories of the same identifier cannot be reconciled by
    /// overwriting one with the other, and silently picking a winner would
    /// destroy gate evidence — the precise record law 11 exists to preserve.
    pub async fn restore_work_order(&self, export: &WorkOrderExport) -> MvResult<RestoreSummary> {
        export.validate().map_err(MvError::InvalidInput)?;

        // Content is checked before any write, so a corrupt export cannot leave
        // a half-restored graph behind.
        let mut decoded = Vec::with_capacity(export.artifacts.len());
        for exported in &export.artifacts {
            let content = BASE64_STANDARD
                .decode(&exported.content_base64)
                .map_err(|err| {
                    MvError::InvalidInput(format!(
                        "artifact {} content is not valid base64: {err}",
                        exported.artifact.artifact_id
                    ))
                })?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            if format!("{:x}", hasher.finalize()) != exported.artifact.content_digest {
                return Err(MvError::InvalidInput(format!(
                    "artifact {} content does not match its recorded digest",
                    exported.artifact.artifact_id
                )));
            }
            decoded.push(content);
        }

        let work_order_id = export.work_order.work_order_id;
        if self
            .store
            .nodes
            .get_work_order(work_order_id)
            .await?
            .is_some()
        {
            return Err(MvError::Conflict(format!(
                "work order {work_order_id} already exists in this vault; \
                 restore does not merge divergent histories"
            )));
        }

        self.store
            .nodes
            .restore_work_order_graph(export, &decoded)
            .await?;

        Ok(RestoreSummary {
            work_order_id,
            nodes: export.nodes.len(),
            edges: export.edges.len(),
            runs: export.runs.len(),
            gate_results: export.gate_results.len(),
            artifacts: export.artifacts.len(),
        })
    }

    /// Complete a run, refusing while any required gate is outstanding.
    ///
    /// Completion is an evidence state, not a claim by the executor.
    pub async fn complete_run(&self, run_id: Uuid) -> MvResult<Result<AgentRun, Vec<GateId>>> {
        let run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        let nodes = self
            .store
            .nodes
            .list_work_order_nodes(run.work_order_id)
            .await?;
        let tier = nodes
            .iter()
            .find(|node| node.node_id == run.node_id)
            .map(|node| node.risk_tier)
            .ok_or_else(|| MvError::NotFound("node contract not found".into()))?;

        let results = self.store.nodes.list_gate_results(run_id).await?;
        let outstanding = missing_gates(tier, &results);
        if !outstanding.is_empty() {
            return Ok(Err(outstanding));
        }

        self.transition_run(run_id, AgentRunStatus::Completed, None)
            .await
            .map(Ok)
    }

    /// Fail a run with an explicit classification. Blind retry is not recovery,
    /// so the class is required rather than inferred.
    pub async fn fail_run(&self, run_id: Uuid, class: RunFailureClass) -> MvResult<AgentRun> {
        self.transition_run(run_id, AgentRunStatus::Failed, Some(class))
            .await
    }

    pub(crate) async fn transition_run(
        &self,
        run_id: Uuid,
        status: AgentRunStatus,
        failure_class: Option<RunFailureClass>,
    ) -> MvResult<AgentRun> {
        let current = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        let from = current.status;
        let now = Utc::now();

        let mut next = current.clone();
        next.status = status;
        next.failure_class = failure_class.or(current.failure_class);
        next.updated_at = now;
        if status == AgentRunStatus::Running && next.started_at.is_none() {
            next.started_at = Some(now);
        }
        if status.is_terminal() {
            next.ended_at = Some(now);
        }

        let local_node_id = self.store.nodes.local_context_node_id().await?;
        let event = Self::agent_run_transition_event(local_node_id, &next, from)?;
        let commit = self
            .store
            .nodes
            .transition_agent_run_with_event(&next, None, &event)
            .await?;
        Ok(commit.run)
    }

    fn work_order_admitted_event(
        local_node_id: Uuid,
        work_order: &WorkOrder,
        node_count: usize,
        edge_count: usize,
    ) -> MvResult<EventEnvelope> {
        let data = serde_json::json!({
            "work_order_id": work_order.work_order_id,
            "node_count": node_count,
            "edge_count": edge_count,
            "governing_node_uri": work_order.governing_node.as_str(),
            "record_digest": canonical_json_sha256(&serde_json::json!({
                "work_order_id": work_order.work_order_id,
                "revision": work_order.revision,
                "status": work_order.status.as_str(),
            })),
        });
        EventEnvelope::new(NewEventEnvelope {
            event_type: WORK_ORDER_ADMITTED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: work_order.work_order_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("work-order-admitted").map_err(MvError::InvalidInput)?,
                "1.0.0",
            )
            .map_err(MvError::InvalidInput)?,
            principal: work_order.principal.clone(),
            actor: work_order.actor.clone(),
            correlation_id: work_order.correlation_id,
            causation_id: work_order.causation_id,
            idempotency_key: IdempotencyKey::parse(&work_order.idempotency_key)
                .map_err(MvError::InvalidInput)?,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: work_order.sensitivity,
            retention: work_order.retention,
            provenance: vec![ProvenanceReference {
                resource: work_order.work_order_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)
    }

    fn agent_run_transition_event(
        local_node_id: Uuid,
        run: &AgentRun,
        from: AgentRunStatus,
    ) -> MvResult<EventEnvelope> {
        let data = serde_json::json!({
            "run_id": run.run_id,
            "from_status": from.as_str(),
            "to_status": run.status.as_str(),
            "record_digest": canonical_json_sha256(&serde_json::json!({
                "run_id": run.run_id,
                "attempt_no": run.attempt_no,
                "status": run.status.as_str(),
            })),
        });
        let key = format!(
            "run-{}-{}-{}",
            run.run_id,
            run.attempt_no,
            run.status.as_str()
        );
        EventEnvelope::new(NewEventEnvelope {
            event_type: AGENT_RUN_LIFECYCLE_TRANSITIONED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: run.run_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("agent-run-lifecycle-transitioned")
                    .map_err(MvError::InvalidInput)?,
                "1.0.0",
            )
            .map_err(MvError::InvalidInput)?,
            principal: run.principal.clone(),
            actor: run.principal.clone(),
            correlation_id: run.correlation_id,
            causation_id: run.causation_id,
            idempotency_key: IdempotencyKey::parse(&key).map_err(MvError::InvalidInput)?,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: vec![ProvenanceReference {
                resource: run.run_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use tempfile::TempDir;

    async fn test_engine() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    /// Register the local Context Node so governance events are admissible.
    async fn register_local_node(engine: &MindVaultEngine) -> Uuid {
        let local_node_id = engine.store.nodes.local_context_node_id().await.unwrap();
        let manifest = ContextCapabilityManifest::new(
            vec![ContextCapability::Discover],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let owner = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"local-context-owner"),
        );
        let mut record = ContextNodeRecord::discovered(
            local_node_id,
            ContextNodeType::Personal,
            owner,
            StableUri::node(local_node_id),
            "Personal Vault",
            manifest,
        )
        .unwrap();
        record.trust_class = ContextNodeTrustClass::local();
        record.status = ContextNodeStatus::Active;

        let data = serde_json::json!({
            "node_id": record.node_id,
            "node_type": record.node_type.as_str(),
            "status": record.status.as_str(),
            "record_digest": record.semantic_digest(),
            "capability_digest": record.capability_manifest.content_digest,
        });
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"governance-test-principal"),
        );
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: CONTEXT_NODE_REGISTERED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: record.node_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("context-node-registered").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: principal.clone(),
            actor: principal,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse("register-local-node").unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: record.node_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        event.payload_digest = record.semantic_digest();

        engine
            .store
            .nodes
            .commit_context_node_with_event(&record, &event)
            .await
            .unwrap();
        local_node_id
    }

    /// Issue a Tool Grant over an exact target set.
    async fn issue_tool_grant(
        engine: &MindVaultEngine,
        local_node_id: Uuid,
        grantee: &StableUri,
        targets: &[&str],
        key: &str,
    ) {
        let grantor =
            StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"grant-owner"));
        let grant = AuthorityGrant::new_tool(
            StableUri::node(local_node_id),
            grantor.clone(),
            grantee.clone(),
            targets
                .iter()
                .map(|value| StableUri::parse(*value).unwrap())
                .collect(),
            vec![ContextCapability::Command],
            "execute an admitted work order",
            Utc::now() + Duration::days(1),
        )
        .unwrap();

        let data = serde_json::json!({
            "grant_id": grant.grant_id,
            "grant_kind": grant.kind.as_str(),
            "grantee_uri": grant.grantee.as_str(),
            "governing_node_uri": grant.governing_node.as_str(),
            "record_digest": grant.semantic_digest(),
        });
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: AUTHORITY_GRANT_ISSUED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: grant.grant_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("authority-grant-issued").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: grantor.clone(),
            actor: grantor,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(key).unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: grant.grant_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        event.payload_digest = grant.semantic_digest();

        engine
            .store
            .nodes
            .commit_authority_grant_with_event(&grant, &event)
            .await
            .unwrap();
    }

    fn proposal(writes: &[&str], tier: RiskTier, key: &str) -> ProposedWorkOrder {
        ProposedWorkOrder {
            goal: "extract candidate decisions from the meeting".into(),
            non_goals: vec!["do not contact external services".into()],
            anchors: Vec::new(),
            success_criteria: vec!["candidate decisions carry provenance".into()],
            prohibited_outcomes: Vec::new(),
            budget: WorkOrderBudget {
                wall_clock_secs: 3600,
                run_attempts: 5,
                model_tokens: 100_000,
                effect_actions: 10,
            },
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            idempotency_key: key.into(),
            nodes: vec![ProposedNode {
                purpose: "extract decisions".into(),
                executor_kind: ExecutorKind::Engine,
                risk_tier: tier,
                read_scope: Vec::new(),
                write_scope: writes
                    .iter()
                    .map(|value| StableUri::parse(*value).unwrap())
                    .collect(),
                inputs: Vec::new(),
                timeout_secs: 600,
                max_attempts: 3,
            }],
            edges: Vec::new(),
        }
    }

    /// Admit a single-node order and start its first run.
    async fn admitted_run(
        engine: &MindVaultEngine,
        local_node_id: Uuid,
        actor: &StableUri,
        target: &str,
        key: &str,
    ) -> (WorkOrder, AgentRun) {
        issue_tool_grant(engine, local_node_id, actor, &[target], key).await;
        let admitted = engine
            .admit_work_order(actor, actor, &proposal(&[target], RiskTier::Low, key))
            .await
            .unwrap()
            .expect("admissible");
        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();

        let run_id = Uuid::now_v7();
        // One timestamp, not two: a new run must satisfy updated_at ==
        // created_at, and two Utc::now() calls can differ by a nanosecond.
        let created_at = Utc::now();
        let run = AgentRun {
            run_id,
            run_uri: StableUri::agent_run(local_node_id, run_id),
            work_order_id: admitted.work_order_id,
            node_id: contracts[0].node_id,
            attempt_no: 1,
            status: AgentRunStatus::Ready,
            failure_class: None,
            principal: admitted.principal.clone(),
            actor: actor.clone(),
            correlation_id: admitted.correlation_id,
            causation_id: None,
            started_at: None,
            ended_at: None,
            created_at,
            updated_at: created_at,
        };
        let data = serde_json::json!({
            "run_id": run.run_id,
            "work_order_id": run.work_order_id,
            "node_id": run.node_id,
            "attempt_no": run.attempt_no,
            "record_digest": "a".repeat(64),
        });
        let event = EventEnvelope::new(NewEventEnvelope {
            event_type: AGENT_RUN_STARTED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: run.run_uri.clone(),
            schema: SchemaReference::new(StableUri::schema("agent-run-started").unwrap(), "1.0.0")
                .unwrap(),
            principal: run.principal.clone(),
            actor: run.principal.clone(),
            correlation_id: run.correlation_id,
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(format!("start-{key}")).unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: vec![ProvenanceReference {
                resource: run.run_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        engine
            .store
            .nodes
            .commit_agent_run_with_event(&run, &WorkOrderSpend::one_run_attempt(), &event)
            .await
            .unwrap();
        (admitted, run)
    }

    #[tokio::test]
    async fn gate_g2_verifies_artifact_content_not_a_recorded_claim() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        let (barren_order, barren_run) = admitted_run(
            &engine,
            local_node_id,
            &actor,
            "mindvault://schemas/g2-barren",
            "g2-barren",
        )
        .await;

        // A run that produced nothing fails G2 rather than passing vacuously.
        let empty = engine
            .verify_run_artifacts(barren_run.run_id, &barren_order.principal)
            .await
            .unwrap();
        assert_eq!(empty.gate, GateId::G2);
        assert_eq!(empty.outcome, GateOutcome::Fail);

        // Gate evidence is immutable: the same run cannot re-roll a failed
        // gate into a pass. Recovery is a new run attempt, not a second
        // verdict on the same attempt.
        assert!(
            engine
                .verify_run_artifacts(barren_run.run_id, &barren_order.principal)
                .await
                .is_err(),
            "a run must not be able to re-evaluate a gate it already failed"
        );

        let (order, run) = admitted_run(
            &engine,
            local_node_id,
            &actor,
            "mindvault://schemas/g2",
            "g2-verify",
        )
        .await;

        let payload = b"verified decision summary";
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let digest = format!("{:x}", hasher.finalize());
        let artifact_id = Uuid::now_v7();
        engine
            .store
            .nodes
            .record_run_artifact(
                &RunArtifact {
                    artifact_id,
                    artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
                    run_id: run.run_id,
                    work_order_id: order.work_order_id,
                    artifact_kind: "decision-summary".into(),
                    content_digest: digest.clone(),
                    schema: None,
                    sensitivity: Sensitivity::Internal,
                    retention: RetentionClass::Operational,
                    provenance: vec![ProvenanceReference {
                        resource: StableUri::parse("mindvault://schemas/g2-source").unwrap(),
                        relation: ProvenanceRelation::WasDerivedFrom,
                    }],
                    created_at: Utc::now(),
                },
                payload,
            )
            .await
            .unwrap();

        let verified = engine
            .verify_run_artifacts(run.run_id, &order.principal)
            .await
            .unwrap();
        assert_eq!(verified.outcome, GateOutcome::Pass);
        // The gate read the bytes back and rehashed them; it did not compare
        // the recorded digest against itself.
        assert_eq!(
            engine
                .store
                .nodes
                .read_run_artifact_payload(artifact_id)
                .await
                .unwrap()
                .unwrap(),
            payload
        );

        // Evidence names what was checked, so two different artifact sets can
        // never produce the same gate evidence.
        assert_ne!(verified.evidence_digest, empty.evidence_digest);
    }

    #[tokio::test]
    async fn an_unconfigured_vault_parks_a_started_run_for_the_owner() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/startable"],
            "startable",
        )
        .await;
        let admitted = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(
                    &["mindvault://schemas/startable"],
                    RiskTier::Low,
                    "startable",
                ),
            )
            .await
            .unwrap()
            .expect("admissible");
        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();

        // No autonomy rule is configured, so the gate defers even at high
        // confidence. The human-led default is the whole point.
        let started = engine
            .start_run(admitted.work_order_id, contracts[0].node_id, &actor, 0.99)
            .await
            .unwrap();
        assert!(started.awaiting_approval);
        assert_eq!(started.run.status, AgentRunStatus::AwaitingApproval);
        assert_eq!(started.run.attempt_no, 1);

        // Parked before taking any lease: an unbounded approval wait must not
        // hold a target URI against every other run.
        assert!(started.leases.is_empty());

        // The attempt was spent even though the owner has not authorized it —
        // the attempt was made, and budgets are never refilled.
        let after = engine
            .store
            .nodes
            .get_work_order(admitted.work_order_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            after.remaining.run_attempts,
            admitted.budget.run_attempts - 1
        );

        // Approving returns it to the ready set rather than straight to work.
        let owner = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"local-context-owner"),
        );
        let resumed = engine
            .resume_approved_run(started.run.run_id, &owner)
            .await
            .unwrap();
        assert_eq!(resumed.status, AgentRunStatus::Ready);
    }

    #[tokio::test]
    async fn starting_a_run_refuses_to_exceed_the_contract_attempt_ceiling() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/capped"],
            "capped",
        )
        .await;

        let mut request = proposal(&["mindvault://schemas/capped"], RiskTier::Low, "capped");
        request.nodes[0].max_attempts = 2;
        let admitted = engine
            .admit_work_order(&actor, &actor, &request)
            .await
            .unwrap()
            .expect("admissible");
        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();
        let node_id = contracts[0].node_id;

        for expected in 1..=2 {
            let started = engine
                .start_run(admitted.work_order_id, node_id, &actor, 0.1)
                .await
                .unwrap();
            // Attempt numbers are derived from what is recorded, so a caller
            // cannot fabricate a history by supplying one.
            assert_eq!(started.run.attempt_no, expected);
        }

        // Blind retry is not recovery.
        assert!(matches!(
            engine
                .start_run(admitted.work_order_id, node_id, &actor, 0.1)
                .await,
            Err(MvError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn a_work_order_survives_an_export_and_restore_into_a_fresh_vault() {
        let (source, _source_dir) = test_engine().await;
        let local_node_id = register_local_node(&source).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        let (order, run) = admitted_run(
            &source,
            local_node_id,
            &actor,
            "mindvault://schemas/portable",
            "portable",
        )
        .await;

        // Content, provenance, and gate evidence — the three things an export
        // that only carried summaries would silently drop.
        let payload = b"exported decision summary";
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let digest = format!("{:x}", hasher.finalize());
        let artifact_id = Uuid::now_v7();
        let anchor = StableUri::parse("mindvault://schemas/portable-source").unwrap();
        source
            .store
            .nodes
            .record_run_artifact(
                &RunArtifact {
                    artifact_id,
                    artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
                    run_id: run.run_id,
                    work_order_id: order.work_order_id,
                    artifact_kind: "decision-summary".into(),
                    content_digest: digest.clone(),
                    schema: None,
                    sensitivity: Sensitivity::Internal,
                    retention: RetentionClass::Operational,
                    provenance: vec![ProvenanceReference {
                        resource: anchor.clone(),
                        relation: ProvenanceRelation::WasDerivedFrom,
                    }],
                    created_at: Utc::now(),
                },
                payload,
            )
            .await
            .unwrap();
        source
            .record_run_gate(
                run.run_id,
                GateId::G0,
                GateOutcome::Pass,
                &order.principal,
                "c".repeat(64),
                Some("scope resolved".into()),
            )
            .await
            .unwrap();

        let export = source.export_work_order(order.work_order_id).await.unwrap();
        assert_eq!(export.format_version, WORK_ORDER_EXPORT_FORMAT_V1);
        assert_eq!(export.runs.len(), 1);
        assert_eq!(export.gate_results.len(), 1);
        assert_eq!(export.artifacts.len(), 1);

        // Portability means a *file*, not an in-process handle.
        let serialized = serde_json::to_string(&export).unwrap();
        let parsed: WorkOrderExport = serde_json::from_str(&serialized).unwrap();

        // Restoring into the vault it came from is refused: two histories of
        // one identifier cannot be reconciled by overwriting.
        assert!(matches!(
            source.restore_work_order(&parsed).await,
            Err(MvError::Conflict(_))
        ));

        let (destination, _dest_dir) = test_engine().await;
        register_local_node(&destination).await;
        let summary = destination.restore_work_order(&parsed).await.unwrap();
        assert_eq!(summary.work_order_id, order.work_order_id);
        assert_eq!(summary.runs, 1);
        assert_eq!(summary.gate_results, 1);
        assert_eq!(summary.artifacts, 1);

        // Identity, budgets, evidence, provenance, and content all survive.
        let restored = destination
            .store
            .nodes
            .get_work_order(order.work_order_id)
            .await
            .unwrap()
            .expect("the work order must exist in the destination vault");
        // Compared against the exported state, not the pre-run handle: starting
        // a run spent a budget attempt and advanced the revision, and both must
        // survive. A restore that reset the budget would hand the destination
        // vault attempts the source had already consumed.
        assert_eq!(restored, export.work_order);
        assert_eq!(restored.revision, 2);
        assert_eq!(
            restored.remaining.run_attempts,
            order.budget.run_attempts - 1
        );

        let restored_gates = destination
            .store
            .nodes
            .list_gate_results(run.run_id)
            .await
            .unwrap();
        assert_eq!(restored_gates.len(), 1);
        assert_eq!(restored_gates[0].gate, GateId::G0);

        let restored_artifacts = destination
            .store
            .nodes
            .list_run_artifacts(order.work_order_id)
            .await
            .unwrap();
        assert_eq!(restored_artifacts.len(), 1);
        assert_eq!(restored_artifacts[0].provenance[0].resource, anchor);
        assert_eq!(
            destination
                .store
                .nodes
                .read_run_artifact_payload(artifact_id)
                .await
                .unwrap()
                .unwrap(),
            payload,
            "artifact bytes must survive the round trip"
        );

        // Authority does not travel with the record. The source node resolved a
        // Tool Grant; the destination has no such grant, so the restored
        // contract carries no authorization and must be re-authorized locally
        // before any new run can pass G0.
        let source_nodes = source
            .store
            .nodes
            .list_work_order_nodes(order.work_order_id)
            .await
            .unwrap();
        assert!(
            source_nodes[0].authorizing_grant_id.is_some(),
            "the source contract was authorized by a grant"
        );
        let restored_nodes = destination
            .store
            .nodes
            .list_work_order_nodes(order.work_order_id)
            .await
            .unwrap();
        assert!(
            restored_nodes[0].authorizing_grant_id.is_none(),
            "an export must not be a way to move authority between vaults"
        );
        // The declared scope itself is preserved — it is the record, not the
        // permission.
        assert_eq!(restored_nodes[0].write_scope, source_nodes[0].write_scope);

        // Re-exporting the restored vault yields the same graph, so the format
        // is a fixed point apart from the authority reference deliberately
        // dropped above.
        let round_tripped = destination
            .export_work_order(order.work_order_id)
            .await
            .unwrap();
        assert_eq!(round_tripped.work_order, export.work_order);
        assert_eq!(round_tripped.edges, export.edges);
        assert_eq!(round_tripped.runs, export.runs);
        assert_eq!(round_tripped.artifacts, export.artifacts);
        assert_eq!(round_tripped.gate_results, export.gate_results);
    }

    #[tokio::test]
    async fn a_tampered_export_is_refused_before_anything_is_written() {
        let (source, _source_dir) = test_engine().await;
        let local_node_id = register_local_node(&source).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        let (order, run) = admitted_run(
            &source,
            local_node_id,
            &actor,
            "mindvault://schemas/tampered",
            "tampered",
        )
        .await;

        let payload = b"original content";
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let digest = format!("{:x}", hasher.finalize());
        let artifact_id = Uuid::now_v7();
        source
            .store
            .nodes
            .record_run_artifact(
                &RunArtifact {
                    artifact_id,
                    artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
                    run_id: run.run_id,
                    work_order_id: order.work_order_id,
                    artifact_kind: "decision-summary".into(),
                    content_digest: digest,
                    schema: None,
                    sensitivity: Sensitivity::Internal,
                    retention: RetentionClass::Operational,
                    provenance: vec![ProvenanceReference {
                        resource: StableUri::parse("mindvault://schemas/tampered-source").unwrap(),
                        relation: ProvenanceRelation::WasDerivedFrom,
                    }],
                    created_at: Utc::now(),
                },
                payload,
            )
            .await
            .unwrap();

        let mut export = source.export_work_order(order.work_order_id).await.unwrap();
        // An export file is untrusted input on the way back in.
        export.artifacts[0].content_base64 = BASE64_STANDARD.encode(b"substituted content");

        let (destination, _dest_dir) = test_engine().await;
        register_local_node(&destination).await;
        assert!(matches!(
            destination.restore_work_order(&export).await,
            Err(MvError::InvalidInput(_))
        ));

        // Nothing was written: content is verified before the transaction opens.
        assert!(destination
            .store
            .nodes
            .get_work_order(order.work_order_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn a_write_target_outside_the_tool_grant_is_refused_at_admission() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));

        // Grant covers alpha only; the proposal also declares beta.
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/alpha"],
            "grant-alpha-only",
        )
        .await;

        let over_scoped = proposal(
            &["mindvault://schemas/alpha", "mindvault://schemas/beta"],
            RiskTier::Standard,
            "over-scoped",
        );
        let refusal = engine
            .admit_work_order(&actor, &actor, &over_scoped)
            .await
            .unwrap()
            .expect_err("an over-scoped contract must be refused");

        match refusal {
            AdmissionRefusal::WriteScopeOutsideGrant { targets, .. } => {
                assert_eq!(targets, vec!["mindvault://schemas/beta".to_string()]);
            }
            other => panic!("expected a write-scope refusal, got {other:?}"),
        }

        // Nothing became schedulable.
        assert!(engine
            .store
            .nodes
            .list_work_orders(None, 100)
            .await
            .unwrap()
            .is_empty());

        // Within the grant, the same shape is admitted.
        let in_scope = proposal(
            &["mindvault://schemas/alpha"],
            RiskTier::Standard,
            "in-scope",
        );
        let admitted = engine
            .admit_work_order(&actor, &actor, &in_scope)
            .await
            .unwrap()
            .expect("an in-scope contract is admissible");
        assert_eq!(admitted.status, WorkOrderStatus::Admitted);
    }

    #[tokio::test]
    async fn a_context_grant_does_not_authorize_a_write_scope() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"reader"));

        // A Context Grant over the very same target must not authorize writing.
        let grantor =
            StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"grant-owner"));
        let grant = AuthorityGrant::new_context(
            StableUri::node(local_node_id),
            grantor.clone(),
            actor.clone(),
            vec![StableUri::parse("mindvault://schemas/alpha").unwrap()],
            vec![ContextCapability::Read],
            "read the meeting source",
            Utc::now() + Duration::days(1),
        )
        .unwrap();
        let data = serde_json::json!({
            "grant_id": grant.grant_id,
            "grant_kind": grant.kind.as_str(),
            "grantee_uri": grant.grantee.as_str(),
            "governing_node_uri": grant.governing_node.as_str(),
            "record_digest": grant.semantic_digest(),
        });
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: AUTHORITY_GRANT_ISSUED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: grant.grant_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("authority-grant-issued").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: grantor.clone(),
            actor: grantor,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse("grant-context-only").unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: grant.grant_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        event.payload_digest = grant.semantic_digest();
        engine
            .store
            .nodes
            .commit_authority_grant_with_event(&grant, &event)
            .await
            .unwrap();

        let refusal = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(
                    &["mindvault://schemas/alpha"],
                    RiskTier::Low,
                    "context-only",
                ),
            )
            .await
            .unwrap()
            .expect_err("reading never implies authority to mutate");
        assert!(matches!(
            refusal,
            AdmissionRefusal::WriteScopeOutsideGrant { .. }
        ));
    }

    #[tokio::test]
    async fn intersecting_write_scopes_acquire_a_derived_conflict_edge() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/shared"],
            "grant-shared",
        )
        .await;

        let mut request = proposal(
            &["mindvault://schemas/shared"],
            RiskTier::Low,
            "derived-conflict",
        );
        // A second contract over the same target, which the author did not
        // mark as conflicting.
        request.nodes.push(request.nodes[0].clone());

        let admitted = engine
            .admit_work_order(&actor, &actor, &request)
            .await
            .unwrap()
            .expect("admissible");

        let edges = engine
            .store
            .nodes
            .list_work_order_edges(admitted.work_order_id)
            .await
            .unwrap();
        assert_eq!(edges.len(), 1, "the overlap must be guarded");
        assert_eq!(edges[0].kind, EdgeKind::Conflict);
        assert!(edges[0].derived);
    }

    #[tokio::test]
    async fn an_authored_conflict_edge_is_refused() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/alpha"],
            "grant-authored",
        )
        .await;

        let mut request = proposal(
            &["mindvault://schemas/alpha"],
            RiskTier::Low,
            "authored-conflict",
        );
        request.nodes.push(request.nodes[0].clone());
        request.edges.push(ProposedEdge {
            from: 0,
            to: 1,
            kind: EdgeKind::Conflict,
        });

        let refusal = engine
            .admit_work_order(&actor, &actor, &request)
            .await
            .unwrap()
            .expect_err("conflict edges are derived, not authored");
        assert!(matches!(refusal, AdmissionRefusal::Invalid { .. }));
    }

    #[tokio::test]
    async fn a_cyclic_proposal_is_refused() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/alpha", "mindvault://schemas/beta"],
            "grant-cycle",
        )
        .await;

        let mut request = proposal(&["mindvault://schemas/alpha"], RiskTier::Low, "cycle");
        let mut second = request.nodes[0].clone();
        second.write_scope = vec![StableUri::parse("mindvault://schemas/beta").unwrap()];
        request.nodes.push(second);
        request.edges.push(ProposedEdge {
            from: 0,
            to: 1,
            kind: EdgeKind::Data,
        });
        request.edges.push(ProposedEdge {
            from: 1,
            to: 0,
            kind: EdgeKind::Data,
        });

        let refusal = engine
            .admit_work_order(&actor, &actor, &request)
            .await
            .unwrap()
            .expect_err("a cyclic dependency graph is inadmissible");
        assert!(matches!(refusal, AdmissionRefusal::DependencyCycle { .. }));
    }

    #[tokio::test]
    async fn an_automated_intent_defers_by_default_rather_than_executing() {
        let (engine, _dir) = test_engine().await;
        let engine = std::sync::Arc::new(engine);

        // A node and a captured intent that would mutate it.
        let node = KnowledgeNode::new(NodeKind::Fact, "meeting notes");
        let node_id = node.id;
        engine.store.nodes.insert(&node).await.unwrap();

        let intent = CapturedIntent::new(node_id, IntentType::SuggestTag).with_confidence(0.99);
        let intent_id = intent.id;
        engine.store.nodes.log_intent(&intent).await.unwrap();

        // No autonomy rule is configured, so the gate defers — even at 0.99
        // confidence. Before the admission fix this path executed unchecked.
        let outcome = engine
            .apply_intent_autonomously(intent_id, 0.99)
            .await
            .unwrap();
        assert_eq!(
            outcome.expect_err("an unconfigured vault must defer"),
            crate::admission::EffectRefusal::Deferred
        );

        // The intent is untouched and still awaiting the owner.
        let stored = engine
            .store
            .nodes
            .get_intent(intent_id)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(stored.status, IntentStatus::Applied);
    }

    #[tokio::test]
    async fn an_owner_authorized_intent_still_applies() {
        let (engine, _dir) = test_engine().await;
        let engine = std::sync::Arc::new(engine);

        let node = KnowledgeNode::new(NodeKind::Fact, "meeting notes");
        let node_id = node.id;
        engine.store.nodes.insert(&node).await.unwrap();

        let mut intent = CapturedIntent::new(node_id, IntentType::SuggestTag).with_confidence(0.5);
        intent.parameters = serde_json::json!({ "tag": "meeting" });
        let intent_id = intent.id;
        engine.store.nodes.log_intent(&intent).await.unwrap();

        // The owner acting directly is the authority the gate defers to, so
        // this path proceeds without an autonomy rule.
        let result = engine.apply_intent(intent_id).await.unwrap();
        assert!(result.success, "owner-authorized application must proceed");
    }

    #[tokio::test]
    async fn completion_is_refused_while_a_required_gate_is_outstanding() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/gated"],
            "grant-gated",
        )
        .await;

        let admitted = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(&["mindvault://schemas/gated"], RiskTier::Low, "gated"),
            )
            .await
            .unwrap()
            .expect("admissible");

        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();
        let run_id = Uuid::now_v7();
        // One timestamp, not two: a new run must satisfy updated_at ==
        // created_at, and two Utc::now() calls can differ by a nanosecond.
        let created_at = Utc::now();
        let run = AgentRun {
            run_id,
            run_uri: StableUri::agent_run(local_node_id, run_id),
            work_order_id: admitted.work_order_id,
            node_id: contracts[0].node_id,
            attempt_no: 1,
            status: AgentRunStatus::Ready,
            failure_class: None,
            principal: admitted.principal.clone(),
            actor: actor.clone(),
            correlation_id: admitted.correlation_id,
            causation_id: None,
            started_at: None,
            ended_at: None,
            created_at,
            updated_at: created_at,
        };
        let data = serde_json::json!({
            "run_id": run.run_id,
            "work_order_id": run.work_order_id,
            "node_id": run.node_id,
            "attempt_no": run.attempt_no,
            "record_digest": "a".repeat(64),
        });
        let event = EventEnvelope::new(NewEventEnvelope {
            event_type: AGENT_RUN_STARTED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: run.run_uri.clone(),
            schema: SchemaReference::new(StableUri::schema("agent-run-started").unwrap(), "1.0.0")
                .unwrap(),
            principal: run.principal.clone(),
            actor: run.principal.clone(),
            correlation_id: run.correlation_id,
            causation_id: None,
            idempotency_key: IdempotencyKey::parse("start-gated-run").unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: vec![ProvenanceReference {
                resource: run.run_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        engine
            .store
            .nodes
            .commit_agent_run_with_event(&run, &WorkOrderSpend::one_run_attempt(), &event)
            .await
            .unwrap();

        // Move the run to a gateable state.
        engine
            .transition_run(run_id, AgentRunStatus::Leased, None)
            .await
            .unwrap();
        engine
            .transition_run(run_id, AgentRunStatus::Running, None)
            .await
            .unwrap();
        engine
            .transition_run(run_id, AgentRunStatus::Gated, None)
            .await
            .unwrap();

        // Completion is an evidence state, not an executor's claim.
        let outstanding = engine
            .complete_run(run_id)
            .await
            .unwrap()
            .expect_err("completion requires the tier's gates");
        assert_eq!(outstanding, RiskTier::Low.required_gates().to_vec());

        for gate in RiskTier::Low.required_gates() {
            engine
                .record_run_gate(
                    run_id,
                    *gate,
                    GateOutcome::Pass,
                    &admitted.principal,
                    "b".repeat(64),
                    None,
                )
                .await
                .unwrap();
        }

        let completed = engine
            .complete_run(run_id)
            .await
            .unwrap()
            .expect("all required gates passed");
        assert_eq!(completed.status, AgentRunStatus::Completed);
        assert!(completed.ended_at.is_some());
    }

    #[tokio::test]
    async fn agent_run_executor_drives_run_to_terminal_with_artifact() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/executable"],
            "executable-grant",
        )
        .await;

        // Admit engine runs without parking on owner approval.
        let mut rule = AutonomyRule::global(0.0);
        rule.allowed_intent_types = vec!["work_order.run.engine".into()];
        rule.max_actions_per_hour = 100;
        engine
            .store
            .nodes
            .add_autonomy_rule(&rule)
            .await
            .unwrap();

        let admitted = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(
                    &["mindvault://schemas/executable"],
                    RiskTier::Low,
                    "executable",
                ),
            )
            .await
            .unwrap()
            .expect("admissible");
        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();
        assert_eq!(contracts[0].executor_kind, ExecutorKind::Engine);

        let started = engine
            .start_run(admitted.work_order_id, contracts[0].node_id, &actor, 0.99)
            .await
            .unwrap();
        assert!(!started.awaiting_approval);
        assert_eq!(started.run.status, AgentRunStatus::Leased);

        let executed = engine.execute_run(started.run.run_id).await.unwrap();
        assert_eq!(executed.run.status, AgentRunStatus::Completed);
        assert!(executed.run.ended_at.is_some());
        assert_eq!(executed.artifact.artifact_kind, "engine.result");
        assert!(!executed.artifact.content_digest.is_empty());
        assert!(!executed.artifact.provenance.is_empty());
        assert_eq!(executed.gate_results.len(), RiskTier::Low.required_gates().len());
        assert!(executed
            .gate_results
            .iter()
            .all(|gate| gate.outcome == GateOutcome::Pass));

        let payload = engine
            .store
            .nodes
            .read_run_artifact_payload(executed.artifact.artifact_id)
            .await
            .unwrap()
            .expect("artifact bytes");
        let body: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(body["executor"], "engine");
        assert_eq!(body["run_id"], executed.run.run_id.to_string());

        let counters = engine.metrics.get_counters().await;
        assert_eq!(
            counters.get("trusted_work_completed").copied().unwrap_or(0),
            1,
            "WATW interim counter must increment on successful execute_run"
        );
    }

    /// SPACE-002 — a run cannot approve itself.
    #[tokio::test]
    async fn work_order_agent_run_cannot_approve_itself() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/executable"],
            "self-approve-grant",
        )
        .await;

        let admitted = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(
                    &["mindvault://schemas/executable"],
                    RiskTier::Low,
                    "self-approve",
                ),
            )
            .await
            .unwrap()
            .expect("admissible");
        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();
        let started = engine
            .start_run(admitted.work_order_id, contracts[0].node_id, &actor, 0.99)
            .await
            .unwrap();
        assert!(started.awaiting_approval);

        let err = engine
            .resume_approved_run(started.run.run_id, &actor)
            .await
            .expect_err("self-approval must fail closed");
        match err {
            MvError::AccessDenied(msg) => assert!(
                msg.contains("cannot approve itself"),
                "unexpected message: {msg}"
            ),
            other => panic!("expected AccessDenied, got {other:?}"),
        }
    }

    /// SPACE-002 — a run cannot record G5 as its own evaluator.
    #[tokio::test]
    async fn work_order_agent_run_cannot_satisfy_own_g5() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/executable"],
            "self-g5-grant",
        )
        .await;

        let mut rule = AutonomyRule::global(0.0);
        rule.allowed_intent_types = vec!["work_order.run.engine".into()];
        rule.max_actions_per_hour = 100;
        engine.store.nodes.add_autonomy_rule(&rule).await.unwrap();

        let admitted = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(
                    &["mindvault://schemas/executable"],
                    RiskTier::Low,
                    "self-g5",
                ),
            )
            .await
            .unwrap()
            .expect("admissible");
        let contracts = engine
            .store
            .nodes
            .list_work_order_nodes(admitted.work_order_id)
            .await
            .unwrap();
        let started = engine
            .start_run(admitted.work_order_id, contracts[0].node_id, &actor, 0.99)
            .await
            .unwrap();
        assert!(!started.awaiting_approval);
        let run_id = started.run.run_id;

        let digest = "a".repeat(64);
        let err = engine
            .record_run_gate(
                run_id,
                GateId::G5,
                GateOutcome::Pass,
                &actor,
                digest,
                Some("self review".into()),
            )
            .await
            .expect_err("self G5 must fail");
        match err {
            MvError::InvalidInput(msg) => assert!(
                msg.contains("cannot satisfy its own G5"),
                "unexpected: {msg}"
            ),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    /// SPACE-002 — a run cannot broaden the grant that authorizes it.
    ///
    /// Write scope is resolved through the Tool Grant at admission. Declaring
    /// targets outside that grant is refused, and a self-issued grant (grantor
    /// == grantee) cannot be constructed to invent the missing authority.
    #[tokio::test]
    async fn work_order_agent_run_cannot_broaden_its_own_grant() {
        let (engine, _dir) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"agent"));
        issue_tool_grant(
            &engine,
            local_node_id,
            &actor,
            &["mindvault://schemas/alpha"],
            "space002-narrow-grant",
        )
        .await;

        let before = engine
            .store
            .nodes
            .list_authority_grants(Some(&actor), None, None)
            .await
            .unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(
            before[0].targets,
            vec![StableUri::parse("mindvault://schemas/alpha").unwrap()]
        );

        // Self-grant cannot mint broader authority.
        let self_grant = AuthorityGrant::new_tool(
            StableUri::node(local_node_id),
            actor.clone(),
            actor.clone(),
            vec![
                StableUri::parse("mindvault://schemas/alpha").unwrap(),
                StableUri::parse("mindvault://schemas/beta").unwrap(),
            ],
            vec![ContextCapability::Execute],
            "self-broaden",
            Utc::now() + chrono::Duration::hours(1),
        );
        assert!(
            self_grant.is_err(),
            "grantor and grantee must remain distinct"
        );

        let refusal = engine
            .admit_work_order(
                &actor,
                &actor,
                &proposal(
                    &["mindvault://schemas/alpha", "mindvault://schemas/beta"],
                    RiskTier::Low,
                    "space002-broaden",
                ),
            )
            .await
            .unwrap()
            .expect_err("over-scope must fail closed");
        match refusal {
            AdmissionRefusal::WriteScopeOutsideGrant { targets, .. } => {
                assert!(targets.iter().any(|t| t.contains("beta")), "{targets:?}");
            }
            other => panic!("expected WriteScopeOutsideGrant, got {other:?}"),
        }

        let after = engine
            .store
            .nodes
            .list_authority_grants(Some(&actor), None, None)
            .await
            .unwrap();
        assert_eq!(after.len(), 1, "failed admit must not mint a rival grant");
        assert_eq!(after[0].targets, before[0].targets);
    }

    /// SPACE-002 — state-machine transitions and identical-retry classification.
    #[test]
    fn work_order_agent_run_state_machine_and_retry_rules() {
        use mv_core::{AgentRunStatus, RunFailureClass, WorkOrderStatus};

        assert!(WorkOrderStatus::Admitted.can_transition_to(WorkOrderStatus::Running));
        assert!(WorkOrderStatus::Running.can_transition_to(WorkOrderStatus::Completed));
        assert!(!WorkOrderStatus::Completed.can_transition_to(WorkOrderStatus::Running));

        assert!(AgentRunStatus::Ready.can_transition_to(AgentRunStatus::Leased));
        assert!(AgentRunStatus::Leased.can_transition_to(AgentRunStatus::Running));
        assert!(AgentRunStatus::Running.can_transition_to(AgentRunStatus::Gated));
        assert!(AgentRunStatus::Gated.can_transition_to(AgentRunStatus::Completed));
        assert!(AgentRunStatus::Failed.is_terminal());
        assert!(!AgentRunStatus::Failed.can_transition_to(AgentRunStatus::Ready));
        assert!(!AgentRunStatus::Completed.can_transition_to(AgentRunStatus::Running));
        assert!(!AgentRunStatus::AwaitingApproval.can_transition_to(AgentRunStatus::Running));

        assert!(RunFailureClass::Transient.permits_identical_retry());
        for class in [
            RunFailureClass::Deterministic,
            RunFailureClass::Specification,
            RunFailureClass::Authorization,
            RunFailureClass::Budget,
            RunFailureClass::Conflict,
        ] {
            assert!(
                !class.permits_identical_retry(),
                "{class:?} must not permit identical retry"
            );
        }
    }

    /// SPACE-002 — AgentRun is a distinct typed object; plan steps cannot masquerade.
    #[test]
    fn work_order_agent_run_is_not_a_plan_step() {
        fn assert_typed<T>() {}
        assert_typed::<AgentRun>();
        assert_typed::<WorkOrder>();
        assert_typed::<RunArtifact>();
        // Plans schema retired (AGENT-002). The AgentRun status vocabulary is
        // the only execution-attempt machine — titles/strings are not runs.
        assert!(AgentRunStatus::Ready.can_transition_to(AgentRunStatus::Leased));
        assert!(!AgentRunStatus::AwaitingApproval.can_transition_to(AgentRunStatus::Running));
        assert!(!AgentRunStatus::AwaitingApproval.may_hold_write_leases());
    }

}
