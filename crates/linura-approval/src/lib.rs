#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use linura_core::{
    ActorKind, ApprovalEvidenceId, ApprovalRequestId, PrincipalId,
};
use linura_policy::{ApprovalClass, PolicyDecision, PolicyEvaluation};

/// v0.3 approval evidence is intentionally short-lived and process-local.
/// Durable authorization belongs to v0.4.
pub const MAX_APPROVAL_TTL_SECONDS: u64 = 86_400;

/// The exact policy evaluation and approval class that must be satisfied.
///
/// Retaining the complete evaluation makes approval binding sensitive to the
/// authenticated principal, request/plan identity, authoritative evidence,
/// provider/resource/capability, material changes/findings, classified risk,
/// risk-policy provenance, and policy revision carried by that evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequirement {
    pub class: ApprovalClass,
    pub evaluation: PolicyEvaluation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalRequirementError {
    NotRequired,
    NotApprovable,
}

impl Display for ApprovalRequirementError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRequired => f.write_str("policy evaluation does not require approval"),
            Self::NotApprovable => {
                f.write_str("denied or blocked policy evaluation cannot become approvable")
            }
        }
    }
}

impl std::error::Error for ApprovalRequirementError {}

impl ApprovalRequirement {
    pub fn try_from_evaluation(
        evaluation: &PolicyEvaluation,
    ) -> Result<Self, ApprovalRequirementError> {
        match &evaluation.decision {
            PolicyDecision::RequireApproval { class, .. } => Ok(Self {
                class: *class,
                evaluation: evaluation.clone(),
            }),
            PolicyDecision::Allow => Err(ApprovalRequirementError::NotRequired),
            PolicyDecision::Deny { .. } | PolicyDecision::Blocked { .. } => {
                Err(ApprovalRequirementError::NotApprovable)
            }
        }
    }
}

/// Trusted local authentication metadata for an approval issuer.
///
/// This type is an authority input supplied by Control after local
/// authentication; it is not a client-asserted wire claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedApprover {
    pub principal: PrincipalId,
    pub kind: ActorKind,
    pub approval_classes: BTreeSet<ApprovalClass>,
}

impl AuthenticatedApprover {
    #[must_use]
    pub fn can_satisfy(&self, class: ApprovalClass) -> bool {
        self.kind == ActorKind::Human && self.approval_classes.contains(&class)
    }
}

/// Immutable, short-lived proof that one authenticated human principal
/// satisfied one exact approval requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalEvidence {
    pub id: ApprovalEvidenceId,
    pub request_id: ApprovalRequestId,
    pub requirement: ApprovalRequirement,
    pub approver: PrincipalId,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

/// Separate revocation state prevents an immutable evidence object from being
/// rewritten after issuance. v0.3 may retain this only in process memory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRevocation {
    pub revoked_by: PrincipalId,
    pub revoked_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalIssueError {
    NonHumanApprover,
    MissingApprovalClass(ApprovalClass),
    InvalidValidityWindow,
    ValidityWindowTooLong,
}

impl Display for ApprovalIssueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonHumanApprover => {
                f.write_str("only an authenticated human principal can issue v0.3 approval")
            }
            Self::MissingApprovalClass(class) => {
                write!(f, "approver does not hold required approval class {class:?}")
            }
            Self::InvalidValidityWindow => {
                f.write_str("approval expiry must be strictly after issuance")
            }
            Self::ValidityWindowTooLong => write!(
                f,
                "approval validity exceeds the v0.3 maximum of {MAX_APPROVAL_TTL_SECONDS} seconds"
            ),
        }
    }
}

impl std::error::Error for ApprovalIssueError {}

