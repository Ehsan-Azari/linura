from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one replacement target, found {count}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


def write(path: str, content: str) -> None:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content, encoding="utf-8")


# ---- Control: retained canonical plan -> trusted review -> public projection ----
replace_once(
    "crates/linura-control/src/plan_preview.rs",
    "    Observation { reason: String },\n    Planning { reason: String },\n    Retention { reason: String },",
    "    Observation { reason: String },\n    Planning { reason: String },\n    Review { reason: String },\n    Retention { reason: String },",
)
replace_once(
    "crates/linura-control/src/plan_preview.rs",
    "            Self::Planning { reason } => write!(f, \"deterministic planning failed: {reason}\"),\n            Self::Retention { reason } => write!(f, \"plan preview retention failed: {reason}\"),",
    "            Self::Planning { reason } => write!(f, \"deterministic planning failed: {reason}\"),\n            Self::Review { reason } => write!(f, \"trusted plan review failed: {reason}\"),\n            Self::Retention { reason } => write!(f, \"plan preview retention failed: {reason}\"),",
)
replace_once(
    "crates/linura-control/src/plan_preview.rs",
    "    pub fn get_plan_preview(\n        &self,\n        principal: &AuthenticatedPrincipal,\n        plan_id: &PlanId,\n    ) -> Result<PlanPreview, PlanPreviewControlError> {",
    "    pub(crate) fn retained_plan(\n        &self,\n        principal: &AuthenticatedPrincipal,\n        plan_id: &PlanId,\n    ) -> Result<ReconciliationPlan, PlanPreviewControlError> {\n        self.previews.get(principal, plan_id).ok_or_else(|| {\n            PlanPreviewControlError::NotRetained {\n                plan_id: plan_id.clone(),\n            }\n        })\n    }\n\n    pub fn get_plan_preview(\n        &self,\n        principal: &AuthenticatedPrincipal,\n        plan_id: &PlanId,\n    ) -> Result<PlanPreview, PlanPreviewControlError> {",
)
replace_once(
    "crates/linura-control/src/policy_review.rs",
    "#[cfg_attr(\n    not(test),\n    allow(\n        dead_code,\n        reason = \"sealed until retained-plan Control orchestration is exposed; making this public would permit fabricated authority material\"\n    )\n)]\npub(crate) fn review_plan(",
    "pub(crate) fn review_plan(",
)
replace_once(
    "crates/linura-control/src/policy_review.rs",
    "#[cfg_attr(\n    not(test),\n    allow(\n        dead_code,\n        reason = \"internal trusted-review constructor used by authority tests and the retained-plan review path\"\n    )\n)]\npub(crate) fn review_subject_for_control(subject: PolicySubject) -> TrustedPolicyReview {",
    "pub(crate) fn review_subject_for_control(subject: PolicySubject) -> TrustedPolicyReview {",
)
replace_once(
    "crates/linura-control/src/lib.rs",
    "mod risk_classification;",
    "mod risk_classification;\nmod review_projection;",
)

