#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter};

use linura_core::{ActorKind, ApprovalEvidenceId, ApprovalRequestId, PrincipalId};

/// v0.3 approval evidence is intentionally short-lived and process-local.
/// Durable authorization belongs to v0.4.
pub const MAX_APPROVAL_TTL_SECONDS: u64 = 86_400;

/// The exact authority binding and approval class that must be satisfied.
///
/// `B` is deliberately generic so this low-level approval domain never depends
/// on policy, transport, planner, provider, or executor crates. Linura Control
/// binds it to the complete `PolicyEvaluation` produced by the policy domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequirement<B, C> {
    pub class: C,
    pub binding: B,
}

/// Trusted local authentication metadata for an approval issuer.
///
/// This is supplied by Linura Control after local authentication. It is not a
/// client-asserted wire claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedApprover<C>
where
    C: Ord,
{
    pub principal: PrincipalId,
    pub kind: ActorKind,
    pub approval_classes: BTreeSet<C>,
}

impl<C> AuthenticatedApprover<C>
where
    C: Copy + Ord,
{
    #[must_use]
    pub fn can_satisfy(&self, class: C) -> bool {
        self.kind == ActorKind::Human && self.approval_classes.contains(&class)
    }
}

/// Immutable, short-lived proof that one authenticated human principal
/// satisfied one exact approval requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalEvidence<B, C> {
    pub id: ApprovalEvidenceId,
    pub request_id: ApprovalRequestId,
    pub requirement: ApprovalRequirement<B, C>,
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
pub enum ApprovalIssueError<C> {
    NonHumanApprover,
    MissingApprovalClass(C),
    InvalidValidityWindow,
    ValidityWindowTooLong,
}

impl<C> Display for ApprovalIssueError<C>
where
    C: Debug,
{
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

impl<C> std::error::Error for ApprovalIssueError<C> where C: Debug {}

impl<B, C> ApprovalEvidence<B, C>
where
    C: Copy + Debug + Eq + Ord,
{
    #[allow(clippy::too_many_arguments)]
    pub fn try_issue(
        id: ApprovalEvidenceId,
        request_id: ApprovalRequestId,
        requirement: ApprovalRequirement<B, C>,
        approver: &AuthenticatedApprover<C>,
        issued_at_unix_seconds: u64,
        expires_at_unix_seconds: u64,
    ) -> Result<Self, ApprovalIssueError<C>> {
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
/// Equality deliberately compares the complete binding rather than only IDs.
/// Any material authority change invalidates the evidence and requires a fresh
/// approval.
#[must_use]
pub fn validate_approval<B, C>(
    evidence: &ApprovalEvidence<B, C>,
    current_requirement: &ApprovalRequirement<B, C>,
    now_unix_seconds: u64,
    revocation: Option<&ApprovalRevocation>,
) -> ApprovalValidation
where
    B: Eq,
    C: Eq,
{
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
    use super::*;
    use linura_core::ValidationError;

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum TestClass {
        Interactive,
        Administrator,
    }

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn requirement() -> ApprovalRequirement<&'static str, TestClass> {
        ApprovalRequirement {
            class: TestClass::Administrator,
            binding: "exact-policy-evaluation",
        }
    }

    fn approver(class: TestClass) -> AuthenticatedApprover<TestClass> {
        AuthenticatedApprover {
            principal: id(PrincipalId::new("uid:0")),
            kind: ActorKind::Human,
            approval_classes: BTreeSet::from([class]),
        }
    }

    fn issued() -> ApprovalEvidence<&'static str, TestClass> {
        ApprovalEvidence::try_issue(
            id(ApprovalEvidenceId::new("approval:evidence:1")),
            id(ApprovalRequestId::new("approval:request:1")),
            requirement(),
            &approver(TestClass::Administrator),
            100,
            200,
        )
        .unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn non_human_principals_cannot_issue_approval() {
        let machine = AuthenticatedApprover {
            principal: id(PrincipalId::new("service:policy")),
            kind: ActorKind::Service,
            approval_classes: BTreeSet::from([TestClass::Administrator]),
        };
        assert_eq!(
            ApprovalEvidence::try_issue(
                id(ApprovalEvidenceId::new("approval:evidence:service")),
                id(ApprovalRequestId::new("approval:request:service")),
                requirement(),
                &machine,
                100,
                200,
            ),
            Err(ApprovalIssueError::NonHumanApprover)
        );
    }

    #[test]
    fn wrong_approval_class_fails_closed() {
        assert_eq!(
            ApprovalEvidence::try_issue(
                id(ApprovalEvidenceId::new("approval:evidence:weak")),
                id(ApprovalRequestId::new("approval:request:weak")),
                requirement(),
                &approver(TestClass::Interactive),
                100,
                200,
            ),
            Err(ApprovalIssueError::MissingApprovalClass(
                TestClass::Administrator
            ))
        );
    }

    #[test]
    fn evidence_is_exact_bound() {
        let evidence = issued();
        let changed = ApprovalRequirement {
            class: TestClass::Administrator,
            binding: "changed-policy-evaluation",
        };
        assert_eq!(
            validate_approval(&evidence, &changed, 150, None),
            ApprovalValidation::BindingMismatch
        );
    }

    #[test]
    fn expiry_revocation_and_future_issuance_fail_closed() {
        let evidence = issued();
        assert_eq!(
            validate_approval(&evidence, &requirement(), 99, None),
            ApprovalValidation::NotYetValid
        );
        assert_eq!(
            validate_approval(&evidence, &requirement(), 200, None),
            ApprovalValidation::Expired
        );
        assert_eq!(
            validate_approval(
                &evidence,
                &requirement(),
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
    fn validity_windows_are_bounded() {
        assert_eq!(
            ApprovalEvidence::try_issue(
                id(ApprovalEvidenceId::new("approval:evidence:long")),
                id(ApprovalRequestId::new("approval:request:long")),
                requirement(),
                &approver(TestClass::Administrator),
                100,
                100 + MAX_APPROVAL_TTL_SECONDS + 1,
            ),
            Err(ApprovalIssueError::ValidityWindowTooLong)
        );
    }
}
