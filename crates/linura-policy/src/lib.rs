#![forbid(unsafe_code)]

use linura_core::{
    Actor, ActorKind, CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId,
    RequestId, ResourceId, RiskClass, SemanticReason,
};
use linura_planner::{PlanFinding, PlanStatus, ReconciliationPlan, StateChange};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApprovalClass {
    InteractiveUser,
    Administrator,
    DestructiveAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySnapshot {
    pub policy_id: PolicyId,
    pub revision_id: PolicyRevisionId,
}

/// Canonical policy-review projection derived from a `ReconciliationPlan`.
///
/// External callers cannot assemble this type field-by-field. Linura Control must
/// obtain it from the canonical planner output plus an authenticated principal,
/// preventing policy evaluation from drifting onto a second independently
/// authored plan model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySubject {
    principal: PrincipalId,
    plan_id: PlanId,
    request_id: RequestId,
    actor: Actor,
    provider: ProviderId,
    resource: ResourceId,
    capability: CapabilityId,
    reason: SemanticReason,
    observed_evidence_id: String,
    prospective_risk: RiskClass,
    status: PlanStatus,
    changes: Vec<StateChange>,
    findings: Vec<PlanFinding>,
}

impl PolicySubject {
    #[must_use]
    pub fn from_plan(principal: PrincipalId, plan: &ReconciliationPlan) -> Self {
        Self {
            principal,
            plan_id: plan.id.clone(),
            request_id: plan.request_id.clone(),
            actor: plan.actor.clone(),
            provider: plan.provider.clone(),
            resource: plan.resource.clone(),
            capability: plan.observation_capability.clone(),
            reason: plan.reason.clone(),
            observed_evidence_id: plan.observed_evidence_id.clone(),
            prospective_risk: plan.prospective_risk,
            status: plan.status,
            changes: plan.changes.clone(),
            findings: plan.findings.clone(),
        }
    }

    #[must_use]
    pub fn principal(&self) -> &PrincipalId {
        &self.principal
    }

    #[must_use]
    pub fn plan_id(&self) -> &PlanId {
        &self.plan_id
    }

    #[must_use]
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    #[must_use]
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    #[must_use]
    pub fn resource(&self) -> &ResourceId {
        &self.resource
    }

    #[must_use]
    pub fn capability(&self) -> &CapabilityId {
        &self.capability
    }

    #[must_use]
    pub fn reason(&self) -> &SemanticReason {
        &self.reason
    }

    #[must_use]
    pub fn observed_evidence_id(&self) -> &str {
        &self.observed_evidence_id
    }

    #[must_use]
    pub const fn prospective_risk(&self) -> RiskClass {
        self.prospective_risk
    }

    #[must_use]
    pub const fn status(&self) -> PlanStatus {
        self.status
    }

    #[must_use]
    pub fn changes(&self) -> &[StateChange] {
        &self.changes
    }

    #[must_use]
    pub fn findings(&self) -> &[PlanFinding] {
        &self.findings
    }

    #[must_use]
    pub fn has_blockers(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.level == linura_planner::PlanFindingLevel::Blocker)
    }
}

