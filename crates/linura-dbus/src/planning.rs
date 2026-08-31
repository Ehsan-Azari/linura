use std::collections::{BTreeMap, BTreeSet, VecDeque};

use linura_core::{
    Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, ProviderId, RequestId,
    RequirementId, ResourceId, RiskClass, SemanticReason,
};
use linura_observation::FreshnessState;
use linura_observation_control::ObservationCoordinator;
use linura_planner::{
    DesiredResource, DesiredState, DeterministicPlanner, PlanFindingLevel, PlanStatus,
    PlanningFreshness, PlanningObservation, ReconciliationPlan,
};
use linura_protocol::{
    PlanDesiredStateRequest, PlanPreview, PlanPreviewChange, PlanPreviewFinding,
    PlanPreviewFindingLevel, PlanPreviewStatus,
};

pub(crate) const MAX_SUMMARY_BYTES: usize = 4 * 1024;
pub(crate) const MAX_ORIGINS_PER_KIND: usize = 64;
pub(crate) const MAX_TOTAL_ORIGINS: usize = 128;
pub(crate) const MAX_DESIRED_ATTRIBUTES: usize = 128;
pub(crate) const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub(crate) const MAX_PREVIEW_ENTRIES: usize = 256;
pub(crate) const MAX_PREVIEW_ENTRY_BYTES: usize = 128 * 1024;
pub(crate) const MAX_PREVIEW_TOTAL_BYTES: usize = 8 * 1024 * 1024;

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedPlanInput {
    owner_uid: u32,
    request_id: RequestId,
    provider: ProviderId,
    resource: ResourceId,
    observation_capability: CapabilityId,
    reason: SemanticReason,
    desired_state: BTreeMap<String, String>,
}

impl NormalizedPlanInput {
    fn plan_id(&self) -> Result<PlanId, String> {
        PlanId::new(self.request_id.as_str().to_string()).map_err(|error| error.to_string())
    }

