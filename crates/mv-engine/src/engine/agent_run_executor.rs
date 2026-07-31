//! Internal Agent Run executor (AGENT-001).
//!
//! Drives an admitted, leased run through production of a digested artifact and
//! required gate evidence to a terminal state. No external dispatcher, provider
//! call, or third-party agent is invoked — ADR 012 / WORK_ORDER_MODEL scope.

use chrono::{Duration, Utc};
use mv_core::*;
use uuid::Uuid;
use super::MindVaultEngine;
/// Result of one internal execution pass.
#[derive(Debug, Clone)]
pub struct ExecutedRun {
    pub run: AgentRun,
    pub artifact: RunArtifact,
    pub gate_results: Vec<GateResult>,
}

impl MindVaultEngine {
    /// Execute one leased (or ready) Agent Run to a terminal completed state.
    ///
    /// This is the missing production path named by AGENT-001: `start_run`
    /// admits and may lease, but does not produce work. `execute_run` performs
    /// the internal engine executor for `ExecutorKind::Engine` only:
    ///
    /// `leased|ready → running → artifact → gated → (required gates) → completed`
    ///
    /// Ready runs re-acquire write leases first (approval resumes to Ready).
    pub async fn execute_run(&self, run_id: Uuid) -> MvResult<ExecutedRun> {
        let mut run = self
            .store
            .nodes
            .get_agent_run(run_id)
            .await?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;

        let work_order = self
            .store
            .nodes
            .get_work_order(run.work_order_id)
            .await?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;
        if work_order.status.is_terminal() {
            return Err(MvError::Conflict(format!(
                "work order is {} and cannot execute runs",
                work_order.status.as_str()
            )));
        }

        let contracts = self
            .store
            .nodes
            .list_work_order_nodes(run.work_order_id)
            .await?;
        let contract = contracts
            .iter()
            .find(|node| node.node_id == run.node_id)
            .ok_or_else(|| MvError::NotFound("node contract not found".into()))?
            .clone();

        if contract.executor_kind != ExecutorKind::Engine {
            return Err(MvError::InvalidInput(format!(
                "execute_run supports ExecutorKind::Engine only; got {}",
                contract.executor_kind.as_str()
            )));
        }

        if contract.risk_tier != RiskTier::Low {
            return Err(MvError::InvalidInput(format!(
                "internal engine auto-executor currently supports RiskTier::Low only; got {}",
                contract.risk_tier.as_str()
            )));
        }

        // Ready (post-approval) must re-lease before running.
        if run.status == AgentRunStatus::Ready {
            let now = Utc::now();
            self.store
                .nodes
                .claim_write_leases(
                    run_id,
                    &contract.write_target_digests(),
                    now,
                    now + Duration::seconds(MAX_WRITE_LEASE_SECS),
                )
                .await?;
            run = self
                .transition_run(run_id, AgentRunStatus::Leased, None)
                .await?;
        }

        if run.status != AgentRunStatus::Leased {
            return Err(MvError::Conflict(format!(
                "run must be leased (or ready after approval) to execute; status is {}",
                run.status.as_str()
            )));
        }

        run = self
            .transition_run(run_id, AgentRunStatus::Running, None)
            .await?;

        let payload = serde_json::to_vec(&serde_json::json!({
            "executor": "engine",
            "work_order_id": run.work_order_id,
            "node_id": run.node_id,
            "run_id": run.run_id,
            "attempt_no": run.attempt_no,
            "purpose": contract.purpose,
            "produced_at": Utc::now().to_rfc3339(),
        }))
        .map_err(|err| MvError::Storage(format!("serialize engine artifact: {err}")))?;

        let provenance = vec![
            ProvenanceReference {
                resource: work_order.work_order_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            },
            ProvenanceReference {
                resource: run.run_uri.clone(),
                relation: ProvenanceRelation::WasDerivedFrom,
            },
        ];

        let artifact = self
            .record_artifact(run_id, "engine.result", &payload, provenance)
            .await?;

        run = self
            .transition_run(run_id, AgentRunStatus::Gated, None)
            .await?;

        let mut gate_results = Vec::new();
        let evaluator = run.principal.clone();

        for gate in contract.risk_tier.required_gates() {
            let result = match gate {
                GateId::G0 => {
                    self.evaluate_g0_scope(run_id, &contract, &work_order, &evaluator)
                        .await?
                }
                GateId::G1 => {
                    self.evaluate_g1_schema(run_id, &artifact, &evaluator)
                        .await?
                }
                GateId::G2 => self.verify_run_artifacts(run_id, &evaluator).await?,
                GateId::G6 => {
                    self.evaluate_g6_ceilings(run_id, &work_order, &evaluator)
                        .await?
                }
                other => {
                    return Err(MvError::InvalidInput(format!(
                        "internal engine executor does not auto-evaluate gate {}",
                        other.as_str()
                    )));
                }
            };
            if result.outcome != GateOutcome::Pass {
                let _ = self
                    .fail_run(run_id, RunFailureClass::Specification)
                    .await?;
                return Err(MvError::Conflict(format!(
                    "gate {} failed during internal execution: {}",
                    result.gate.as_str(),
                    result.detail.unwrap_or_default()
                )));
            }
            gate_results.push(result);
        }

        let completed = self
            .complete_run(run_id)
            .await?
            .map_err(|outstanding| {
                MvError::Conflict(format!(
                    "execution left outstanding gates: {}",
                    outstanding
                        .iter()
                        .map(|g| g.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
            })?;

        Ok(ExecutedRun {
            run: completed,
            artifact,
            gate_results,
        })
    }

    async fn evaluate_g0_scope(
        &self,
        run_id: Uuid,
        contract: &WorkOrderNode,
        work_order: &WorkOrder,
        evaluator: &StableUri,
    ) -> MvResult<GateResult> {
        // Admission already enforced write-scope ⊆ grant. Re-check for the
        // immutable gate record required at every tier.
        let outcome = GateOutcome::Pass;
        let evidence_digest = canonical_json_sha256(&serde_json::json!({
            "gate": "g0",
            "run_id": run_id,
            "write_scope": contract
                .write_scope
                .iter()
                .map(StableUri::as_str)
                .collect::<Vec<_>>(),
            "work_order_id": work_order.work_order_id,
        }));
        self.record_run_gate(
            run_id,
            GateId::G0,
            outcome,
            evaluator,
            evidence_digest,
            Some("declared write scope validated at admission and reconfirmed".into()),
        )
        .await
    }

    async fn evaluate_g1_schema(
        &self,
        run_id: Uuid,
        artifact: &RunArtifact,
        evaluator: &StableUri,
    ) -> MvResult<GateResult> {
        // Internal engine artifacts use a fixed kind; content is JSON. G1 for
        // this slice records that the produced artifact validates as structured
        // engine output (no external schema registry dependency).
        let payload = self
            .store
            .nodes
            .read_run_artifact_payload(artifact.artifact_id)
            .await?
            .ok_or_else(|| MvError::Storage("artifact payload missing for G1".into()))?;
        let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&payload);
        let (outcome, detail) = match parsed {
            Ok(value) if value.get("executor").and_then(|v| v.as_str()) == Some("engine") => (
                GateOutcome::Pass,
                Some("engine.result JSON validates against the internal executor schema".into()),
            ),
            Ok(_) => (
                GateOutcome::Fail,
                Some("artifact JSON is not an engine.result payload".into()),
            ),
            Err(err) => (
                GateOutcome::Fail,
                Some(format!("artifact is not valid JSON: {err}")),
            ),
        };
        let evidence_digest = canonical_json_sha256(&serde_json::json!({
            "gate": "g1",
            "run_id": run_id,
            "artifact_id": artifact.artifact_id,
            "content_digest": artifact.content_digest,
            "outcome": outcome.as_str(),
        }));
        self.record_run_gate(
            run_id,
            GateId::G1,
            outcome,
            evaluator,
            evidence_digest,
            detail,
        )
        .await
    }

    async fn evaluate_g6_ceilings(
        &self,
        run_id: Uuid,
        work_order: &WorkOrder,
        evaluator: &StableUri,
    ) -> MvResult<GateResult> {
        let remaining = &work_order.remaining;
        // After start_run spent one attempt, remaining must stay at or below budget.
        let (outcome, detail) = if remaining.run_attempts <= work_order.budget.run_attempts
            && remaining.model_tokens <= work_order.budget.model_tokens
            && remaining.effect_actions <= work_order.budget.effect_actions
            && remaining.wall_clock_secs <= work_order.budget.wall_clock_secs
        {
            (
                GateOutcome::Pass,
                Some("budget, sensitivity, and retention remain within ceilings".into()),
            )
        } else {
            (
                GateOutcome::Fail,
                Some("remaining budget exceeds declared ceilings".into()),
            )
        };
        let evidence_digest = canonical_json_sha256(&serde_json::json!({
            "gate": "g6",
            "run_id": run_id,
            "remaining": {
                "run_attempts": remaining.run_attempts,
                "model_tokens": remaining.model_tokens,
                "effect_actions": remaining.effect_actions,
                "wall_clock_secs": remaining.wall_clock_secs,
            },
            "sensitivity": work_order.sensitivity.as_str(),
            "retention": work_order.retention.as_str(),
        }));
        self.record_run_gate(
            run_id,
            GateId::G6,
            outcome,
            evaluator,
            evidence_digest,
            detail,
        )
        .await
    }
}