write(
    "crates/linura-control/src/review_projection.rs",
    r'''use linura_core::PlanId;
use linura_policy::{ApprovalClass, PolicyDecision, ReviewFindingLevel, ReviewPlanStatus};
use linura_protocol::{
    PlanPreviewChange, PlanPreviewFinding, PlanPreviewFindingLevel, PlanPreviewStatus, PlanReview,
    PlanReviewApprovalClass, PlanReviewDecision,
};

use crate::{AuthenticatedPrincipal, PlanPreviewControl, PlanPreviewControlError, TrustedPolicyReview};

impl PlanPreviewControl {
    /// Review only canonical plan material retained by this Control instance for
    /// the authenticated principal. Public callers supply identity, never plan
    /// material, policy subjects, policy decisions, or authority risk.
    pub fn review_plan(
        &self,
        principal: &AuthenticatedPrincipal,
        plan_id: &PlanId,
    ) -> Result<PlanReview, PlanPreviewControlError> {
        let plan = self.retained_plan(principal, plan_id)?;
        let review = crate::policy_review::review_plan(principal, &plan).map_err(|error| {
            PlanPreviewControlError::Review {
                reason: format!("{error:?}"),
            }
        })?;
        Ok(plan_review_projection(plan.prospective_risk, &review))
    }

    /// Deterministic explanation is the exact same authority projection as
    /// review. v0.3 retains no separate durable authorization state.
    pub fn explain_plan_review(
        &self,
        principal: &AuthenticatedPrincipal,
        plan_id: &PlanId,
    ) -> Result<PlanReview, PlanPreviewControlError> {
        self.review_plan(principal, plan_id)
    }
}

fn plan_review_projection(
    planner_risk_floor: linura_core::RiskClass,
    review: &TrustedPolicyReview,
) -> PlanReview {
    let subject = review.subject();
    let binding = review.binding();
    let (decision, approval_class, decision_reason) = match review.decision() {
        PolicyDecision::Allow => (PlanReviewDecision::Allow, None, String::new()),
        PolicyDecision::Deny { reason } => (PlanReviewDecision::Deny, None, reason.clone()),
        PolicyDecision::RequireApproval { class, reason } => (
            PlanReviewDecision::RequireApproval,
            Some(map_approval_class(*class)),
            reason.clone(),
        ),
        PolicyDecision::Blocked { reason } => {
            (PlanReviewDecision::Blocked, None, reason.clone())
        }
    };

    PlanReview {
        plan_id: subject.plan_id().clone(),
        request_id: subject.request_id().clone(),
        principal: subject.principal().clone(),
        actor: subject.actor().clone(),
        provider: subject.provider().clone(),
        resource: subject.resource().clone(),
        observation_capability: subject.capability().clone(),
        reason: subject.reason().clone(),
        observed_evidence_id: subject.observed_evidence_id().to_owned(),
        planner_risk_floor,
        reviewed_risk: subject.prospective_risk(),
        status: map_status(subject.status()),
        policy_id: binding.policy_id.clone(),
        policy_revision_id: binding.policy_revision_id.clone(),
        decision,
        approval_class,
        decision_reason,
        execution_authorized: false,
        changes: subject
            .changes()
            .iter()
            .map(|change| PlanPreviewChange {
                key: change.key.clone(),
                current: change.current.clone(),
                desired: change.desired.clone(),
            })
            .collect(),
        findings: subject
            .findings()
            .iter()
            .map(|finding| PlanPreviewFinding {
                code: finding.code.clone(),
                level: map_finding_level(finding.level),
                message: finding.message.clone(),
            })
            .collect(),
    }
}

const fn map_status(status: ReviewPlanStatus) -> PlanPreviewStatus {
    match status {
        ReviewPlanStatus::NoChange => PlanPreviewStatus::NoChange,
        ReviewPlanStatus::ChangeProposed => PlanPreviewStatus::ChangeProposed,
        ReviewPlanStatus::Blocked => PlanPreviewStatus::Blocked,
    }
}

const fn map_finding_level(level: ReviewFindingLevel) -> PlanPreviewFindingLevel {
    match level {
        ReviewFindingLevel::Pass => PlanPreviewFindingLevel::Pass,
        ReviewFindingLevel::Warning => PlanPreviewFindingLevel::Warning,
        ReviewFindingLevel::Blocker => PlanPreviewFindingLevel::Blocker,
    }
}

const fn map_approval_class(class: ApprovalClass) -> PlanReviewApprovalClass {
    match class {
        ApprovalClass::InteractiveUser => PlanReviewApprovalClass::InteractiveUser,
        ApprovalClass::Administrator => PlanReviewApprovalClass::Administrator,
        ApprovalClass::DestructiveAction => PlanReviewApprovalClass::DestructiveAction,
    }
}

#[cfg(test)]
mod tests {
    use linura_core::{
        Actor, ActorId, ActorKind, CapabilityId, PlanId, PrincipalId, ProviderId, RequestId,
        ResourceId, RiskClass, SemanticReason,
    };
    use linura_policy::{PolicySubject, ReviewPlanStatus, ReviewedChange};

    use super::*;
    use crate::policy_review::review_subject_for_control;

    #[test]
    fn projection_is_non_executable_and_typed() {
        let subject = PolicySubject::try_new(
            PrincipalId::new("unix:uid:1000").unwrap_or_else(|error| unreachable!("{error}")),
            PlanId::new("request:test").unwrap_or_else(|error| unreachable!("{error}")),
            RequestId::new("request:test").unwrap_or_else(|error| unreachable!("{error}")),
            Actor {
                id: ActorId::new("actor:test").unwrap_or_else(|error| unreachable!("{error}")),
                kind: ActorKind::Service,
                interactive: false,
            },
            ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}")),
            ResourceId::new("systemd:unit:test.service")
                .unwrap_or_else(|error| unreachable!("{error}")),
            CapabilityId::new("systemd.unit.observe")
                .unwrap_or_else(|error| unreachable!("{error}")),
            SemanticReason {
                summary: "review test".into(),
                intent_ids: vec![],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            "evidence:test".into(),
            RiskClass::SecuritySensitive,
            ReviewPlanStatus::ChangeProposed,
            vec![ReviewedChange {
                key: "active_state".into(),
                current: Some("inactive".into()),
                desired: "active".into(),
            }],
            vec![],
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
        let review = review_subject_for_control(subject);
        let projection = plan_review_projection(RiskClass::SystemMutation, &review);
        assert_eq!(projection.planner_risk_floor, RiskClass::SystemMutation);
        assert_eq!(projection.reviewed_risk, RiskClass::SecuritySensitive);
        assert_eq!(projection.decision, PlanReviewDecision::RequireApproval);
        assert_eq!(
            projection.approval_class,
            Some(PlanReviewApprovalClass::Administrator)
        );
        assert!(!projection.execution_authorized);
    }
}
''',
)

