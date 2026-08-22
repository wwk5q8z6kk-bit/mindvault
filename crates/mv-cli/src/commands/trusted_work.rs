use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use mv_core::*;
use mv_engine::engine::{IssueAuthorityGrantRequest, ProposedNode, ProposedWorkOrder};
use uuid::Uuid;

/// Run the wedge value proof on the local vault (Engine / Low risk).
///
/// Flow: register Context Node → Tool Grant → admit Work Order → start run →
/// execute → print artifact digest and WATW counter.
pub async fn demo(config_path: &str) -> Result<()> {
    let engine = super::load_engine(config_path).await?;

    engine
        .register_local_context_node("MindVault Wedge Vault")
        .await
        .context("register local context node")?;

    let local_node_id = engine.local_context_node_id().await?;
    let actor = StableUri::principal(local_node_id, Uuid::new_v5(&local_node_id, b"cli-demo"));
    let target = StableUri::parse("mindvault://schemas/executable")
        .map_err(|e| anyhow!(e))?;

    engine
        .issue_authority_grant(IssueAuthorityGrantRequest {
            kind: AuthorityGrantKind::Tool,
            grantee: actor.clone(),
            targets: vec![target.clone()],
            capabilities: vec![ContextCapability::Command],
            purpose: "cli trusted-work wedge demo".into(),
            expires_at: Utc::now() + Duration::days(1),
            sensitivity_ceiling: Sensitivity::Internal,
            retention_ceiling: RetentionClass::Durable,
            idempotency_key: IdempotencyKey::parse("cli-trusted-work-grant")
                .map_err(|e| anyhow!(e))?,
        })
        .await
        .context("issue tool grant")?;

    // Clear autonomy parking so the demo completes without a separate approve step.
    let mut rule = AutonomyRule::global(0.0);
    rule.allowed_intent_types = vec!["work_order.run.engine".into()];
    rule.max_actions_per_hour = 100;
    engine
        .store
        .nodes
        .add_autonomy_rule(&rule)
        .await
        .context("add autonomy rule")?;

    let key = format!("cli-trusted-work-{}", Uuid::now_v7());
    let admitted = engine
        .admit_work_order(
            &actor,
            &actor,
            &ProposedWorkOrder {
                goal: "produce a provenance-linked trusted work artifact".into(),
                non_goals: vec!["do not contact external services".into()],
                anchors: Vec::new(),
                success_criteria: vec!["engine run completes with digested artifact".into()],
                prohibited_outcomes: Vec::new(),
                budget: WorkOrderBudget {
                    wall_clock_secs: 600,
                    run_attempts: 3,
                    model_tokens: 10_000,
                    effect_actions: 0,
                },
                sensitivity: Sensitivity::Internal,
                retention: RetentionClass::Operational,
                idempotency_key: key,
                nodes: vec![ProposedNode {
                    purpose: "internal engine trusted work".into(),
                    executor_kind: ExecutorKind::Engine,
                    risk_tier: RiskTier::Low,
                    read_scope: Vec::new(),
                    write_scope: vec![target],
                    inputs: Vec::new(),
                    timeout_secs: 120,
                    max_attempts: 3,
                }],
                edges: Vec::new(),
            },
        )
        .await
        .context("admit work order")?
        .map_err(|refusal| anyhow!("work order refused: {refusal:?}"))?;

    let contracts = engine
        .store
        .nodes
        .list_work_order_nodes(admitted.work_order_id)
        .await
        .context("list node contracts")?;
    let node_id = contracts
        .first()
        .map(|n| n.node_id)
        .ok_or_else(|| anyhow!("admitted work order has no node contracts"))?;

    let started = engine
        .start_run(admitted.work_order_id, node_id, &actor, 0.99)
        .await
        .context("start run")?;

    let run_id = started.run.run_id;
    if started.awaiting_approval {
        engine
            .resume_approved_run(run_id)
            .await
            .context("approve parked run")?;
    }

    let executed = engine.execute_run(run_id).await.context("execute run")?;
    let counters = engine.metrics.get_counters().await;
    let watw = counters.get("trusted_work_completed").copied().unwrap_or(0);

    println!("Trusted work completed");
    println!("======================");
    println!("work_order_id:   {}", admitted.work_order_id);
    println!("run_id:          {}", executed.run.run_id);
    println!("status:          {}", executed.run.status.as_str());
    println!("artifact_id:     {}", executed.artifact.artifact_id);
    println!("artifact_digest: {}", executed.artifact.content_digest);
    println!(
        "gates_passed:    {}",
        executed
            .gate_results
            .iter()
            .map(|g| g.gate.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("watw_counter:    {watw}  (process-lifetime trusted_work_completed)");
    println!();
    println!("North-star interim metric: completed governed Engine runs with required gates Pass.");
    Ok(())
}

/// Print the process-lifetime WATW interim counter from the loaded engine.
pub async fn watw(config_path: &str) -> Result<()> {
    let engine = super::load_engine(config_path).await?;
    let counters = engine.metrics.get_counters().await;
    let watw = counters.get("trusted_work_completed").copied().unwrap_or(0);
    println!("trusted_work_completed={watw}");
    println!(
        "(process-lifetime counter; persists only while this engine instance is alive)"
    );
    Ok(())
}
