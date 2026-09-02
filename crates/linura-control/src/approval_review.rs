use std::collections::BTreeMap;

use crate::approval::{
    ApprovalEvidence, ApprovalIssueError, ApprovalRequirement, ApprovalRevocation,
    ApprovalValidation, AuthenticatedApprover, validate_approval,
};
use linura_core::{ApprovalEvidenceId, ApprovalRequestId, ValidationError};
use linura_policy::{ApprovalClass, PolicyDecision, PolicyEvaluation};

pub const MAX_APPROVAL_ENTRIES: usize = 256;
pub const MAX_APPROVAL_ENTRY_BYTES: usize = 256 * 1024;
pub const MAX_APPROVAL_TOTAL_BYTES: usize = 4 * 1024 * 1024;
const APPROVAL_RECORD_OVERHEAD_BYTES: usize = 256;

pub type PolicyApprovalRequirement = ApprovalRequirement<PolicyEvaluation, ApprovalClass>;
pub type PolicyApprovalEvidence = ApprovalEvidence<PolicyEvaluation, ApprovalClass>;
pub type PolicyAuthenticatedApprover = AuthenticatedApprover<ApprovalClass>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalRequirementError {
    NotRequired,
    NotApprovable,
}

/// Convert one exact policy evaluation into its typed approval requirement.
pub fn approval_requirement_from_evaluation(
    evaluation: &PolicyEvaluation,
) -> Result<PolicyApprovalRequirement, ApprovalRequirementError> {
    match &evaluation.decision {
        PolicyDecision::RequireApproval { class, .. } => Ok(ApprovalRequirement::new(
            *class,
            evaluation.clone(),
        )),
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

fn add_len(total: &mut usize, value: &str) {
    *total = total.saturating_add(value.len());
}

/// Deterministic upper-accounting estimate for one retained policy evaluation.
/// It counts all variable-size strings/IDs material to exact approval binding;
/// fixed struct/map overhead is covered separately by
/// `APPROVAL_RECORD_OVERHEAD_BYTES`.
fn approval_binding_bytes(evaluation: &PolicyEvaluation) -> usize {
    let subject = &evaluation.subject;
    let mut bytes = 0usize;

    add_len(&mut bytes, subject.principal().as_str());
    add_len(&mut bytes, subject.plan_id().as_str());
    add_len(&mut bytes, subject.request_id().as_str());
    add_len(&mut bytes, subject.actor().id.as_str());
    add_len(&mut bytes, subject.provider().as_str());
    add_len(&mut bytes, subject.resource().as_str());
    add_len(&mut bytes, subject.capability().as_str());
    add_len(&mut bytes, subject.observed_evidence_id());
    add_len(&mut bytes, &subject.reason().summary);
    for id in &subject.reason().intent_ids {
        add_len(&mut bytes, id.as_str());
    }
    for id in &subject.reason().requirement_ids {
        add_len(&mut bytes, id.as_str());
    }
    for id in &subject.reason().capability_ids {
        add_len(&mut bytes, id.as_str());
    }
    for change in subject.changes() {
        add_len(&mut bytes, &change.key);
        if let Some(current) = &change.current {
            add_len(&mut bytes, current);
        }
        add_len(&mut bytes, &change.desired);
    }
    for finding in subject.findings() {
        add_len(&mut bytes, &finding.code);
        add_len(&mut bytes, &finding.message);
    }

    add_len(&mut bytes, evaluation.binding.principal.as_str());
    add_len(&mut bytes, evaluation.binding.plan_id.as_str());
    add_len(&mut bytes, evaluation.binding.request_id.as_str());
    add_len(&mut bytes, &evaluation.binding.observed_evidence_id);
    add_len(&mut bytes, evaluation.binding.provider.as_str());
    add_len(&mut bytes, evaluation.binding.resource.as_str());
    add_len(&mut bytes, evaluation.binding.capability.as_str());
    add_len(&mut bytes, evaluation.binding.policy_id.as_str());
    add_len(&mut bytes, evaluation.binding.policy_revision_id.as_str());
    match &evaluation.decision {
        PolicyDecision::Allow => {}
        PolicyDecision::Deny { reason }
        | PolicyDecision::RequireApproval { reason, .. }
        | PolicyDecision::Blocked { reason } => add_len(&mut bytes, reason),
    }
    bytes
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApprovalRecord {
    evidence: PolicyApprovalEvidence,
    revocation: Option<ApprovalRevocation>,
    accounted_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalControlError {
    Issue(PolicyApprovalIssueError),
    InvalidEvidenceId(ValidationError),
    IdempotencyConflict,
    CapacityExceeded,
    EntryTooLarge,
    TotalCapacityExceeded,
    SequenceExhausted,
    EvidenceNotFound,
    RevokerNotAuthorized,
}

/// Bounded, process-local v0.3 approval lifecycle.
///
/// The store deliberately has no persistence, prepare record, executor handle,
/// or crash-recovery claim. Exact retries return the original evidence; request
/// ID reuse with changed authority material fails closed. Live authority
/// evidence is never silently evicted to make room for new evidence.
#[derive(Debug, Default)]
pub struct ApprovalReviewControl {
    records: BTreeMap<ApprovalEvidenceId, ApprovalRecord>,
    requests: BTreeMap<ApprovalRequestId, ApprovalEvidenceId>,
    next_evidence_sequence: u64,
    total_accounted_bytes: usize,
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

    #[must_use]
    pub const fn total_accounted_bytes(&self) -> usize {
        self.total_accounted_bytes
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
            if existing.evidence.requirement() == &requirement
                && existing.evidence.approver() == approver.principal()
                && existing.evidence.issued_at_unix_seconds() == issued_at_unix_seconds
                && existing.evidence.expires_at_unix_seconds() == expires_at_unix_seconds
            {
                return Ok(existing.evidence.clone());
            }
            return Err(ApprovalControlError::IdempotencyConflict);
        }

        if self.records.len() >= MAX_APPROVAL_ENTRIES {
            return Err(ApprovalControlError::CapacityExceeded);
        }

        let accounted_bytes = approval_binding_bytes(evaluation)
            .saturating_add(request_id.as_str().len())
            .saturating_add(approver.principal().as_str().len())
            .saturating_add(APPROVAL_RECORD_OVERHEAD_BYTES);
        if accounted_bytes > MAX_APPROVAL_ENTRY_BYTES {
            return Err(ApprovalControlError::EntryTooLarge);
        }
        let new_total = self
            .total_accounted_bytes
            .checked_add(accounted_bytes)
            .ok_or(ApprovalControlError::TotalCapacityExceeded)?;
        if new_total > MAX_APPROVAL_TOTAL_BYTES {
            return Err(ApprovalControlError::TotalCapacityExceeded);
        }

        let evidence_id = ApprovalEvidenceId::new(format!(
            "approval:evidence:{:016x}",
            self.next_evidence_sequence
        ))
        .map_err(ApprovalControlError::InvalidEvidenceId)?;
        self.next_evidence_sequence = self
            .next_evidence_sequence
            .checked_add(1)
            .ok_or(ApprovalControlError::SequenceExhausted)?;

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
                accounted_bytes,
            },
        );
        self.total_accounted_bytes = new_total;
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
        if !revoker.can_satisfy(*record.evidence.requirement().class()) {
            return Err(ApprovalControlError::RevokerNotAuthorized);
        }
        if record.revocation.is_none() {
            record.revocation = Some(ApprovalRevocation::new(
                revoker.principal().clone(),
                revoked_at_unix_seconds,
            ));
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
        PolicyAuthenticatedApprover::new(
            id(PrincipalId::new("uid:0")),
            ActorKind::Human,
            BTreeSet::from([ApprovalClass::Administrator]),
        )
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
        let weak = PolicyAuthenticatedApprover::new(
            id(PrincipalId::new("uid:1001")),
            ActorKind::Human,
            BTreeSet::from([ApprovalClass::InteractiveUser]),
        );
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
            control.validate(evidence.id(), &evaluation, 150),
            ApprovalValidation::Satisfied
        );
        control
            .revoke(evidence.id(), &admin(), 151)
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        assert_eq!(
            control.validate(evidence.id(), &evaluation, 152),
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
        let weak = PolicyAuthenticatedApprover::new(
            id(PrincipalId::new("uid:1001")),
            ActorKind::Human,
            BTreeSet::from([ApprovalClass::InteractiveUser]),
        );
        assert_eq!(
            control.revoke(evidence.id(), &weak, 150),
            Err(ApprovalControlError::RevokerNotAuthorized)
        );
    }

    #[test]
    fn retained_bytes_are_bounded_and_idempotent_retry_does_not_grow_store() {
        let evaluation = evaluation(RiskClass::SecuritySensitive);
        let request = id(ApprovalRequestId::new("approval:request:bytes"));
        let mut control = ApprovalReviewControl::default();
        let _ = control
            .issue(request.clone(), &evaluation, &admin(), 100, 200)
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        let first_bytes = control.total_accounted_bytes();
        assert!(first_bytes > 0);
        assert!(first_bytes <= MAX_APPROVAL_TOTAL_BYTES);
        let _ = control
            .issue(request, &evaluation, &admin(), 100, 200)
            .unwrap_or_else(|error| unreachable!("{error:?}"));
        assert_eq!(control.total_accounted_bytes(), first_bytes);
    }
}