# ---- Protocol + SDK ----
replace_once(
    "crates/linura-protocol/src/lib.rs",
    "    Actor, Capability, CapabilityId, IntentId, PlanId, ProviderId, RequestId, ResourceId,\n    RiskClass, SemanticReason, SetupId,",
    "    Actor, Capability, CapabilityId, IntentId, PlanId, PolicyId, PolicyRevisionId, PrincipalId,\n    ProviderId, RequestId, ResourceId, RiskClass, SemanticReason, SetupId,",
)
protocol_insert = r'''
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanReviewApprovalClass {
    InteractiveUser,
    Administrator,
    DestructiveAction,
}

impl PlanReviewApprovalClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InteractiveUser => "interactive-user",
            Self::Administrator => "administrator",
            Self::DestructiveAction => "destructive-action",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanReviewDecision {
    Allow,
    Deny,
    RequireApproval,
    Blocked,
}

impl PlanReviewDecision {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::RequireApproval => "require-approval",
            Self::Blocked => "blocked",
        }
    }
}

/// Public, transport-neutral projection of Control's trusted review of one
/// retained canonical plan. This is explanation/authorization evidence only;
/// it deliberately contains no executor handle, grant, prepare record, or
/// conversion into an external effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanReview {
    pub plan_id: PlanId,
    pub request_id: RequestId,
    pub principal: PrincipalId,
    pub actor: Actor,
    pub provider: ProviderId,
    pub resource: ResourceId,
    pub observation_capability: CapabilityId,
    pub reason: SemanticReason,
    pub observed_evidence_id: String,
    pub planner_risk_floor: RiskClass,
    pub reviewed_risk: RiskClass,
    pub status: PlanPreviewStatus,
    pub policy_id: PolicyId,
    pub policy_revision_id: PolicyRevisionId,
    pub decision: PlanReviewDecision,
    pub approval_class: Option<PlanReviewApprovalClass>,
    pub decision_reason: String,
    pub execution_authorized: bool,
    pub changes: Vec<PlanPreviewChange>,
    pub findings: Vec<PlanPreviewFinding>,
}

'''
replace_once(
    "crates/linura-protocol/src/lib.rs",
    "#[derive(Clone, Debug, Eq, PartialEq)]\npub enum IntentCommand {",
    protocol_insert + "#[derive(Clone, Debug, Eq, PartialEq)]\npub enum IntentCommand {",
)
replace_once(
    "crates/linura-protocol/src/lib.rs",
    "        assert_eq!(PlanPreviewFindingLevel::Blocker.as_str(), \"blocker\");",
    "        assert_eq!(PlanPreviewFindingLevel::Blocker.as_str(), \"blocker\");\n        assert_eq!(PlanReviewDecision::RequireApproval.as_str(), \"require-approval\");\n        assert_eq!(PlanReviewApprovalClass::Administrator.as_str(), \"administrator\");",
)
replace_once(
    "crates/linura-sdk/src/lib.rs",
    "    Actor, ActorId, ActorKind, AuthorityClass, Capability, CapabilityId, IntentId, PlanId,\n    ProfileId, ProviderId, RequestId, RequirementId, ResourceId, RiskClass, SemanticReason,",
    "    Actor, ActorId, ActorKind, AuthorityClass, Capability, CapabilityId, IntentId, PlanId,\n    PolicyId, PolicyRevisionId, PrincipalId, ProfileId, ProviderId, RequestId, RequirementId,\n    ResourceId, RiskClass, SemanticReason,",
)
replace_once(
    "crates/linura-sdk/src/lib.rs",
    "    PlanDesiredStateRequest, PlanPreview, PlanPreviewChange, PlanPreviewFinding,\n    PlanPreviewFindingLevel, PlanPreviewStatus, PortableProfileExport, PortableSetupExport,",
    "    PlanDesiredStateRequest, PlanPreview, PlanPreviewChange, PlanPreviewFinding,\n    PlanPreviewFindingLevel, PlanPreviewStatus, PlanReview, PlanReviewApprovalClass,\n    PlanReviewDecision, PortableProfileExport, PortableSetupExport,",
)