/// Identity binding for one policy evaluation.
///
/// v0.3 approval evidence must match this exact binding. A different principal,
/// plan, authoritative evidence identity, policy revision, resource or capability
/// is a different review subject and cannot reuse the previous decision. The
/// enclosing `PolicyEvaluation` also retains the full exact `PolicySubject`, so
/// material planned changes/findings/provenance can be compared rather than
/// relying on `PlanId` alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewBinding {
    pub principal: PrincipalId,
    pub plan_id: PlanId,
    pub request_id: RequestId,
    pub observed_evidence_id: String,
    pub provider: ProviderId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub policy_id: PolicyId,
    pub policy_revision_id: PolicyRevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny {
        reason: String,
    },
    RequireApproval {
        class: ApprovalClass,
        reason: String,
    },
    Blocked {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyEvaluation {
    pub subject: PolicySubject,
    pub binding: ReviewBinding,
    pub decision: PolicyDecision,
}

pub trait PolicyEngine {
    fn snapshot(&self) -> &PolicySnapshot;
    fn evaluate_decision(&self, subject: &PolicySubject) -> PolicyDecision;

    #[must_use]
    fn evaluate(&self, subject: &PolicySubject) -> PolicyEvaluation {
        let snapshot = self.snapshot();
        PolicyEvaluation {
            subject: subject.clone(),
            binding: ReviewBinding {
                principal: subject.principal().clone(),
                plan_id: subject.plan_id().clone(),
                request_id: subject.request_id().clone(),
                observed_evidence_id: subject.observed_evidence_id().to_owned(),
                provider: subject.provider().clone(),
                resource: subject.resource().clone(),
                capability: subject.capability().clone(),
                policy_id: snapshot.policy_id.clone(),
                policy_revision_id: snapshot.revision_id.clone(),
            },
            decision: self.evaluate_decision(subject),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselinePolicy {
    snapshot: PolicySnapshot,
}

impl Default for BaselinePolicy {
    fn default() -> Self {
        Self {
            snapshot: PolicySnapshot {
                policy_id: PolicyId::new("policy:baseline")
                    .unwrap_or_else(|error| unreachable!("{error}")),
                revision_id: PolicyRevisionId::new("policy:baseline:v1")
                    .unwrap_or_else(|error| unreachable!("{error}")),
            },
        }
    }
}

impl PolicyEngine for BaselinePolicy {
    fn snapshot(&self) -> &PolicySnapshot {
        &self.snapshot
    }

    fn evaluate_decision(&self, subject: &PolicySubject) -> PolicyDecision {
        if subject.status() == PlanStatus::Blocked || subject.has_blockers() {
            return PolicyDecision::Blocked {
                reason: "plan contains blockers and cannot enter approval review".into(),
            };
        }
        if subject.actor().kind == ActorKind::Remote {
            return PolicyDecision::Deny {
                reason: "remote actors are disabled in the initial profile".into(),
            };
        }
        if subject.actor().kind == ActorKind::Agent
            && subject.prospective_risk() >= RiskClass::SystemMutation
        {
            return PolicyDecision::RequireApproval {
                class: ApprovalClass::InteractiveUser,
                reason: "agents are untrusted proposers and cannot authorize system mutations"
                    .into(),
            };
        }
        match subject.prospective_risk() {
            RiskClass::ReadOnly | RiskClass::UserState => PolicyDecision::Allow,
            RiskClass::SystemMutation => PolicyDecision::RequireApproval {
                class: ApprovalClass::InteractiveUser,
                reason: "system mutation requires interactive approval".into(),
            },
            RiskClass::SecuritySensitive => PolicyDecision::RequireApproval {
                class: ApprovalClass::Administrator,
                reason: "security-sensitive mutation requires administrator approval".into(),
            },
            RiskClass::Destructive => PolicyDecision::RequireApproval {
                class: ApprovalClass::DestructiveAction,
                reason: "destructive mutation requires dedicated approval".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linura_core::{ActorId, IntentId, ValidationError};
    use linura_planner::{PlanFindingLevel, StateChange};

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn subject(kind: ActorKind, risk: RiskClass, status: PlanStatus) -> PolicySubject {
        PolicySubject {
            principal: id(PrincipalId::new("uid:1000")),
            plan_id: id(PlanId::new("plan:test")),
            request_id: id(RequestId::new("req:test")),
            actor: Actor {
                id: id(ActorId::new("actor:test")),
                kind,
                interactive: kind == ActorKind::Human,
            },
            provider: id(ProviderId::new("systemd")),
            resource: id(ResourceId::new("systemd:unit:test.service")),
            capability: id(CapabilityId::new("systemd.unit.observe")),
            reason: SemanticReason {
                summary: "test intent".into(),
                intent_ids: vec![id(IntentId::new("intent:test"))],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            observed_evidence_id: "evidence:test".into(),
            prospective_risk: risk,
            status,
            changes: vec![StateChange {
                key: "active_state".into(),
                current: Some("inactive".into()),
                desired: "active".into(),
            }],
            findings: if status == PlanStatus::Blocked {
                vec![PlanFinding {
                    code: "blocked".into(),
                    level: PlanFindingLevel::Blocker,
                    message: "blocked for test".into(),
                }]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn agent_system_mutation_requires_approval() {
        let policy = BaselinePolicy::default();
        let expected_subject = subject(
            ActorKind::Agent,
            RiskClass::SystemMutation,
            PlanStatus::ChangeProposed,
        );
        let evaluation = policy.evaluate(&expected_subject);
        assert!(matches!(
            evaluation.decision,
            PolicyDecision::RequireApproval { .. }
        ));
        assert_eq!(evaluation.subject, expected_subject);
        assert_eq!(evaluation.binding.principal.as_str(), "uid:1000");
        assert_eq!(evaluation.binding.observed_evidence_id, "evidence:test");
    }

    #[test]
    fn remote_is_denied() {
        let policy = BaselinePolicy::default();
        let evaluation = policy.evaluate(&subject(
            ActorKind::Remote,
            RiskClass::ReadOnly,
            PlanStatus::NoChange,
        ));
        assert!(matches!(evaluation.decision, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn blocked_plan_cannot_be_approved() {
        let policy = BaselinePolicy::default();
        let evaluation = policy.evaluate(&subject(
            ActorKind::Human,
            RiskClass::SystemMutation,
            PlanStatus::Blocked,
        ));
        assert!(matches!(evaluation.decision, PolicyDecision::Blocked { .. }));
    }
}
