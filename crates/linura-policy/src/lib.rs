#![forbid(unsafe_code)]

use linura_core::{ActionPlan, ActorKind, RiskClass};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalClass {
    InteractiveUser,
    Administrator,
    DestructiveAction,
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
}

pub trait PolicyEngine {
    fn evaluate(&self, plan: &ActionPlan) -> PolicyDecision;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BaselinePolicy;

impl PolicyEngine for BaselinePolicy {
    fn evaluate(&self, plan: &ActionPlan) -> PolicyDecision {
        if plan.actor.kind == ActorKind::Remote {
            return PolicyDecision::Deny {
                reason: "remote actors are disabled in the initial profile".into(),
            };
        }
        if plan.actor.kind == ActorKind::Agent && plan.risk >= RiskClass::SystemMutation {
            return PolicyDecision::RequireApproval {
                class: ApprovalClass::InteractiveUser,
                reason: "agents are untrusted proposers and cannot apply system mutations without explicit approval".into(),
            };
        }
        match plan.risk {
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
        Actor, CapabilityId, Compensation, Effect, IntentId, PlanId, Preconditions, RequestId,
        ResourceId, SemanticReason, ValidationError, Verification,
    };

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn plan(kind: ActorKind, risk: RiskClass) -> ActionPlan {
        ActionPlan {
            id: id(PlanId::new("plan:test")),
            request_id: id(RequestId::new("req:test")),
            actor: Actor {
                id: "actor".into(),
                kind,
                interactive: kind == ActorKind::Human,
            },
            resource: id(ResourceId::new("service:test")),
            capability: id(CapabilityId::new("service.manage")),
            risk,
            reason: SemanticReason {
                summary: "test intent".into(),
                intent_ids: vec![id(IntentId::new("intent:test"))],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            preconditions: Preconditions { statements: vec![] },
            effects: vec![Effect {
                id: "effect".into(),
                executor: "test".into(),
                operation: "noop".into(),
                arguments: vec![],
                compensation: Compensation::None,
            }],
            verification: vec![Verification {
                description: "verified".into(),
            }],
        }
    }

    #[test]
    fn agent_system_mutation_requires_approval() {
        let decision = BaselinePolicy.evaluate(&plan(ActorKind::Agent, RiskClass::SystemMutation));
        assert!(matches!(decision, PolicyDecision::RequireApproval { .. }));
    }

    #[test]
    fn remote_is_denied() {
        let decision = BaselinePolicy.evaluate(&plan(ActorKind::Remote, RiskClass::ReadOnly));
        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }
}