# ---- D-Bus wire adapter ----
replace_once(
    "crates/linura-dbus/src/planning.rs",
    "    Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, ProviderId, RequestId,\n    RequirementId, ResourceId, RiskClass, SemanticReason,",
    "    Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, PolicyId, PolicyRevisionId,\n    PrincipalId, ProviderId, RequestId, RequirementId, ResourceId, RiskClass, SemanticReason,",
)
replace_once(
    "crates/linura-dbus/src/planning.rs",
    "    PlanDesiredStateRequest, PlanPreview, PlanPreviewChange, PlanPreviewFinding,\n    PlanPreviewFindingLevel, PlanPreviewStatus,",
    "    PlanDesiredStateRequest, PlanPreview, PlanPreviewChange, PlanPreviewFinding,\n    PlanPreviewFindingLevel, PlanPreviewStatus, PlanReview, PlanReviewApprovalClass,\n    PlanReviewDecision,",
)
review_wire_aliases = r'''
pub(crate) type PlanReviewPolicyWire = (String, String, String, String, String);
pub(crate) type PlanReviewDecisionWire = (String, bool, String, String, bool);
pub(crate) type PlanReviewWire = (
    PlanIdsWire,
    String,
    PlanActorWire,
    PlanRouteWire,
    PlanReasonWire,
    String,
    PlanReviewPolicyWire,
    PlanReviewDecisionWire,
    Vec<PlanChangeWire>,
    Vec<PlanFindingWire>,
);

'''
replace_once(
    "crates/linura-dbus/src/planning.rs",
    ");\n\npub(crate) fn plan_request_wire(request: &PlanDesiredStateRequest) -> PlanRequestWire {",
    ");\n\n" + review_wire_aliases + "pub(crate) fn plan_request_wire(request: &PlanDesiredStateRequest) -> PlanRequestWire {",
)
review_wire_functions = r'''
pub(crate) fn plan_review_wire(review: &PlanReview) -> PlanReviewWire {
    let approval_class = review
        .approval_class
        .map_or_else(String::new, |class| class.as_str().into());
    (
        (
            review.plan_id.as_str().into(),
            review.request_id.as_str().into(),
        ),
        review.principal.as_str().into(),
        (
            review.actor.id.as_str().into(),
            actor_kind_name(review.actor.kind).into(),
            review.actor.interactive,
        ),
        (
            review.provider.as_str().into(),
            review.resource.as_str().into(),
            review.observation_capability.as_str().into(),
        ),
        (
            review.reason.summary.clone(),
            review
                .reason
                .intent_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
            review
                .reason
                .requirement_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
            review
                .reason
                .capability_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
        ),
        review.observed_evidence_id.clone(),
        (
            risk_name(review.planner_risk_floor).into(),
            risk_name(review.reviewed_risk).into(),
            review.status.as_str().into(),
            review.policy_id.as_str().into(),
            review.policy_revision_id.as_str().into(),
        ),
        (
            review.decision.as_str().into(),
            review.approval_class.is_some(),
            approval_class,
            review.decision_reason.clone(),
            review.execution_authorized,
        ),
        review
            .changes
            .iter()
            .map(|change| {
                (
                    change.key.clone(),
                    change.current.is_some(),
                    change.current.clone().unwrap_or_default(),
                    change.desired.clone(),
                )
            })
            .collect(),
        review
            .findings
            .iter()
            .map(|finding| {
                (
                    finding.code.clone(),
                    finding.level.as_str().into(),
                    finding.message.clone(),
                )
            })
            .collect(),
    )
}

pub(crate) fn plan_review_from_wire(wire: PlanReviewWire) -> Result<PlanReview, String> {
    let (
        ids,
        principal,
        actor,
        route,
        reason,
        observed_evidence_id,
        (planner_risk_floor, reviewed_risk, status, policy_id, policy_revision_id),
        (decision, has_approval_class, approval_class, decision_reason, execution_authorized),
        changes,
        findings,
    ) = wire;

    let preview = plan_preview_from_wire((
        ids,
        actor,
        route,
        reason,
        observed_evidence_id,
        planner_risk_floor.clone(),
        status,
        execution_authorized,
        changes,
        findings,
    ))?;
    if decision_reason.len() > 16 * 1024 || decision_reason.chars().any(char::is_control) {
        return Err("plan review decision reason violates transport bounds".into());
    }

    let decision = parse_review_decision(&decision)?;
    let approval_class = if has_approval_class {
        Some(parse_review_approval_class(&approval_class)?)
    } else {
        if !approval_class.is_empty() {
            return Err("plan review supplied an approval class while has_approval_class=false".into());
        }
        None
    };
    match decision {
        PlanReviewDecision::RequireApproval => {
            if approval_class.is_none() || decision_reason.trim().is_empty() {
                return Err("require-approval review lacks class or reason".into());
            }
        }
        PlanReviewDecision::Deny | PlanReviewDecision::Blocked => {
            if approval_class.is_some() || decision_reason.trim().is_empty() {
                return Err("deny/blocked review has inconsistent approval metadata".into());
            }
        }
        PlanReviewDecision::Allow => {
            if approval_class.is_some() {
                return Err("allow review unexpectedly carries an approval class".into());
            }
        }
    }

    let planner_risk_floor = parse_risk(&planner_risk_floor)?;
    let reviewed_risk = parse_risk(&reviewed_risk)?;
    if reviewed_risk < planner_risk_floor && decision != PlanReviewDecision::Blocked {
        return Err("plan review lowered the planner risk floor without blocking".into());
    }

    Ok(PlanReview {
        plan_id: preview.plan_id,
        request_id: preview.request_id,
        principal: PrincipalId::new(principal).map_err(|error| error.to_string())?,
        actor: preview.actor,
        provider: preview.provider,
        resource: preview.resource,
        observation_capability: preview.observation_capability,
        reason: preview.reason,
        observed_evidence_id: preview.observed_evidence_id,
        planner_risk_floor,
        reviewed_risk,
        status: preview.status,
        policy_id: PolicyId::new(policy_id).map_err(|error| error.to_string())?,
        policy_revision_id: PolicyRevisionId::new(policy_revision_id)
            .map_err(|error| error.to_string())?,
        decision,
        approval_class,
        decision_reason,
        execution_authorized: false,
        changes: preview.changes,
        findings: preview.findings,
    })
}

fn parse_review_decision(value: &str) -> Result<PlanReviewDecision, String> {
    match value {
        "allow" => Ok(PlanReviewDecision::Allow),
        "deny" => Ok(PlanReviewDecision::Deny),
        "require-approval" => Ok(PlanReviewDecision::RequireApproval),
        "blocked" => Ok(PlanReviewDecision::Blocked),
        _ => Err(format!("unknown plan review decision {value:?}")),
    }
}

fn parse_review_approval_class(value: &str) -> Result<PlanReviewApprovalClass, String> {
    match value {
        "interactive-user" => Ok(PlanReviewApprovalClass::InteractiveUser),
        "administrator" => Ok(PlanReviewApprovalClass::Administrator),
        "destructive-action" => Ok(PlanReviewApprovalClass::DestructiveAction),
        _ => Err(format!("unknown plan review approval class {value:?}")),
    }
}

'''
replace_once(
    "crates/linura-dbus/src/planning.rs",
    "fn parse_ids<T, E, F>(values: Vec<String>, constructor: F) -> Result<Vec<T>, String>",
    review_wire_functions + "fn parse_ids<T, E, F>(values: Vec<String>, constructor: F) -> Result<Vec<T>, String>",
)