impl ApprovalEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn try_issue(
        id: ApprovalEvidenceId,
        request_id: ApprovalRequestId,
        requirement: ApprovalRequirement,
        approver: &AuthenticatedApprover,
        issued_at_unix_seconds: u64,
        expires_at_unix_seconds: u64,
    ) -> Result<Self, ApprovalIssueError> {
        if approver.kind != ActorKind::Human {
            return Err(ApprovalIssueError::NonHumanApprover);
        }
        if !approver.can_satisfy(requirement.class) {
            return Err(ApprovalIssueError::MissingApprovalClass(requirement.class));
        }
        if expires_at_unix_seconds <= issued_at_unix_seconds {
            return Err(ApprovalIssueError::InvalidValidityWindow);
        }
        if expires_at_unix_seconds - issued_at_unix_seconds > MAX_APPROVAL_TTL_SECONDS {
            return Err(ApprovalIssueError::ValidityWindowTooLong);
        }

        Ok(Self {
            id,
            request_id,
            requirement,
            approver: approver.principal.clone(),
            issued_at_unix_seconds,
            expires_at_unix_seconds,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalValidation {
    Satisfied,
    BindingMismatch,
    NotYetValid,
    Expired,
    Revoked,
}

/// Validate immutable evidence against the current exact requirement.
///
/// Equality of `ApprovalRequirement` deliberately compares the complete policy
/// evaluation rather than only IDs. Any material review change invalidates the
/// evidence and requires a new approval.
#[must_use]
pub fn validate_approval(
    evidence: &ApprovalEvidence,
    current_requirement: &ApprovalRequirement,
    now_unix_seconds: u64,
    revocation: Option<&ApprovalRevocation>,
) -> ApprovalValidation {
    if &evidence.requirement != current_requirement {
        return ApprovalValidation::BindingMismatch;
    }
    if now_unix_seconds < evidence.issued_at_unix_seconds {
        return ApprovalValidation::NotYetValid;
    }
    if now_unix_seconds >= evidence.expires_at_unix_seconds {
        return ApprovalValidation::Expired;
    }
    if revocation.is_some() {
        return ApprovalValidation::Revoked;
    }
    ApprovalValidation::Satisfied
}

#[cfg(test)]
mod tests {
    use linura_core::{
        Actor, ActorId, CapabilityId, IntentId, PlanId, PolicyId, PolicyRevisionId, ProviderId,
        RequestId, ResourceId, RiskClass, SemanticReason, ValidationError,
    };
    use linura_policy::{
        BaselinePolicy, PolicyEngine, PolicySubject, ReviewPlanStatus, ReviewedChange,
    };

    use super::*;

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn evaluation(kind: ActorKind, risk: RiskClass) -> PolicyEvaluation {
        let subject = PolicySubject::try_new(
            id(PrincipalId::new("uid:1000")),
            id(PlanId::new("plan:approval")),
            id(RequestId::new("request:approval")),
            Actor {
                id: id(ActorId::new("actor:approval")),
                kind,
                interactive: kind == ActorKind::Human,
            },
            id(ProviderId::new("systemd")),
            id(ResourceId::new("systemd:unit:test.service")),
            id(CapabilityId::new("systemd.unit.observe")),
            SemanticReason {
                summary: "exercise approval lifecycle".into(),
                intent_ids: vec![id(IntentId::new("intent:approval"))],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            "evidence:approval".into(),
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

    fn approver(class: ApprovalClass) -> AuthenticatedApprover {
        AuthenticatedApprover {
            principal: id(PrincipalId::new("uid:0")),
            kind: ActorKind::Human,
            approval_classes: BTreeSet::from([class]),
        }
    }

    fn issued(requirement: ApprovalRequirement) -> ApprovalEvidence {
        ApprovalEvidence::try_issue(
            id(ApprovalEvidenceId::new("approval:evidence:1")),
            id(ApprovalRequestId::new("approval:request:1")),
            requirement.clone(),
            &approver(requirement.class),
            100,
            200,
        )
        .unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn requirement_is_derived_only_from_require_approval_decision() {
        let evaluation = evaluation(ActorKind::Human, RiskClass::SecuritySensitive);
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(requirement.class, ApprovalClass::Administrator);
        assert_eq!(requirement.evaluation, evaluation);
    }

    #[test]
    fn non_human_principals_cannot_issue_approval() {
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation(
            ActorKind::Human,
            RiskClass::SystemMutation,
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        let machine = AuthenticatedApprover {
            principal: id(PrincipalId::new("service:policy")),
            kind: ActorKind::Service,
            approval_classes: BTreeSet::from([ApprovalClass::InteractiveUser]),
        };
        assert_eq!(
            ApprovalEvidence::try_issue(
                id(ApprovalEvidenceId::new("approval:evidence:service")),
                id(ApprovalRequestId::new("approval:request:service")),
                requirement,
                &machine,
                100,
                200,
            ),
            Err(ApprovalIssueError::NonHumanApprover)
        );
    }

    #[test]
    fn wrong_approval_class_fails_closed() {
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation(
            ActorKind::Human,
            RiskClass::SecuritySensitive,
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        let weak = approver(ApprovalClass::InteractiveUser);
        assert_eq!(
            ApprovalEvidence::try_issue(
                id(ApprovalEvidenceId::new("approval:evidence:weak")),
                id(ApprovalRequestId::new("approval:request:weak")),
                requirement,
                &weak,
                100,
                200,
            ),
            Err(ApprovalIssueError::MissingApprovalClass(
                ApprovalClass::Administrator
            ))
        );
    }

    #[test]
    fn evidence_is_exact_bound_to_the_evaluation() {
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation(
            ActorKind::Human,
            RiskClass::SecuritySensitive,
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        let evidence = issued(requirement.clone());

        let mut changed = requirement;
        changed.evaluation.binding.policy_revision_id =
            id(PolicyRevisionId::new("policy:baseline:v2"));
        changed.evaluation.binding.policy_id = id(PolicyId::new("policy:baseline"));

        assert_eq!(
            validate_approval(&evidence, &changed, 150, None),
            ApprovalValidation::BindingMismatch
        );
    }

    #[test]
    fn expiry_and_revocation_fail_closed() {
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation(
            ActorKind::Human,
            RiskClass::SystemMutation,
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        let evidence = issued(requirement.clone());

        assert_eq!(
            validate_approval(&evidence, &requirement, 200, None),
            ApprovalValidation::Expired
        );
        assert_eq!(
            validate_approval(
                &evidence,
                &requirement,
                150,
                Some(&ApprovalRevocation {
                    revoked_by: id(PrincipalId::new("uid:0")),
                    revoked_at_unix_seconds: 140,
                }),
            ),
            ApprovalValidation::Revoked
        );
    }

    #[test]
    fn evidence_not_yet_valid_fails_closed() {
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation(
            ActorKind::Human,
            RiskClass::SystemMutation,
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        let evidence = issued(requirement.clone());
        assert_eq!(
            validate_approval(&evidence, &requirement, 99, None),
            ApprovalValidation::NotYetValid
        );
    }

    #[test]
    fn validity_windows_are_bounded() {
        let requirement = ApprovalRequirement::try_from_evaluation(&evaluation(
            ActorKind::Human,
            RiskClass::SystemMutation,
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            ApprovalEvidence::try_issue(
                id(ApprovalEvidenceId::new("approval:evidence:long")),
                id(ApprovalRequestId::new("approval:request:long")),
                requirement.clone(),
                &approver(requirement.class),
                100,
                100 + MAX_APPROVAL_TTL_SECONDS + 1,
            ),
            Err(ApprovalIssueError::ValidityWindowTooLong)
        );
    }
}
