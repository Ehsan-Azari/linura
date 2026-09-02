use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter};

use linura_core::{ActorKind, ApprovalEvidenceId, ApprovalRequestId, PrincipalId};

pub const MAX_APPROVAL_TTL_SECONDS: u64 = 86_400;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequirement<B, C> {
    class: C,
    binding: B,
}

impl<B, C> ApprovalRequirement<B, C> {
    pub(crate) fn new(class: C, binding: B) -> Self {
        Self { class, binding }
    }

    #[must_use]
    pub const fn class(&self) -> &C {
        &self.class
    }

    #[must_use]
    pub const fn binding(&self) -> &B {
        &self.binding
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedApprover<C>
where
    C: Ord,
{
    principal: PrincipalId,
    kind: ActorKind,
    approval_classes: BTreeSet<C>,
}

impl<C> AuthenticatedApprover<C>
where
    C: Copy + Ord,
{
    pub(crate) fn new(
        principal: PrincipalId,
        kind: ActorKind,
        approval_classes: BTreeSet<C>,
    ) -> Self {
        Self {
            principal,
            kind,
            approval_classes,
        }
    }

    #[must_use]
    pub fn principal(&self) -> &PrincipalId {
        &self.principal
    }

    #[must_use]
    pub fn can_satisfy(&self, class: C) -> bool {
        self.kind == ActorKind::Human && self.approval_classes.contains(&class)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalEvidence<B, C> {
    id: ApprovalEvidenceId,
    request_id: ApprovalRequestId,
    requirement: ApprovalRequirement<B, C>,
    approver: PrincipalId,
    issued_at_unix_seconds: u64,
    expires_at_unix_seconds: u64,
}

impl<B, C> ApprovalEvidence<B, C> {
    #[must_use]
    pub fn id(&self) -> &ApprovalEvidenceId {
        &self.id
    }

    #[must_use]
    pub fn request_id(&self) -> &ApprovalRequestId {
        &self.request_id
    }

    #[must_use]
    pub fn requirement(&self) -> &ApprovalRequirement<B, C> {
        &self.requirement
    }

    #[must_use]
    pub fn approver(&self) -> &PrincipalId {
        &self.approver
    }

    #[must_use]
    pub const fn issued_at_unix_seconds(&self) -> u64 {
        self.issued_at_unix_seconds
    }

    #[must_use]
    pub const fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at_unix_seconds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRevocation {
    revoked_by: PrincipalId,
    revoked_at_unix_seconds: u64,
}

impl ApprovalRevocation {
    pub(crate) fn new(revoked_by: PrincipalId, revoked_at_unix_seconds: u64) -> Self {
        Self {
            revoked_by,
            revoked_at_unix_seconds,
        }
    }

    #[must_use]
    pub fn revoked_by(&self) -> &PrincipalId {
        &self.revoked_by
    }

    #[must_use]
    pub const fn revoked_at_unix_seconds(&self) -> u64 {
        self.revoked_at_unix_seconds
    }
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
                write!(
                    f,
                    "approver does not hold required approval class {class:?}"
                )
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
    pub(crate) fn try_issue(
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
pub(crate) fn validate_approval<B, C>(
    evidence: &ApprovalEvidence<B, C>,
    current_binding: &B,
    current_class: &C,
    now_unix_seconds: u64,
    revocation: Option<&ApprovalRevocation>,
) -> ApprovalValidation
where
    B: Eq,
    C: Eq,
{
    if evidence.requirement.binding() != current_binding
        || evidence.requirement.class() != current_class
    {
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