# ---- D-Bus server/client ----
replace_once(
    "crates/linura-dbus/src/lib.rs",
    "    PlanPreview, ProtocolVersion, ProviderSnapshot,",
    "    PlanPreview, PlanReview, ProtocolVersion, ProviderSnapshot,",
)
replace_once(
    "crates/linura-dbus/src/lib.rs",
    "use planning::{PlanPreviewWire, PlanRequestWire};",
    "use planning::{PlanPreviewWire, PlanRequestWire, PlanReviewWire};",
)
server_methods = r'''

    async fn review_plan(
        &self,
        plan_id: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<PlanReviewWire> {
        let caller = authenticated_caller(connection, &header).await?;
        let principal = principal_from_caller(&caller)?;
        let plan_id = PlanId::new(plan_id).map_err(|error| fdo_failed(error.to_string()))?;
        self.with_state(move |state| {
            state
                .review_plan(&principal, &plan_id)
                .map(|review| planning::plan_review_wire(&review))
                .map_err(|error| error.to_string())
        })
        .await
    }

    async fn explain_plan_review(
        &self,
        plan_id: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<PlanReviewWire> {
        let caller = authenticated_caller(connection, &header).await?;
        let principal = principal_from_caller(&caller)?;
        let plan_id = PlanId::new(plan_id).map_err(|error| fdo_failed(error.to_string()))?;
        self.with_state(move |state| {
            state
                .explain_plan_review(&principal, &plan_id)
                .map(|review| planning::plan_review_wire(&review))
                .map_err(|error| error.to_string())
        })
        .await
    }
'''
replace_once(
    "crates/linura-dbus/src/lib.rs",
    "        .await\n    }\n}\n\n/// Adds contract lifecycle annotations to the macro-generated D-Bus introspection",
    "        .await\n    }" + server_methods + "\n}\n\n/// Adds contract lifecycle annotations to the macro-generated D-Bus introspection",
)
client_methods = r'''

    pub fn review_plan(&self, plan_id: &PlanId) -> Result<PlanReview, TransportError> {
        let response: PlanReviewWire = self
            .proxy()?
            .call("ReviewPlan", &(plan_id.as_str(),))
            .map_err(TransportError::from)?;
        planning::plan_review_from_wire(response).map_err(TransportError::new)
    }

    pub fn explain_plan_review(&self, plan_id: &PlanId) -> Result<PlanReview, TransportError> {
        let response: PlanReviewWire = self
            .proxy()?
            .call("ExplainPlanReview", &(plan_id.as_str(),))
            .map_err(TransportError::from)?;
        planning::plan_review_from_wire(response).map_err(TransportError::new)
    }
'''
replace_once(
    "crates/linura-dbus/src/lib.rs",
    "        planning::plan_preview_from_wire(response).map_err(TransportError::new)\n    }\n}\n\n#[derive(Debug)]\npub struct TransportError",
    "        planning::plan_preview_from_wire(response).map_err(TransportError::new)\n    }" + client_methods + "\n}\n\n#[derive(Debug)]\npub struct TransportError",
)
replace_once(
    "crates/linura-dbus/src/lib.rs",
    "        for method in [\"PlanDesiredState\", \"GetPlanPreview\", \"ExplainPlanPreview\"] {",
    "        for method in [\n            \"PlanDesiredState\",\n            \"GetPlanPreview\",\n            \"ExplainPlanPreview\",\n            \"ReviewPlan\",\n            \"ExplainPlanReview\",\n        ] {",
)