    fn desired_resource(&self) -> DesiredResource {
        DesiredResource {
            provider: self.provider.clone(),
            resource: self.resource.clone(),
            observation_capability: self.observation_capability.clone(),
            state: self.desired_state.clone(),
            reason: self.reason.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PreviewKey {
    owner_uid: u32,
    plan_id: PlanId,
}

impl PreviewKey {
    fn from_input(input: &NormalizedPlanInput) -> Result<Self, String> {
        Ok(Self {
            owner_uid: input.owner_uid,
            plan_id: input.plan_id()?,
        })
    }

    fn new(owner_uid: u32, plan_id: PlanId) -> Self {
        Self { owner_uid, plan_id }
    }
}

#[derive(Clone, Debug)]
struct StoredPreview {
    input: NormalizedPlanInput,
    plan: ReconciliationPlan,
    retained_bytes: usize,
}

#[derive(Debug)]
struct PreviewStore {
    capacity: usize,
    max_entry_bytes: usize,
    max_total_bytes: usize,
    retained_bytes: usize,
    entries: BTreeMap<PreviewKey, StoredPreview>,
    order: VecDeque<PreviewKey>,
}

impl Default for PreviewStore {
    fn default() -> Self {
        Self {
            capacity: MAX_PREVIEW_ENTRIES,
            max_entry_bytes: MAX_PREVIEW_ENTRY_BYTES,
            max_total_bytes: MAX_PREVIEW_TOTAL_BYTES,
            retained_bytes: 0,
            entries: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }
}

impl PreviewStore {
    #[cfg(test)]
    fn with_limits(
        capacity: usize,
        max_entry_bytes: usize,
        max_total_bytes: usize,
    ) -> Result<Self, String> {
        if capacity == 0
            || max_entry_bytes == 0
            || max_total_bytes == 0
            || max_entry_bytes > max_total_bytes
        {
            return Err("preview retention limits are invalid".into());
        }
        Ok(Self {
            capacity,
            max_entry_bytes,
            max_total_bytes,
            retained_bytes: 0,
            entries: BTreeMap::new(),
            order: VecDeque::new(),
        })
    }

    fn replay(&self, input: &NormalizedPlanInput) -> Result<Option<ReconciliationPlan>, String> {
        let key = PreviewKey::from_input(input)?;
        let Some(stored) = self.entries.get(&key) else {
            return Ok(None);
        };
        if stored.input != *input {
            return Err(format!(
                "request/plan id {} already identifies a different normalized request for the authenticated principal",
                key.plan_id.as_str(),
            ));
        }
        Ok(Some(stored.plan.clone()))
    }

    fn insert(
        &mut self,
        input: NormalizedPlanInput,
        plan: ReconciliationPlan,
    ) -> Result<(), String> {
        let key = PreviewKey::from_input(&input)?;
        let plan_id = key.plan_id.clone();
        if plan.id != plan_id || plan.request_id != input.request_id {
            return Err("retained preview identity does not match normalized request".into());
        }
        if plan.provider != input.provider
            || plan.resource != input.resource
            || plan.observation_capability != input.observation_capability
            || plan.reason != input.reason
        {
            return Err("retained preview route or semantic reason does not match request".into());
        }

        if let Some(existing) = self.entries.get(&key) {
            if existing.input == input && existing.plan == plan {
                return Ok(());
            }
            return Err(format!(
                "request/plan id {} already identifies a different retained preview",
                plan_id.as_str()
            ));
        }

        let retained_bytes =
            estimate_input_bytes(&input).saturating_add(estimate_plan_bytes(&plan));
        if retained_bytes > self.max_entry_bytes || retained_bytes > self.max_total_bytes {
            return Err(format!(
                "preview requires {retained_bytes} retained bytes, exceeding per-entry limit {}",
                self.max_entry_bytes
            ));
        }

        while self.entries.len() >= self.capacity
            || self.retained_bytes.saturating_add(retained_bytes) > self.max_total_bytes
        {
            let Some(oldest) = self.order.pop_front() else {
                return Err("preview retention accounting is inconsistent".into());
            };
            if let Some(removed) = self.entries.remove(&oldest) {
                self.retained_bytes = self.retained_bytes.saturating_sub(removed.retained_bytes);
            }
        }

        self.retained_bytes = self.retained_bytes.saturating_add(retained_bytes);
        self.order.push_back(key.clone());
        self.entries.insert(
            key,
            StoredPreview {
                input,
                plan,
                retained_bytes,
            },
        );
        Ok(())
    }

    fn get(&self, owner_uid: u32, plan_id: &PlanId) -> Option<ReconciliationPlan> {
        self.entries
            .get(&PreviewKey::new(owner_uid, plan_id.clone()))
            .map(|stored| stored.plan.clone())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

#[derive(Debug)]
pub(crate) struct ControlState {
    pub(crate) coordinator: ObservationCoordinator,
    previews: PreviewStore,
}

impl ControlState {
    pub(crate) fn new(coordinator: ObservationCoordinator) -> Self {
        Self {
            coordinator,
            previews: PreviewStore::default(),
        }
    }

    pub(crate) fn plan_desired_state(
        &mut self,
        actor: Actor,
        owner_uid: u32,
        wire: PlanRequestWire,
    ) -> Result<PlanPreviewWire, String> {
        let input = normalize_plan_input(owner_uid, wire)?;
        if let Some(retained) = self.previews.replay(&input)? {
            return Ok(plan_preview_wire(&preview_from_plan(&retained)));
        }

        let observation_request = linura_protocol::ObservationRequest {
            provider: input.provider.clone(),
            resource: input.resource.clone(),
            capability: input.observation_capability.clone(),
        };
        let response = self
            .coordinator
            .observe(&observation_request)
            .map_err(|error| error.to_string())?;
        let observation = planning_observation(&response);
        let plan = DeterministicPlanner
            .plan_resource(
                input.request_id.clone(),
                actor,
                input.desired_resource(),
                &observation,
            )
            .map_err(|error| error.to_string())?;
        self.previews.insert(input, plan.clone())?;
        Ok(plan_preview_wire(&preview_from_plan(&plan)))
    }

    pub(crate) fn get_plan_preview(
        &self,
        owner_uid: u32,
        plan_id: &str,
    ) -> Result<PlanPreviewWire, String> {
        let plan_id = PlanId::new(plan_id).map_err(|error| error.to_string())?;
        let plan = self.previews.get(owner_uid, &plan_id).ok_or_else(|| {
            "plan preview is not retained for this authenticated user".to_string()
        })?;
        Ok(plan_preview_wire(&preview_from_plan(&plan)))
    }
}

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

fn normalize_plan_input(
    owner_uid: u32,
    wire: PlanRequestWire,
) -> Result<NormalizedPlanInput, String> {
    let (
        (request_id, provider, resource, observation_capability),
        (summary, intent_ids, requirement_ids, capability_ids),
        desired_pairs,
    ) = wire;

    if summary.len() > MAX_SUMMARY_BYTES {
        return Err(format!(
            "semantic summary exceeds {MAX_SUMMARY_BYTES} UTF-8 bytes"
        ));
    }
    if summary.chars().any(char::is_control) {
        return Err("semantic summary contains control characters".into());
    }
    if intent_ids.len() > MAX_ORIGINS_PER_KIND
        || requirement_ids.len() > MAX_ORIGINS_PER_KIND
        || capability_ids.len() > MAX_ORIGINS_PER_KIND
    {
        return Err(format!(
            "semantic origin kind exceeds {MAX_ORIGINS_PER_KIND} entries"
        ));
    }
    let total_origins = intent_ids
        .len()
        .saturating_add(requirement_ids.len())
        .saturating_add(capability_ids.len());
    if total_origins > MAX_TOTAL_ORIGINS {
        return Err(format!(
            "semantic origins exceed {MAX_TOTAL_ORIGINS} total entries"
        ));
    }
    if desired_pairs.is_empty() || desired_pairs.len() > MAX_DESIRED_ATTRIBUTES {
        return Err(format!(
            "desired state must contain between 1 and {MAX_DESIRED_ATTRIBUTES} attributes"
        ));
    }

    let mut request_bytes = request_id
        .len()
        .saturating_add(provider.len())
        .saturating_add(resource.len())
        .saturating_add(observation_capability.len())
        .saturating_add(summary.len());
    for value in intent_ids
        .iter()
        .chain(requirement_ids.iter())
        .chain(capability_ids.iter())
    {
        request_bytes = request_bytes.saturating_add(value.len());
    }
    for (key, value) in &desired_pairs {
        request_bytes = request_bytes
            .saturating_add(key.len())
            .saturating_add(value.len());
    }
    if request_bytes > MAX_REQUEST_BYTES {
        return Err(format!(
            "normalized planning request exceeds {MAX_REQUEST_BYTES} UTF-8 bytes"
        ));
    }

    let reason = SemanticReason {
        summary,
        intent_ids: parse_ids(intent_ids, IntentId::new)?,
        requirement_ids: parse_ids(requirement_ids, RequirementId::new)?,
        capability_ids: parse_ids(capability_ids, CapabilityId::new)?,
    };
    reason.validate().map_err(|error| error.to_string())?;

    let request_id = RequestId::new(request_id).map_err(|error| error.to_string())?;
    let provider = ProviderId::new(provider).map_err(|error| error.to_string())?;
    let resource = ResourceId::new(resource).map_err(|error| error.to_string())?;
    let observation_capability =
        CapabilityId::new(observation_capability).map_err(|error| error.to_string())?;

    let mut desired_state = BTreeMap::new();
    for (key, value) in desired_pairs {
        if desired_state.insert(key.clone(), value).is_some() {
            return Err(format!(
                "desired state contains duplicate attribute {key:?}"
            ));
        }
    }

    let input = NormalizedPlanInput {
        owner_uid,
        request_id,
        provider,
        resource,
        observation_capability,
        reason,
        desired_state,
    };
    DesiredState {
        resources: vec![input.desired_resource()],
    }
    .validate()
    .map_err(|error| error.to_string())?;
    Ok(input)
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

fn planning_observation(response: &linura_protocol::ObservationResponse) -> PlanningObservation {
    PlanningObservation {
        provider: response.observation.provider.clone(),
        resource: response.observation.resource.clone(),
        observation_capability: response.observation.capability.clone(),
        authority: response.observation.authority.as_str().into(),
        evidence_id: response.observation.evidence_id(),
        freshness: match response.freshness {
            FreshnessState::Current => PlanningFreshness::Current,
            FreshnessState::Stale => PlanningFreshness::Stale,
            FreshnessState::Future => PlanningFreshness::Future,
        },
        attributes: response
            .observation
            .attributes
            .iter()
            .map(|(key, value)| (key.clone(), value.to_string()))
            .collect(),
    }
}

fn preview_from_plan(plan: &ReconciliationPlan) -> PlanPreview {
    PlanPreview {
        plan_id: plan.id.clone(),
        request_id: plan.request_id.clone(),
        actor: plan.actor.clone(),
        provider: plan.provider.clone(),
        resource: plan.resource.clone(),
        observation_capability: plan.observation_capability.clone(),
        reason: plan.reason.clone(),
        observed_evidence_id: plan.observed_evidence_id.clone(),
        prospective_risk: plan.prospective_risk,
        status: match plan.status {
            PlanStatus::NoChange => PlanPreviewStatus::NoChange,
            PlanStatus::ChangeProposed => PlanPreviewStatus::ChangeProposed,
            PlanStatus::Blocked => PlanPreviewStatus::Blocked,
        },
        execution_authorized: plan.execution_authorized(),
        changes: plan
            .changes
            .iter()
            .map(|change| PlanPreviewChange {
                key: change.key.clone(),
                current: change.current.clone(),
                desired: change.desired.clone(),
            })
            .collect(),
        findings: plan
            .findings
            .iter()
            .map(|finding| PlanPreviewFinding {
                code: finding.code.clone(),
                level: match finding.level {
                    PlanFindingLevel::Pass => PlanPreviewFindingLevel::Pass,
                    PlanFindingLevel::Warning => PlanPreviewFindingLevel::Warning,
                    PlanFindingLevel::Blocker => PlanPreviewFindingLevel::Blocker,
                },
                message: finding.message.clone(),
            })
            .collect(),
    }
}

fn plan_preview_wire(preview: &PlanPreview) -> PlanPreviewWire {
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

fn estimate_input_bytes(input: &NormalizedPlanInput) -> usize {
    input
        .request_id
        .as_str()
        .len()
        .saturating_add(input.provider.as_str().len())
        .saturating_add(input.resource.as_str().len())
        .saturating_add(input.observation_capability.as_str().len())
        .saturating_add(semantic_reason_bytes(&input.reason))
        .saturating_add(
            input
                .desired_state
                .iter()
                .map(|(key, value)| key.len().saturating_add(value.len()))
                .fold(0usize, usize::saturating_add),
        )
}

fn estimate_plan_bytes(plan: &ReconciliationPlan) -> usize {
    plan.id
        .as_str()
        .len()
        .saturating_add(plan.request_id.as_str().len())
        .saturating_add(plan.actor.id.as_str().len())
        .saturating_add(plan.provider.as_str().len())
        .saturating_add(plan.resource.as_str().len())
        .saturating_add(plan.observation_capability.as_str().len())
        .saturating_add(semantic_reason_bytes(&plan.reason))
        .saturating_add(plan.observed_evidence_id.len())
        .saturating_add(
            plan.changes
                .iter()
                .map(|change| {
                    change
                        .key
                        .len()
                        .saturating_add(change.current.as_deref().map_or(0, str::len))
                        .saturating_add(change.desired.len())
                })
                .fold(0usize, usize::saturating_add),
        )
        .saturating_add(
            plan.findings
                .iter()
                .map(|finding| finding.code.len().saturating_add(finding.message.len()))
                .fold(0usize, usize::saturating_add),
        )
}

fn semantic_reason_bytes(reason: &SemanticReason) -> usize {
    reason
        .summary
        .len()
        .saturating_add(
            reason
                .intent_ids
                .iter()
                .map(|id| id.as_str().len())
                .fold(0usize, usize::saturating_add),
        )
        .saturating_add(
            reason
                .requirement_ids
                .iter()
                .map(|id| id.as_str().len())
                .fold(0usize, usize::saturating_add),
        )
        .saturating_add(
            reason
                .capability_ids
                .iter()
                .map(|id| id.as_str().len())
                .fold(0usize, usize::saturating_add),
        )
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
    use linura_observation::{ObservationAuthority, ObservedValue};
    use linura_protocol::ObservationResponse;

    fn actor(id: &str) -> Actor {
        Actor {
            id: ActorId::new(id).unwrap_or_else(|error| unreachable!("{error}")),
            kind: ActorKind::Service,
            interactive: false,
        }
    }

    fn input(request_id: &str, owner_uid: u32, desired: &str) -> NormalizedPlanInput {
        NormalizedPlanInput {
            owner_uid,
            request_id: RequestId::new(request_id).unwrap_or_else(|error| unreachable!("{error}")),
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
            desired_state: BTreeMap::from([("active_state".into(), desired.into())]),
        }
    }

    fn plan(input: &NormalizedPlanInput, actor: Actor, evidence: &str) -> ReconciliationPlan {
        let response = ObservationResponse {
            observation: linura_observation::ObservationEnvelope {
                provider: input.provider.clone(),
                resource: input.resource.clone(),
                capability: input.observation_capability.clone(),
                authority: ObservationAuthority::SyntheticTest,
                observed_at_unix_ms: 1_000,
                valid_for_ms: 1_000,
                sequence: 1,
                attributes: BTreeMap::from([(
                    "active_state".into(),
                    ObservedValue::Text("inactive".into()),
                )]),
            },
            freshness: FreshnessState::Current,
        };
        let mut observation = planning_observation(&response);
        observation.evidence_id = evidence.into();
        DeterministicPlanner
            .plan_resource(
                input.request_id.clone(),
                actor,
                input.desired_resource(),
                &observation,
            )
            .unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn normalization_rejects_oversized_summary_before_observation() {
        let wire = (
            (
                "request:test".into(),
                "systemd".into(),
                "systemd:unit:test.service".into(),
                "systemd.unit.observe".into(),
            ),
            (
                "x".repeat(MAX_SUMMARY_BYTES + 1),
                vec!["intent:test".into()],
                vec![],
                vec![],
            ),
            vec![("active_state".into(), "active".into())],
        );
        assert!(normalize_plan_input(1000, wire).is_err());
    }

    #[test]
    fn same_unix_principal_can_replay_across_transport_actor_changes() {
        let input = input("request:retry", 1000, "active");
        let first = plan(&input, actor("dbus:first"), "evidence:first");
        let mut store = PreviewStore::default();
        store
            .insert(input.clone(), first.clone())
            .unwrap_or_else(|error| unreachable!("{error}"));
        let replay = store
            .replay(&input)
            .unwrap_or_else(|error| unreachable!("{error}"))
            .unwrap_or_else(|| unreachable!("retained preview missing"));
        assert_eq!(replay, first);
        assert_eq!(replay.actor.id.as_str(), "dbus:first");
    }

    #[test]
    fn same_id_with_different_normalized_input_conflicts() {
        let first_input = input("request:conflict", 1000, "active");
        let second_input = input("request:conflict", 1000, "inactive");
        let first = plan(&first_input, actor("dbus:first"), "evidence:first");
        let mut store = PreviewStore::default();
        store
            .insert(first_input, first)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert!(store.replay(&second_input).is_err());
    }

    #[test]
    fn request_ids_are_namespaced_by_authenticated_unix_principal() {
        let first_input = input("request:private", 1000, "active");
        let other_input = input("request:private", 1001, "inactive");
        let first = plan(&first_input, actor("dbus:first"), "evidence:first");
        let other = plan(&other_input, actor("dbus:other"), "evidence:other");
        let plan_id = first.id.clone();
        let mut store = PreviewStore::default();
        store
            .insert(first_input, first)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert!(
            store
                .replay(&other_input)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .is_none()
        );
        store
            .insert(other_input, other)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(store.len(), 2);
        assert_eq!(
            store
                .get(1000, &plan_id)
                .unwrap_or_else(|| unreachable!("first uid preview missing"))
                .actor
                .id
                .as_str(),
            "dbus:first"
        );
        assert_eq!(
            store
                .get(1001, &plan_id)
                .unwrap_or_else(|| unreachable!("second uid preview missing"))
                .actor
                .id
                .as_str(),
            "dbus:other"
        );
        assert!(store.get(1002, &plan_id).is_none());
    }

    #[test]
    fn retention_is_bounded_by_aggregate_bytes() {
        let first_input = input("request:one", 1000, "active");
        let first = plan(&first_input, actor("dbus:first"), "evidence:first");
        let first_bytes =
            estimate_input_bytes(&first_input).saturating_add(estimate_plan_bytes(&first));

        let second_input = input("request:two", 1000, "active");
        let second = plan(&second_input, actor("dbus:second"), "evidence:second");
        let second_bytes =
            estimate_input_bytes(&second_input).saturating_add(estimate_plan_bytes(&second));
        let aggregate_limit = first_bytes.max(second_bytes);
        let mut store = PreviewStore::with_limits(8, aggregate_limit, aggregate_limit)
            .unwrap_or_else(|error| unreachable!("{error}"));

        store
            .insert(first_input, first)
            .unwrap_or_else(|error| unreachable!("{error}"));
        store
            .insert(second_input, second)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(store.len(), 1);
        assert!(store.retained_bytes() <= aggregate_limit);
    }

    #[test]
    fn wire_decoder_rejects_inconsistent_status_and_execution_authority() {
        let input = input("request:wire", 1000, "active");
        let preview = preview_from_plan(&plan(&input, actor("dbus:first"), "evidence:first"));
        let mut wire = plan_preview_wire(&preview);
        wire.7 = true;
        assert!(plan_preview_from_wire(wire).is_err());

        let mut wire = plan_preview_wire(&preview);
        wire.6 = "no-change".into();
        assert!(plan_preview_from_wire(wire).is_err());
    }

    #[test]
    fn oversized_single_preview_is_rejected() {
        let input = input("request:oversized", 1000, "active");
        let plan = plan(&input, actor("dbus:first"), "evidence:first");
        let mut store =
            PreviewStore::with_limits(8, 1, 8).unwrap_or_else(|error| unreachable!("{error}"));
        assert!(store.insert(input, plan).is_err());
        assert_eq!(store.len(), 0);
    }
}
