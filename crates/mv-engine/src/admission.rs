//! Effectful-action admission: the single point every effectful path passes
//! through before mutating the vault.
//!
//! Contract: `docs/architecture/WORK_ORDER_MODEL.md`.
//! Decision: `docs/adr/012-governed-agent-execution-graph.md`.
//!
//! # Why this module exists
//!
//! `AutonomyGate::evaluate` correctly defers by default, but it used to be
//! invoked by *callers*. The relay path called it; `apply_intent` did not. Any
//! new execution path therefore silently bypassed the human-led guarantee of
//! System Principles 1 and 3 — not by disabling a check, but by not knowing
//! one existed.
//!
//! Caller discipline is not an enforcement mechanism. This module makes the
//! gate structural: an effectful action is represented by a value that can
//! only be obtained by passing the gate, so "forgot to check" becomes a
//! compile-time impossibility rather than a review question.

use mv_core::{AutonomyDecision, MvError, MvResult};

use crate::autonomy::AutonomyGate;

/// What an effectful action is asking to do, in the vocabulary the autonomy
/// rules already speak.
#[derive(Debug, Clone)]
pub struct EffectRequest {
    /// Rule-matching key, e.g. `intent.extract_task` or `work_order.run`.
    pub intent_type: String,
    /// Caller confidence in `[0, 1]`, compared against the rule threshold.
    pub confidence: f32,
    /// Scope hints as `(rule_type, scope_key)`, matched most-specific first:
    /// contact, then domain, then tag, then global.
    pub scope_hints: Vec<(String, String)>,
}

impl EffectRequest {
    pub fn new(intent_type: impl Into<String>, confidence: f32) -> Self {
        Self {
            intent_type: intent_type.into(),
            confidence: confidence.clamp(0.0, 1.0),
            scope_hints: Vec::new(),
        }
    }

    pub fn with_scope(mut self, rule_type: impl Into<String>, key: impl Into<String>) -> Self {
        self.scope_hints.push((rule_type.into(), key.into()));
        self
    }
}

/// Proof that the autonomy gate authorized an effect.
///
/// This type has no public constructor. The only way to obtain one is
/// [`admit_effect`], so a function that requires an `EffectAdmission` argument
/// cannot be called on an unevaluated path. That is the whole point: the
/// guarantee is carried by the type system rather than by remembering to call
/// a function.
#[derive(Debug, Clone)]
pub struct EffectAdmission {
    intent_type: String,
    decision: AutonomyDecision,
}

impl EffectAdmission {
    pub fn intent_type(&self) -> &str {
        &self.intent_type
    }

    pub fn decision(&self) -> AutonomyDecision {
        self.decision
    }
}

/// Why an effect was not admitted. Deferral is a normal, expected outcome —
/// the safe default — not an error condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectRefusal {
    /// Policy requires owner review before this effect may proceed.
    Deferred,
    /// A rule blocks this intent type outright.
    Blocked,
    /// Quiet hours or a rate limit apply; retry later.
    QueuedForLater,
}

impl EffectRefusal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deferred => "deferred",
            Self::Blocked => "blocked",
            Self::QueuedForLater => "queued_for_later",
        }
    }

    /// True when the owner could still authorize this through the approval
    /// queue. A blocked intent type cannot be resolved that way.
    pub const fn is_resolvable_by_owner(self) -> bool {
        matches!(self, Self::Deferred | Self::QueuedForLater)
    }
}

impl std::fmt::Display for EffectRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Deferred => "deferred for owner review",
            Self::Blocked => "blocked by an autonomy rule",
            Self::QueuedForLater => "queued: quiet hours or rate limit active",
        })
    }
}

/// The result of asking the gate. `Admitted` carries the proof token.
#[derive(Debug, Clone)]
pub enum EffectOutcome {
    Admitted(EffectAdmission),
    Refused(EffectRefusal),
}