# ---- Canonical D-Bus XML ----
xml_methods = r'''    <method name="ReviewPlan">
      <arg name="plan_id" type="s" direction="in"/>
      <arg name="ids" type="(ss)" direction="out"/>
      <arg name="principal" type="s" direction="out"/>
      <arg name="actor" type="(ssb)" direction="out"/>
      <arg name="route" type="(sss)" direction="out"/>
      <arg name="reason" type="(sasasas)" direction="out"/>
      <arg name="observed_evidence_id" type="s" direction="out"/>
      <arg name="review" type="(sssss)" direction="out"/>
      <arg name="decision" type="(sbssb)" direction="out"/>
      <arg name="changes" type="a(sbss)" direction="out"/>
      <arg name="findings" type="a(sss)" direction="out"/>
    </method>
    <method name="ExplainPlanReview">
      <arg name="plan_id" type="s" direction="in"/>
      <arg name="ids" type="(ss)" direction="out"/>
      <arg name="principal" type="s" direction="out"/>
      <arg name="actor" type="(ssb)" direction="out"/>
      <arg name="route" type="(sss)" direction="out"/>
      <arg name="reason" type="(sasasas)" direction="out"/>
      <arg name="observed_evidence_id" type="s" direction="out"/>
      <arg name="review" type="(sssss)" direction="out"/>
      <arg name="decision" type="(sbssb)" direction="out"/>
      <arg name="changes" type="a(sbss)" direction="out"/>
      <arg name="findings" type="a(sss)" direction="out"/>
    </method>
'''
replace_once(
    "interfaces/dbus/org.linura.Control1.xml",
    "  </interface>\n</node>\n",
    xml_methods + "  </interface>\n</node>\n",
)

