use std::collections::{BTreeMap, BTreeSet};

use linura_control::{
    MAX_DESIRED_ATTRIBUTES, MAX_ORIGINS_PER_KIND, MAX_SUMMARY_BYTES, MAX_TOTAL_ORIGINS,
};
use linura_core::{
    Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, ProviderId, RequestId,
    RequirementId, ResourceId, RiskClass, SemanticReason,
};
use linura_protocol::{
    PlanDesiredStateRequest, PlanPreview, PlanPreviewChange, PlanPreviewFinding,
    PlanPreviewFindingLevel, PlanPreviewStatus,
};

pub(crate) type PlanRequestRouteWire = (String, String, String, String);
pub(crate) type PlanReasonWire = (String, Vec<String>, Vec<String>, Vec<String>);
pub(crate) type PlanRequestWire = (PlanRequestRouteWire, PlanReasonWire, Vec<(String, String)>);
pub(crate) type PlanIdsWire = (String, String);
pub(crate) type PlanActorWire = (String, String, bool);
pub(crate) type PlanRouteWire = (String, String, String);
pub(crate) type PlanChangeWire = (String, bool, String, String);
pub(crate) type PlanFindingWire = (String, String, String);
pub(crate) type PlanPreviewWire = (
    PlanIdsWire,
    PlanActorWire,
    PlanRouteWire,
    PlanReasonWire,
    String,
    String,
    String,
    bool,
    Vec<PlanChangeWire>,
    Vec<PlanFindingWire>,
);