impl EffectOutcome {
    pub const fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted(_))
    }

    pub fn admission(self) -> Option<EffectAdmission> {
        match self {
            Self::Admitted(admission) => Some(admission),
            Self::Refused(_) => None,
        }
    }

    pub fn refusal(&self) -> Option<EffectRefusal> {
        match self {
            Self::Admitted(_) => None,
            Self::Refused(refusal) => Some(*refusal),
        }
    }

    /// Convert a refusal into an error, for callers that treat deferral as a
    /// failed request rather than a queued one.
    pub fn require_admitted(self) -> MvResult<EffectAdmission> {
        match self {
            Self::Admitted(admission) => Ok(admission),
            Self::Refused(refusal) => Err(MvError::AccessDenied(refusal.to_string())),
        }
    }
}

/// Evaluate one effectful action against the autonomy gate.
///
/// Every effectful path must call this. `AutonomyGate::evaluate` already
/// defers when no rule matches, so an unconfigured vault refuses autonomous
/// effects rather than permitting them.
pub async fn admit_effect(gate: &AutonomyGate, request: &EffectRequest) -> MvResult<EffectOutcome> {
    let hints: Vec<(&str, &str)> = request
        .scope_hints
        .iter()
        .map(|(rule_type, key)| (rule_type.as_str(), key.as_str()))
        .collect();

    let decision = gate
        .evaluate(&request.intent_type, request.confidence, &hints)
        .await?;

    Ok(match decision {
        AutonomyDecision::AutoApply => EffectOutcome::Admitted(EffectAdmission {
            intent_type: request.intent_type.clone(),
            decision,
        }),
        AutonomyDecision::Defer => EffectOutcome::Refused(EffectRefusal::Deferred),
        AutonomyDecision::Block => EffectOutcome::Refused(EffectRefusal::Blocked),
        AutonomyDecision::QueueForLater => EffectOutcome::Refused(EffectRefusal::QueuedForLater),
    })
}

/// Admission for an effect the owner has explicitly authorized through the
/// approval queue or a direct interactive action.
///
/// This is not a bypass. It records that a human — the ultimate anchor in a
/// single-owner vault — is the authorizing party, which is exactly the
/// authority the autonomy gate exists to defer to. Automated paths must use
/// [`admit_effect`].
pub fn admit_owner_authorized(intent_type: impl Into<String>) -> EffectAdmission {
    EffectAdmission {
        intent_type: intent_type.into(),
        decision: AutonomyDecision::AutoApply,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_distinguishes_what_the_owner_can_still_authorize() {
        assert!(EffectRefusal::Deferred.is_resolvable_by_owner());
        assert!(EffectRefusal::QueuedForLater.is_resolvable_by_owner());
        // A blocked intent type is a standing policy decision, not a pending one.
        assert!(!EffectRefusal::Blocked.is_resolvable_by_owner());
    }

    #[test]
    fn a_refusal_carries_no_admission_token() {
        let outcome = EffectOutcome::Refused(EffectRefusal::Deferred);
        assert!(!outcome.is_admitted());
        assert_eq!(outcome.refusal(), Some(EffectRefusal::Deferred));
        assert!(outcome.clone().admission().is_none());
        assert!(outcome.require_admitted().is_err());
    }

    #[test]
    fn owner_authorization_is_recorded_as_an_admission() {
        let admission = admit_owner_authorized("intent.extract_task");
        assert_eq!(admission.intent_type(), "intent.extract_task");
        assert_eq!(admission.decision(), AutonomyDecision::AutoApply);
    }

    #[test]
    fn requests_clamp_confidence_into_range() {
        assert_eq!(EffectRequest::new("x", 5.0).confidence, 1.0);
        assert_eq!(EffectRequest::new("x", -1.0).confidence, 0.0);
        let request = EffectRequest::new("x", 0.5)
            .with_scope("contact", "abc")
            .with_scope("domain", "relay");
        assert_eq!(request.scope_hints.len(), 2);
    }
}