# ---- CLI ----
replace_once(
    "apps/linuractl/src/main.rs",
    "    PlanPreview, ProtocolVersion, ProviderId, RequestId, RequirementId, ResourceId, RiskClass,\n    SemanticReason,",
    "    PlanPreview, PlanReview, ProtocolVersion, ProviderId, RequestId, RequirementId, ResourceId,\n    RiskClass, SemanticReason,",
)
replace_once(
    "apps/linuractl/src/main.rs",
    "    CommandInfo {\n        name: \"explain-plan-preview\",\n        summary: \"Explain one retained non-executable preview\",\n        offline: false,\n    },",
    "    CommandInfo {\n        name: \"explain-plan-preview\",\n        summary: \"Explain one retained non-executable preview\",\n        offline: false,\n    },\n    CommandInfo {\n        name: \"review-plan\",\n        summary: \"Review one retained canonical plan through trusted policy\",\n        offline: false,\n    },\n    CommandInfo {\n        name: \"explain-plan-review\",\n        summary: \"Explain the trusted review of one retained canonical plan\",\n        offline: false,\n    },",
)
replace_once(
    "apps/linuractl/src/main.rs",
    "        Some(\"help\") | Some(\"--help\") | Some(\"-h\") | None => print_help(),",
    "        Some(\"review-plan\") => {\n            require_arity(&args, 2, \"review-plan <plan-id>\")?;\n            let plan_id = PlanId::new(args[1].clone())?;\n            let review = LocalControlClient::connect()?.review_plan(&plan_id)?;\n            print_plan_review(&review);\n        }\n        Some(\"explain-plan-review\") => {\n            require_arity(&args, 2, \"explain-plan-review <plan-id>\")?;\n            let plan_id = PlanId::new(args[1].clone())?;\n            let review = LocalControlClient::connect()?.explain_plan_review(&plan_id)?;\n            print_plan_review(&review);\n        }\n        Some(\"help\") | Some(\"--help\") | Some(\"-h\") | None => print_help(),",
)
review_printer = r'''
fn print_plan_review(review: &PlanReview) {
    field("plan_id", review.plan_id.as_str());
    field("request_id", review.request_id.as_str());
    field("principal", review.principal.as_str());
    field("actor_id", review.actor.id.as_str());
    field("actor_kind", actor_kind_name(review.actor.kind));
    field("provider", review.provider.as_str());
    field("resource", review.resource.as_str());
    field("capability", review.observation_capability.as_str());
    field("evidence_id", &review.observed_evidence_id);
    field("planner_risk_floor", risk_name(review.planner_risk_floor));
    field("reviewed_risk", risk_name(review.reviewed_risk));
    field("status", review.status.as_str());
    field("policy_id", review.policy_id.as_str());
    field("policy_revision_id", review.policy_revision_id.as_str());
    field("decision", review.decision.as_str());
    field(
        "approval_required",
        if review.approval_class.is_some() { "true" } else { "false" },
    );
    if let Some(class) = review.approval_class {
        field("approval_class", class.as_str());
    }
    if !review.decision_reason.is_empty() {
        field("decision_reason", &review.decision_reason);
    }
    field(
        "execution_authorized",
        if review.execution_authorized { "true" } else { "false" },
    );
    for (index, change) in review.changes.iter().enumerate() {
        field(&format!("change.{index}.key"), &change.key);
        if let Some(current) = &change.current {
            field(&format!("change.{index}.current"), current);
        }
        field(&format!("change.{index}.desired"), &change.desired);
    }
    for (index, finding) in review.findings.iter().enumerate() {
        field(&format!("finding.{index}.code"), &finding.code);
        field(&format!("finding.{index}.level"), finding.level.as_str());
        field(&format!("finding.{index}.message"), &finding.message);
    }
}

'''
replace_once(
    "apps/linuractl/src/main.rs",
    "const fn actor_kind_name(kind: ActorKind) -> &'static str {",
    review_printer + "const fn actor_kind_name(kind: ActorKind) -> &'static str {",
)

