#![forbid(unsafe_code)]

//! Linura's unprivileged local authority/control-plane orchestration.
//!
//! The current implemented authority surface owns authenticated authoritative
//! observation and deterministic non-executable plan previews. v0.3 adds
//! policy/risk review and short-lived exact-bound approval semantics on top of
//! that canonical plan lineage.
//!
//! The superseded 0.0.0 `Provider::plan -> ActionPlan -> ControlPlane::apply`
//! scaffold was removed rather than preserved as a legacy path. The canonical
//! eleven-stage lifecycle state machine remains in `linura-lifecycle`, and the
//! narrow executor package scaffolds remain for v0.5 qualification after v0.4
//! establishes durable prepare/recovery semantics.

mod approval;
mod approval_review;
mod plan_preview;
mod policy_review;
mod risk_classification;

pub use approval::{
    ApprovalEvidence, ApprovalIssueError, ApprovalRequirement, ApprovalRevocation,
    ApprovalValidation, AuthenticatedApprover, MAX_APPROVAL_TTL_SECONDS, validate_approval,
};
pub use approval_review::{
    ApprovalControlError, ApprovalRequirementError, ApprovalReviewControl, MAX_APPROVAL_ENTRIES,
    PolicyApprovalEvidence, PolicyApprovalIssueError, PolicyApprovalRequirement,
    PolicyAuthenticatedApprover, approval_requirement_from_evaluation, issue_policy_approval,
    validate_policy_approval,
};
pub use plan_preview::{
    AuthenticatedPrincipal, MAX_DESIRED_ATTRIBUTES, MAX_ORIGINS_PER_KIND, MAX_PREVIEW_ENTRIES,
    MAX_PREVIEW_ENTRY_BYTES, MAX_PREVIEW_TOTAL_BYTES, MAX_REQUEST_BYTES, MAX_SUMMARY_BYTES,
    MAX_TOTAL_ORIGINS, PlanPreviewControl, PlanPreviewControlError,
};
pub use policy_review::{PolicySubjectError, policy_subject_from_plan};
