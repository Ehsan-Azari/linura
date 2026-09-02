use std::collections::BTreeMap;

use crate::approval::{
    ApprovalEvidence, ApprovalIssueError, ApprovalRequirement, ApprovalRevocation,
    ApprovalValidation, AuthenticatedApprover, validate_approval,
};
use linura_core::{ApprovalEvidenceId, ApprovalRequestId, PrincipalId, ValidationError};
use linura_policy::{ApprovalClass, PolicyDecision, PolicyEvaluation};

pub const MAX_APPROVAL_ENTRIES: usize = 256;

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApprovalRecord {
    evidence: PolicyApprovalEvidence,
    revocation: Option<ApprovalRevocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalControlError {
    Issue(PolicyApprovalIssueError),
    InvalidEvidenceId(ValidationError),
    IdempotencyConflict,
    CapacityExceeded,
    EvidenceNotFound,
    RevokerNotAuthorized,
}

/// Bounded, process-local v0.3 approval lifecycle.
///
/// The store deliberately has no persistence, prepare record, executor handle,
/// or crash-recovery claim. Repeating the same approval request with identical
/// normalized inputs returns the original evidence; reusing the request ID with
/// different material fails closed.
#[derive(Debug, Default)]
pub struct ApprovalReviewControl {
    records: BTreeMap<ApprovalEvidenceId, ApprovalRecord>,
    requests: BTreeMap<ApprovalRequestId, ApprovalEvidenceId>,
    next_evidence_sequence: u64,
}

impl ApprovalReviewControl {
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn issue(
        &mut self,
        request_id: ApprovalRequestId,
        evaluation: &PolicyEvaluation,
        approver: &PolicyAuthenticatedApprover,
        issued_at_unix_seconds: u64,
        expires_at_unix_seconds: u64,
    ) -> Result<PolicyApprovalEvidence, ApprovalControlError> {
        let requirement = approval_requirement_from_evaluation(evaluation).map_err(|error| {
            ApprovalControlError::Issue(PolicyApprovalIssueError::Requirement(error))
        })?;

        if let Some(existing_id) = self.requests.get(&request_id) {
            let Some(existing) = self.records.get(existing_id) else {
                return Err(ApprovalControlError::IdempotencyConflict);
            };
            if existing.evidence.requirement == requirement
                && existing.evidence.approver == approver.principal
                && existing.evidence.issued_at_unix_seconds == issued_at_unix_seconds
                && existing.evidence.expires_at_unix_seconds == expires_at_unix_seconds
            {
                return Ok(existing.evidence.clone());
            }
            return Err(ApprovalControlError::IdempotencyConflict);
        }

        if self.records.len() >= MAX_APPROVAL_ENTRIES {
            return Err(ApprovalControlError::CapacityExceeded);
        }

        let evidence_id = ApprovalEvidenceId::new(format!(
            "approval:evidence:{:016x}",
            self.next_evidence_sequence
        ))
        .map_err(ApprovalControlError::InvalidEvidenceId)?;
        self.next_evidence_sequence = self.next_evidence_sequence.wrapping_add(1);

        let evidence = ApprovalEvidence::try_issue(
            evidence_id.clone(),
            request_id.clone(),
            requirement,
            approver,
            issued_at_unix_seconds,
            expires_at_unix_seconds,
        )
        .map_err(|error| ApprovalControlError::Issue(PolicyApprovalIssueError::Evidence(error)))?;

        self.requests.insert(request_id, evidence_id.clone());
        self.records.insert(
            evidence_id,
            ApprovalRecord {
                evidence: evidence.clone(),
                revocation: None,
            },
        );
        Ok(evidence)
    }

    #[must_use]
    pub fn validate(
        &self,
        evidence_id: &ApprovalEvidenceId,
        evaluation: &PolicyEvaluation,
        now_unix_seconds: u64,
    ) -> ApprovalValidation {
        let Some(record) = self.records.get(evidence_id) else {
            return ApprovalValidation::BindingMismatch;
        };
        validate_policy_approval(
            &record.evidence,
            evaluation,
            now_unix_seconds,
            record.revocation.as_ref(),
        )
    }

    pub fn revoke(
        &mut self,
        evidence_id: &ApprovalEvidenceId,
        revoker: &PolicyAuthenticatedApprover,
        revoked_at_unix_seconds: u64,
    ) -> Result<(), ApprovalControlError> {
        let Some(record) = self.records.get_mut(evidence_id) else {
            return Err(ApprovalControlError::EvidenceNotFound);
        };
        if !revoker.can_satisfy(record.evidence.requirement.class) {
            return Err(ApprovalControlError::RevokerNotAuthorized);
        }
        if record.revocation.is_none() {
            record.revocation = Some(ApprovalRevocation {
                revoked_by: revoker.principal.clone(),
                revoked_at_unix_seconds,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn get(&self, evidence_id: &ApprovalEvidenceId) -> Option<&PolicyApprovalEvidence> {
        self.records.get(evidence_id).map(|record| &record.evidence)
    }
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
        changed.binding.policy_revision_id = id(PolicyRevisionId::new("policy:baseline:v2"));
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

    #[test]
    fn process_local_issue_is_idempotent_and_conflicts_fail_closed() {
        let evaluation = evaluation(RiskClass::SecuritySensitive);
        let request = id(ApprovalRequestId::new("approval:request:idempotent"));
        let mut control = ApprovalReviewControl::default();
        let first = control
            .issue(request.clone(), &evaluation, &admin(), 100, 200)
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        let retry = control
            .issue(request.clone(), &evaluation, &admin(), 100, 200)
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        assert_eq!(first, retry);
        assert_eq!(control.len(), 1);
        assert_eq!(
            control.issue(request, &evaluation, &admin(), 100, 201),
            Err(ApprovalControlError::IdempotencyConflict)
        );
    }

    #[test]
    fn process_local_revocation_is_fail_closed() {
        let evaluation = evaluation(RiskClass::SecuritySensitive);
        let mut control = ApprovalReviewControl::default();
        let evidence = control
            .issue(
                id(ApprovalRequestId::new("approval:request:revoke")),
                &evaluation,
                &admin(),
                100,
                200,
            )
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        assert_eq!(
            control.validate(&evidence.id, &evaluation, 150),
            ApprovalValidation::Satisfied
        );
        control
            .revoke(&evidence.id, &admin(), 151)
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        assert_eq!(
            control.validate(&evidence.id, &evaluation, 152),
            ApprovalValidation::Revoked
        );
    }

    #[test]
    fn unqualified_revoker_is_rejected() {
        let evaluation = evaluation(RiskClass::SecuritySensitive);
        let mut control = ApprovalReviewControl::default();
        let evidence = control
            .issue(
                id(ApprovalRequestId::new("approval:request:revoker")),
                &evaluation,
                &admin(),
                100,
                200,
            )
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        let weak = PolicyAuthenticatedApprover {
            principal: id(PrincipalId::new("uid:1001")),
            kind: ActorKind::Human,
            approval_classes: BTreeSet::from([ApprovalClass::InteractiveUser]),
        };
        assert_eq!(
            control.revoke(&evidence.id, &weak, 150),
            Err(ApprovalControlError::RevokerNotAuthorized)
        );
    }
}