# ---- VM acceptance: prove the real review path without forging human approval ----
acceptance = Path("tests/acceptance/008-control1-plan-preview.json")
text = acceptance.read_text(encoding="utf-8")
old = "grep -F \\\"change.0.desired=active\\\" <<<\\\"$first\\\"; retry=$(linuractl plan-preview"
new = "grep -F \\\"change.0.desired=active\\\" <<<\\\"$first\\\"; review=$(linuractl review-plan request:vm-change); grep -F \\\"plan_id=request:vm-change\\\" <<<\\\"$review\\\"; grep -F \\\"principal=unix:uid:$(id -u)\\\" <<<\\\"$review\\\"; grep -F \\\"planner_risk_floor=system-mutation\\\" <<<\\\"$review\\\"; grep -F \\\"reviewed_risk=security-sensitive\\\" <<<\\\"$review\\\"; grep -F \\\"status=change-proposed\\\" <<<\\\"$review\\\"; grep -F \\\"policy_id=policy:baseline\\\" <<<\\\"$review\\\"; grep -F \\\"policy_revision_id=policy:baseline:v1\\\" <<<\\\"$review\\\"; grep -F \\\"decision=require-approval\\\" <<<\\\"$review\\\"; grep -F \\\"approval_required=true\\\" <<<\\\"$review\\\"; grep -F \\\"approval_class=administrator\\\" <<<\\\"$review\\\"; grep -F \\\"execution_authorized=false\\\" <<<\\\"$review\\\"; grep -F \\\"authority-risk-classified\\\" <<<\\\"$review\\\"; review_retry=$(linuractl review-plan request:vm-change); [[ \\\"$review_retry\\\" == \\\"$review\\\" ]]; review_explained=$(linuractl explain-plan-review request:vm-change); [[ \\\"$review_explained\\\" == \\\"$review\\\" ]]; retry=$(linuractl plan-preview"
if text.count(old) != 1:
    raise SystemExit("acceptance: change-review insertion target not found exactly once")
text = text.replace(old, new, 1)
old = "grep -F \\\"=attribute-not-observed\\\" <<<\\\"$blocked\\\"; pre_conflict=$(linuractl explain"
new = "grep -F \\\"=attribute-not-observed\\\" <<<\\\"$blocked\\\"; blocked_review=$(linuractl review-plan request:vm-blocked); grep -F \\\"decision=blocked\\\" <<<\\\"$blocked_review\\\"; grep -F \\\"approval_required=false\\\" <<<\\\"$blocked_review\\\"; grep -F \\\"execution_authorized=false\\\" <<<\\\"$blocked_review\\\"; if grep -F \\\"approval_class=\\\" <<<\\\"$blocked_review\\\" >/dev/null; then echo \\\"blocked review unexpectedly carries approval class\\\" >&2; exit 1; fi; pre_conflict=$(linuractl explain"
if text.count(old) != 1:
    raise SystemExit("acceptance: blocked-review insertion target not found exactly once")
acceptance.write_text(text.replace(old, new, 1), encoding="utf-8")

# ---- Qualification wording: preserve the trust boundary explicitly ----
replace_once(
    "docs/qualification/v0.3.0.md",
    "6. exercise required approval where applicable;",
    "6. exercise the required-approval boundary where applicable: the real daemon/CLI VM path must prove the required approval class, while authenticated approval-evidence issuance/expiry/revocation remains qualified by exact-source deterministic Control tests until a trusted human interaction adapter exists; service D-Bus credentials must never be promoted into human/admin approval authority;",
)

print("v0.3 completion source transformations applied")
