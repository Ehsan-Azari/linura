#![forbid(unsafe_code)]

use linura_core::{
    ActorKind, CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass,
};
use linura_planner::{PlanStatus, ReconciliationPlan};

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

/// Exact immutable subject presented to policy review.
///
/// The canonical reconciliation plan is retained intact rather than re-created
/// as a second policy-owned plan model. The authenticated principal is bound
/// alongside it so caller identity cannot be inferred from client-supplied
/// provenance alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySubject {
    principal: PrincipalId,
    plan: ReconciliationPlan,
}

impl PolicySubject {
    #[must_use]
    pub fn from_plan(principal: PrincipalId, plan: &ReconciliationPlan) -> Self {
        Self {
            principal,
            plan: plan.clone(),
        }
    }

    #[must_use]
    pub fn principal(&self) -> &PrincipalId {
        &self.principal
    }

    #[must_use]
    pub fn plan(&self) -> &ReconciliationPlan {
        &self.plan
    }
}

/// Identity binding for a policy evaluation.
///
/// v0.3 approval evidence must match this exact binding. A different principal,
/// plan, authoritative evidence identity, policy revision, resource or capability
/// is a different review subject and cannot reuse the previous decision.
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
    pub binding: ReviewBinding,
    pub decision: PolicyDecision,
}

pub trait PolicyEngine {
    fn snapshot(&self) -> &PolicySnapshot;
    fn evaluate_decision(&self, subject: &PolicySubject) -> PolicyDecision;

    #[must_use]
    fn evaluate(&self, subject: &PolicySubject) -> PolicyEvaluation {
        let plan = subject.plan();
        let snapshot = self.snapshot();
        PolicyEvaluation {
            binding: ReviewBinding {
                principal: subject.principal().clone(),
                plan_id: plan.id.clone(),
                request_id: plan.request_id.clone(),
                observed_evidence_id: plan.observed_evidence_id.clone(),
                provider: plan.provider.clone(),
                resource: plan.resource.clone(),
                capability: plan.observation_capability.clone(),
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
        let plan = subject.plan();

        if plan.status == PlanStatus::Blocked || plan.has_blockers() {
            return PolicyDecision::Blocked {
                reason: "plan contains blockers and cannot enter approval review".into(),
            };
        }
        if plan.actor.kind == ActorKind::Remote {
            return PolicyDecision::Deny {
                reason: "remote actors are disabled in the initial profile".into(),
            };
        }
        if plan.actor.kind == ActorKind::Agent && plan.prospective_risk >= RiskClass::SystemMutation {
            return PolicyDecision::RequireApproval {
                class: ApprovalClass::InteractiveUser,
                reason: "agents are untrusted proposers and cannot authorize system mutations"
                    .into(),
            };
        }
        match plan.prospective_risk {
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
    use linura_core::{
        Actor, ActorId, IntentId, SemanticReason, ValidationError,
    };
    use linura_planner::{PlanFinding, PlanFindingLevel, StateChange};

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn subject(kind: ActorKind, risk: RiskClass, status: PlanStatus) -> PolicySubject {
        PolicySubject {
            principal: id(PrincipalId::new("uid:1000")),
            plan: ReconciliationPlan::for_policy_test(
                id(PlanId::new("plan:test")),
                id(RequestId::new("req:test")),
                Actor {
                    id: id(ActorId::new("actor:test")),
                    kind,
                    interactive: kind == ActorKind::Human,
                },
                id(ProviderId::new("systemd")),
                id(ResourceId::new("systemd:unit:test.service")),
                id(CapabilityId::new("systemd.unit.observe")),
                SemanticReason {
                    summary: "test intent".into(),
                    intent_ids: vec![id(IntentId::new("intent:test"))],
                    requirement_ids: vec![],
                    capability_ids: vec![],
                },
                "evidence:test".into(),
                risk,
                status,
                vec![StateChange {
                    key: "active_state".into(),
                    current: Some("inactive".into()),
                    desired: "active".into(),
                }],
                if status == PlanStatus::Blocked {
                    vec![PlanFinding {
                        code: "blocked".into(),
                        level: PlanFindingLevel::Blocker,
                        message: "blocked for test".into(),
                    }]
                } else {
                    vec![]
                },
            ),
        }
    }

    #[test]
    fn agent_system_mutation_requires_approval() {
        let policy = BaselinePolicy::default();
        let evaluation = policy.evaluate(&subject(
            ActorKind::Agent,
            RiskClass::SystemMutation,
            PlanStatus::ChangeProposed,
        ));
        assert!(matches!(
            evaluation.decision,
            PolicyDecision::RequireApproval { .. }
        ));
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
