use linura_approval::{
    ApprovalEvidence, ApprovalIssueError, ApprovalRequirement, ApprovalRevocation, ApprovalValidation,
    AuthenticatedApprover, validate_approval,
};
use linura_core::{ApprovalEvidenceId, ApprovalRequestId};
use linura_policy::{ApprovalClass, PolicyDecision, PolicyEvaluation};

pub type PolicyApprovalRequirement = ApprovalRequirement<PolicyEvaluation, ApprovalClass>;
pub type PolicyApprovalEvidence = ApprovalEvidence<PolicyEvaluation, ApprovalClass>;
pub type PolicyAuthenticatedApprover = AuthenticatedApprover<ApprovalClass>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalRequirementError {
    NotRequired,
    NotApprovable,
}

/// Convert one exact policy evaluation into its typed approval requirement.
///
/// Keeping this conversion in Control preserves the global layering invariant:
/// policy is not consumed or orchestrated by the generic approval domain.
pub fn approval_requirement_from_evaluation(
    evaluation: &PolicyEvaluation,
) -> Result<PolicyApprovalRequirement, ApprovalRequirementError> {
    match &evaluation.decision {
        PolicyDecision::RequireApproval { class, .. } => Ok(ApprovalRequirement {
            class: *class,
            binding: evaluation.clone(),
        }),
        PolicyDecision::Allow => Err(ApprovalRequirementError::NotRequired),
        PolicyDecision::Deny { .. } | PolicyDecision::Blocked { .. } => {
            Err(ApprovalRequirementError::NotApprovable)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn issue_policy_approval(
    id: ApprovalEvidenceId,
    request_id: ApprovalRequestId,
    evaluation: &PolicyEvaluation,
    approver: &PolicyAuthenticatedApprover,
    issued_at_unix_seconds: u64,
    expires_at_unix_seconds: u64,
) -> Result<PolicyApprovalEvidence, PolicyApprovalIssueError> {
    let requirement = approval_requirement_from_evaluation(evaluation)
        .map_err(PolicyApprovalIssueError::Requirement)?;
    ApprovalEvidence::try_issue(
        id,
        request_id,
        requirement,
        approver,
        issued_at_unix_seconds,
        expires_at_unix_seconds,
    )
    .map_err(PolicyApprovalIssueError::Evidence)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyApprovalIssueError {
    Requirement(ApprovalRequirementError),
    Evidence(ApprovalIssueError<ApprovalClass>),
}

#[must_use]
pub fn validate_policy_approval(
    evidence: &PolicyApprovalEvidence,
    evaluation: &PolicyEvaluation,
    now_unix_seconds: u64,
    revocation: Option<&ApprovalRevocation>,
) -> ApprovalValidation {
    let Ok(requirement) = approval_requirement_from_evaluation(evaluation) else {
        return ApprovalValidation::BindingMismatch;
    };
    validate_approval(evidence, &requirement, now_unix_seconds, revocation)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use linura_core::{
        Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, PolicyRevisionId, PrincipalId,
        ProviderId, RequestId, ResourceId, RiskClass, SemanticReason, ValidationError,
    };
    use linura_policy::{
        BaselinePolicy, PolicyEngine, PolicySubject, ReviewPlanStatus, ReviewedChange,
    };

    use super::*;

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn evaluation(risk: RiskClass) -> PolicyEvaluation {
        let subject = PolicySubject::try_new(
            id(PrincipalId::new("uid:1000")),
            id(PlanId::new("plan:approval-control")),
            id(RequestId::new("request:approval-control")),
            Actor {
                id: id(ActorId::new("actor:approval-control")),
                kind: ActorKind::Human,
                interactive: true,
            },
            id(ProviderId::new("systemd")),
            id(ResourceId::new("systemd:unit:test.service")),
            id(CapabilityId::new("systemd.unit.observe")),
            SemanticReason {
                summary: "review approval binding".into(),
                intent_ids: vec![id(IntentId::new("intent:approval-control"))],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            "evidence:approval-control".into(),
            risk,
            ReviewPlanStatus::ChangeProposed,
            vec![ReviewedChange {
                key: "active_state".into(),
                current: Some("inactive".into()),
                desired: "active".into(),
            }],
            vec![],
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
        BaselinePolicy::default().evaluate(&subject)
    }

    fn admin() -> PolicyAuthenticatedApprover {
        PolicyAuthenticatedApprover {
            principal: id(PrincipalId::new("uid:0")),
            kind: ActorKind::Human,
            approval_classes: BTreeSet::from([ApprovalClass::Administrator]),
        }
    }

    #[test]
    fn exact_policy_evaluation_is_the_approval_binding() {
        let evaluation = evaluation(RiskClass::SecuritySensitive);
        let evidence = issue_policy_approval(
            id(ApprovalEvidenceId::new("approval:evidence:control")),
            id(ApprovalRequestId::new("approval:request:control")),
            &evaluation,
            &admin(),
            100,
            200,
        )
        .unwrap_or_else(|error| unreachable!("{error:?}"));
        assert_eq!(
            validate_policy_approval(&evidence, &evaluation, 150, None),
            ApprovalValidation::Satisfied
        );

        let mut changed = evaluation;
        changed.binding.policy_revision_id =
            id(PolicyRevisionId::new("policy:baseline:v2"));
        assert_eq!(
            validate_policy_approval(&evidence, &changed, 150, None),
            ApprovalValidation::BindingMismatch
        );
    }

    #[test]
    fn weak_approver_cannot_satisfy_security_sensitive_requirement() {
        let evaluation = evaluation(RiskClass::SecuritySensitive);
        let weak = PolicyAuthenticatedApprover {
            principal: id(PrincipalId::new("uid:1001")),
            kind: ActorKind::Human,
            approval_classes: BTreeSet::from([ApprovalClass::InteractiveUser]),
        };
        assert!(matches!(
            issue_policy_approval(
                id(ApprovalEvidenceId::new("approval:evidence:weak-control")),
                id(ApprovalRequestId::new("approval:request:weak-control")),
                &evaluation,
                &weak,
                100,
                200,
            ),
            Err(PolicyApprovalIssueError::Evidence(
                ApprovalIssueError::MissingApprovalClass(ApprovalClass::Administrator)
            ))
        ));
    }

    #[test]
    fn blocked_and_allowed_decisions_cannot_be_minted_as_approval() {
        let mut blocked = evaluation(RiskClass::SecuritySensitive);
        blocked.decision = PolicyDecision::Blocked {
            reason: "blocked for test".into(),
        };
        assert_eq!(
            approval_requirement_from_evaluation(&blocked),
            Err(ApprovalRequirementError::NotApprovable)
        );

        let mut allowed = evaluation(RiskClass::SecuritySensitive);
        allowed.decision = PolicyDecision::Allow;
        assert_eq!(
            approval_requirement_from_evaluation(&allowed),
            Err(ApprovalRequirementError::NotRequired)
        );
    }
}