pub(crate) fn plan_request_wire(request: &PlanDesiredStateRequest) -> PlanRequestWire {
    (
        (
            request.request_id.as_str().into(),
            request.provider.as_str().into(),
            request.resource.as_str().into(),
            request.observation_capability.as_str().into(),
        ),
        (
            request.reason.summary.clone(),
            request
                .reason
                .intent_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
            request
                .reason
                .requirement_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
            request
                .reason
                .capability_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
        ),
        request
            .desired_state
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

/// Decode the D-Bus representation into the transport-neutral protocol request.
///
/// Resource/retention policy is intentionally not implemented here; those rules
/// are enforced again by `linura-control` before observation. This adapter only
/// rejects malformed wire values (including duplicate map keys that would be lost
/// by conversion into a `BTreeMap`) and applies shared public transport bounds.
pub(crate) fn plan_request_from_wire(
    wire: PlanRequestWire,
) -> Result<PlanDesiredStateRequest, String> {
    let (
        (request_id, provider, resource, observation_capability),
        (summary, intent_ids, requirement_ids, capability_ids),
        desired_pairs,
    ) = wire;

    if summary.len() > MAX_SUMMARY_BYTES || summary.chars().any(char::is_control) {
        return Err("plan request semantic summary violates transport bounds".into());
    }
    if intent_ids.len() > MAX_ORIGINS_PER_KIND
        || requirement_ids.len() > MAX_ORIGINS_PER_KIND
        || capability_ids.len() > MAX_ORIGINS_PER_KIND
        || intent_ids
            .len()
            .saturating_add(requirement_ids.len())
            .saturating_add(capability_ids.len())
            > MAX_TOTAL_ORIGINS
    {
        return Err("plan request semantic origins violate transport bounds".into());
    }
    if desired_pairs.is_empty() || desired_pairs.len() > MAX_DESIRED_ATTRIBUTES {
        return Err("plan request desired-state count violates transport bounds".into());
    }

    let reason = SemanticReason {
        summary,
        intent_ids: parse_ids(intent_ids, IntentId::new)?,
        requirement_ids: parse_ids(requirement_ids, RequirementId::new)?,
        capability_ids: parse_ids(capability_ids, CapabilityId::new)?,
    };
    reason.validate().map_err(|error| error.to_string())?;

    let mut desired_state = BTreeMap::new();
    for (key, value) in desired_pairs {
        if desired_state.insert(key.clone(), value).is_some() {
            return Err(format!(
                "desired state contains duplicate attribute {key:?}"
            ));
        }
    }

    Ok(PlanDesiredStateRequest {
        request_id: RequestId::new(request_id).map_err(|error| error.to_string())?,
        provider: ProviderId::new(provider).map_err(|error| error.to_string())?,
        resource: ResourceId::new(resource).map_err(|error| error.to_string())?,
        observation_capability: CapabilityId::new(observation_capability)
            .map_err(|error| error.to_string())?,
        reason,
        desired_state,
    })
}

pub(crate) fn plan_preview_wire(preview: &PlanPreview) -> PlanPreviewWire {
    (
        (
            preview.plan_id.as_str().into(),
            preview.request_id.as_str().into(),
        ),
        (
            preview.actor.id.as_str().into(),
            actor_kind_name(preview.actor.kind).into(),
            preview.actor.interactive,
        ),
        (
            preview.provider.as_str().into(),
            preview.resource.as_str().into(),
            preview.observation_capability.as_str().into(),
        ),
        (
            preview.reason.summary.clone(),
            preview
                .reason
                .intent_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
            preview
                .reason
                .requirement_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
            preview
                .reason
                .capability_ids
                .iter()
                .map(|id| id.as_str().into())
                .collect(),
        ),
        preview.observed_evidence_id.clone(),
        risk_name(preview.prospective_risk).into(),
        preview.status.as_str().into(),
        preview.execution_authorized,
        preview
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
        preview
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

pub(crate) fn plan_preview_from_wire(wire: PlanPreviewWire) -> Result<PlanPreview, String> {
    let (
        (plan_id, request_id),
        (actor_id, actor_kind, interactive),
        (provider, resource, observation_capability),
        (summary, intent_ids, requirement_ids, capability_ids),
        observed_evidence_id,
        prospective_risk,
        status,
        execution_authorized,
        changes,
        findings,
    ) = wire;
    if execution_authorized {
        return Err("Control1 returned an executable preview, which v0.2.0 forbids".into());
    }
    if summary.len() > MAX_SUMMARY_BYTES || summary.chars().any(char::is_control) {
        return Err("plan preview semantic summary violates transport bounds".into());
    }
    if intent_ids.len() > MAX_ORIGINS_PER_KIND
        || requirement_ids.len() > MAX_ORIGINS_PER_KIND
        || capability_ids.len() > MAX_ORIGINS_PER_KIND
        || intent_ids
            .len()
            .saturating_add(requirement_ids.len())
            .saturating_add(capability_ids.len())
            > MAX_TOTAL_ORIGINS
    {
        return Err("plan preview semantic origins violate transport bounds".into());
    }
    if changes.len() > MAX_DESIRED_ATTRIBUTES {
        return Err("plan preview contains too many state changes".into());
    }
    if findings.is_empty() || findings.len() > MAX_DESIRED_ATTRIBUTES.saturating_add(16) {
        return Err("plan preview finding count violates transport bounds".into());
    }

    let reason = SemanticReason {
        summary,
        intent_ids: parse_ids(intent_ids, IntentId::new)?,
        requirement_ids: parse_ids(requirement_ids, RequirementId::new)?,
        capability_ids: parse_ids(capability_ids, CapabilityId::new)?,
    };
    reason.validate().map_err(|error| error.to_string())?;
    if observed_evidence_id.trim().is_empty() || observed_evidence_id.chars().any(char::is_control)
    {
        return Err("plan preview evidence id is invalid".into());
    }

    let plan_id = PlanId::new(plan_id).map_err(|error| error.to_string())?;
    let request_id = RequestId::new(request_id).map_err(|error| error.to_string())?;
    if plan_id.as_str() != request_id.as_str() {
        return Err("plan preview plan id does not match request id".into());
    }

    let mut seen_change_keys = BTreeSet::new();
    let changes = changes
        .into_iter()
        .map(|(key, has_current, current, desired)| {
            if key.trim().is_empty()
                || key.len() > 256
                || key.chars().any(char::is_control)
                || !seen_change_keys.insert(key.clone())
            {
                return Err(format!(
                    "plan preview contains invalid or duplicate change key {key:?}"
                ));
            }
            if desired.len() > 4_096 || desired.chars().any(char::is_control) {
                return Err(format!(
                    "plan preview contains invalid desired value for {key:?}"
                ));
            }
            if !has_current && !current.is_empty() {
                return Err(format!(
                    "plan preview supplied a current value for {key:?} while has_current=false"
                ));
            }
            Ok(PlanPreviewChange {
                key,
                current: has_current.then_some(current),
                desired,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let findings = findings
        .into_iter()
        .map(|(code, level, message)| {
            if code.trim().is_empty()
                || code.len() > 256
                || code.chars().any(char::is_control)
                || message.trim().is_empty()
                || message.len() > 16 * 1024
                || message.chars().any(char::is_control)
            {
                return Err("plan preview contains an invalid finding".into());
            }
            Ok(PlanPreviewFinding {
                code,
                level: parse_finding_level(&level)?,
                message,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let status = parse_status(&status)?;
    let prospective_risk = parse_risk(&prospective_risk)?;
    let has_blocker = findings
        .iter()
        .any(|finding| finding.level == PlanPreviewFindingLevel::Blocker);
    match status {
        PlanPreviewStatus::NoChange => {
            if !changes.is_empty() || prospective_risk != RiskClass::ReadOnly || has_blocker {
                return Err("no-change preview violates change/risk/blocker invariants".into());
            }
        }
        PlanPreviewStatus::ChangeProposed => {
            if changes.is_empty() || has_blocker {
                return Err("change-proposed preview violates change/blocker invariants".into());
            }
        }
        PlanPreviewStatus::Blocked => {
            if !has_blocker {
                return Err("blocked preview does not contain a blocker finding".into());
            }
        }
    }

    Ok(PlanPreview {
        plan_id,
        request_id,
        actor: Actor {
            id: ActorId::new(actor_id).map_err(|error| error.to_string())?,
            kind: parse_actor_kind(&actor_kind)?,
            interactive,
        },
        provider: ProviderId::new(provider).map_err(|error| error.to_string())?,
        resource: ResourceId::new(resource).map_err(|error| error.to_string())?,
        observation_capability: CapabilityId::new(observation_capability)
            .map_err(|error| error.to_string())?,
        reason,
        observed_evidence_id,
        prospective_risk,
        status,
        execution_authorized: false,
        changes,
        findings,
    })
}

fn parse_ids<T, E, F>(values: Vec<String>, constructor: F) -> Result<Vec<T>, String>
where
    E: std::fmt::Display,
    F: Fn(String) -> Result<T, E>,
{
    values
        .into_iter()
        .map(|value| constructor(value).map_err(|error| error.to_string()))
        .collect()
}

const fn actor_kind_name(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Human => "human",
        ActorKind::Service => "service",
        ActorKind::Agent => "agent",
        ActorKind::Remote => "remote",
    }
}

fn parse_actor_kind(value: &str) -> Result<ActorKind, String> {
    match value {
        "human" => Ok(ActorKind::Human),
        "service" => Ok(ActorKind::Service),
        "agent" => Ok(ActorKind::Agent),
        "remote" => Ok(ActorKind::Remote),
        _ => Err(format!("unknown actor kind {value:?}")),
    }
}

const fn risk_name(risk: RiskClass) -> &'static str {
    match risk {
        RiskClass::ReadOnly => "read-only",
        RiskClass::UserState => "user-state",
        RiskClass::SystemMutation => "system-mutation",
        RiskClass::SecuritySensitive => "security-sensitive",
        RiskClass::Destructive => "destructive",
    }
}

fn parse_risk(value: &str) -> Result<RiskClass, String> {
    match value {
        "read-only" => Ok(RiskClass::ReadOnly),
        "user-state" => Ok(RiskClass::UserState),
        "system-mutation" => Ok(RiskClass::SystemMutation),
        "security-sensitive" => Ok(RiskClass::SecuritySensitive),
        "destructive" => Ok(RiskClass::Destructive),
        _ => Err(format!("unknown plan risk {value:?}")),
    }
}

fn parse_status(value: &str) -> Result<PlanPreviewStatus, String> {
    match value {
        "no-change" => Ok(PlanPreviewStatus::NoChange),
        "change-proposed" => Ok(PlanPreviewStatus::ChangeProposed),
        "blocked" => Ok(PlanPreviewStatus::Blocked),
        _ => Err(format!("unknown plan preview status {value:?}")),
    }
}

fn parse_finding_level(value: &str) -> Result<PlanPreviewFindingLevel, String> {
    match value {
        "pass" => Ok(PlanPreviewFindingLevel::Pass),
        "warning" => Ok(PlanPreviewFindingLevel::Warning),
        "blocker" => Ok(PlanPreviewFindingLevel::Blocker),
        _ => Err(format!("unknown plan finding level {value:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PlanDesiredStateRequest {
        PlanDesiredStateRequest {
            request_id: RequestId::new("request:test")
                .unwrap_or_else(|error| unreachable!("{error}")),
            provider: ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}")),
            resource: ResourceId::new("systemd:unit:test.service")
                .unwrap_or_else(|error| unreachable!("{error}")),
            observation_capability: CapabilityId::new("systemd.unit.observe")
                .unwrap_or_else(|error| unreachable!("{error}")),
            reason: SemanticReason {
                summary: "keep test active".into(),
                intent_ids: vec![
                    IntentId::new("intent:test").unwrap_or_else(|error| unreachable!("{error}")),
                ],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            desired_state: BTreeMap::from([("active_state".into(), "active".into())]),
        }
    }

    fn preview() -> PlanPreview {
        PlanPreview {
            plan_id: PlanId::new("request:test").unwrap_or_else(|error| unreachable!("{error}")),
            request_id: RequestId::new("request:test")
                .unwrap_or_else(|error| unreachable!("{error}")),
            actor: Actor {
                id: ActorId::new("dbus:test").unwrap_or_else(|error| unreachable!("{error}")),
                kind: ActorKind::Service,
                interactive: false,
            },
            provider: ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}")),
            resource: ResourceId::new("systemd:unit:test.service")
                .unwrap_or_else(|error| unreachable!("{error}")),
            observation_capability: CapabilityId::new("systemd.unit.observe")
                .unwrap_or_else(|error| unreachable!("{error}")),
            reason: request().reason,
            observed_evidence_id: "evidence:test".into(),
            prospective_risk: RiskClass::SystemMutation,
            status: PlanPreviewStatus::ChangeProposed,
            execution_authorized: false,
            changes: vec![PlanPreviewChange {
                key: "active_state".into(),
                current: Some("inactive".into()),
                desired: "active".into(),
            }],
            findings: vec![PlanPreviewFinding {
                code: "plan.valid".into(),
                level: PlanPreviewFindingLevel::Pass,
                message: "preview is valid".into(),
            }],
        }
    }

    #[test]
    fn typed_request_round_trips_through_wire() {
        let request = request();
        let decoded = plan_request_from_wire(plan_request_wire(&request))
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(decoded, request);
    }

    #[test]
    fn request_decoder_rejects_duplicate_desired_state_keys() {
        let mut wire = plan_request_wire(&request());
        wire.2.push(("active_state".into(), "inactive".into()));
        assert!(plan_request_from_wire(wire).is_err());
    }

    #[test]
    fn wire_decoder_rejects_inconsistent_status_and_execution_authority() {
        let preview = preview();
        let mut wire = plan_preview_wire(&preview);
        wire.7 = true;
        assert!(plan_preview_from_wire(wire).is_err());

        let mut wire = plan_preview_wire(&preview);
        wire.6 = "no-change".into();
        assert!(plan_preview_from_wire(wire).is_err());
    }
}
