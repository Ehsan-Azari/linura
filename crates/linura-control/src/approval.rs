use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter};

use linura_core::{ActorKind, ApprovalEvidenceId, ApprovalRequestId, PrincipalId};

pub const MAX_APPROVAL_TTL_SECONDS: u64 = 86_400;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequirement<B, C> {
    pub class: C,
    pub binding: B,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalEvidence<B, C> {
    pub id: ApprovalEvidenceId,
    pub request_id: ApprovalRequestId,
    pub requirement: ApprovalRequirement<B, C>,
    pub approver: PrincipalId,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

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
