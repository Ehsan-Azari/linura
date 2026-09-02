use std::collections::{BTreeMap, VecDeque};
use std::fmt::{Display, Formatter};

use linura_core::{Actor, PlanId};
use linura_graph::SystemGraph;
use linura_observation::FreshnessState;
use linura_observation_control::{ObservationControlError, ObservationCoordinator};
use linura_planner::{
    DesiredResource, DesiredState, DeterministicPlanner, PlanFindingLevel, PlanStatus,
    PlanningFreshness, PlanningObservation, ReconciliationPlan,
};
use linura_protocol::{
    ObservationExplanation, ObservationRequest, ObservationResponse, PlanDesiredStateRequest,
    PlanPreview, PlanPreviewChange, PlanPreviewFinding, PlanPreviewFindingLevel, PlanPreviewStatus,
    ProviderSnapshot,
};

pub const MAX_SUMMARY_BYTES: usize = 4 * 1024;
pub const MAX_ORIGINS_PER_KIND: usize = 64;
pub const MAX_TOTAL_ORIGINS: usize = 128;
pub const MAX_DESIRED_ATTRIBUTES: usize = 128;
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_PREVIEW_ENTRIES: usize = 256;
pub const MAX_PREVIEW_ENTRY_BYTES: usize = 128 * 1024;
pub const MAX_PREVIEW_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_PRINCIPAL_BYTES: usize = 256;

/// Stable authenticated identity used to namespace process-local control state.
///
/// Transports derive this value from authenticated credentials. It is deliberately
/// separate from [`Actor`]: actor identity records the concrete request provenance,
/// while the principal remains stable across transport reconnects for replay and
/// retained-preview authorization.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthenticatedPrincipal(String);

impl AuthenticatedPrincipal {
    pub fn new(value: impl Into<String>) -> Result<Self, PlanPreviewControlError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(PlanPreviewControlError::InvalidPrincipal {
                reason: "authenticated principal cannot be empty".into(),
            });
        }
        if value.len() > MAX_PRINCIPAL_BYTES {
            return Err(PlanPreviewControlError::InvalidPrincipal {
                reason: format!(
                    "authenticated principal exceeds {MAX_PRINCIPAL_BYTES} UTF-8 bytes"
                ),
            });
        }
        if value.chars().any(char::is_control) {
            return Err(PlanPreviewControlError::InvalidPrincipal {
                reason: "authenticated principal contains control characters".into(),
            });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanPreviewControlError {
    InvalidPrincipal { reason: String },
    InvalidRequest { reason: String },
    IdempotencyConflict { plan_id: PlanId },
    Observation { reason: String },
    Planning { reason: String },
    Review { reason: String },
    Retention { reason: String },
    NotRetained { plan_id: PlanId },
}

impl Display for PlanPreviewControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrincipal { reason } => {
                write!(f, "invalid authenticated principal: {reason}")
            }
            Self::InvalidRequest { reason } => write!(f, "invalid plan preview request: {reason}"),
            Self::IdempotencyConflict { plan_id } => write!(
                f,
                "request/plan id {} already identifies a different normalized request for the authenticated principal",
                plan_id.as_str()
            ),
            Self::Observation { reason } => write!(f, "authoritative observation failed: {reason}"),
            Self::Planning { reason } => write!(f, "deterministic planning failed: {reason}"),
            Self::Review { reason } => write!(f, "trusted plan review failed: {reason}"),
            Self::Retention { reason } => write!(f, "plan preview retention failed: {reason}"),
            Self::NotRetained { plan_id } => write!(
                f,
                "plan preview {} is not retained for this authenticated principal",
                plan_id.as_str()
            ),
        }
    }
}

impl std::error::Error for PlanPreviewControlError {}

impl From<ObservationControlError> for PlanPreviewControlError {
    fn from(error: ObservationControlError) -> Self {
        Self::Observation {
            reason: error.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedPlanInput {
    principal: AuthenticatedPrincipal,
    request: PlanDesiredStateRequest,
}

impl NormalizedPlanInput {
    fn new(
        principal: AuthenticatedPrincipal,
        request: PlanDesiredStateRequest,
    ) -> Result<Self, PlanPreviewControlError> {
        validate_request(&request)?;
        Ok(Self { principal, request })
    }

    fn plan_id(&self) -> Result<PlanId, PlanPreviewControlError> {
        PlanId::new(self.request.request_id.as_str().to_string()).map_err(|error| {
            PlanPreviewControlError::InvalidRequest {
                reason: error.to_string(),
            }
        })
    }

    fn desired_resource(&self) -> DesiredResource {
        DesiredResource {
            provider: self.request.provider.clone(),
            resource: self.request.resource.clone(),
            observation_capability: self.request.observation_capability.clone(),
            state: self.request.desired_state.clone(),
            reason: self.request.reason.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PreviewKey {
    principal: AuthenticatedPrincipal,
    plan_id: PlanId,
}

impl PreviewKey {
    fn from_input(input: &NormalizedPlanInput) -> Result<Self, PlanPreviewControlError> {
        Ok(Self {
            principal: input.principal.clone(),
            plan_id: input.plan_id()?,
        })
    }

    fn new(principal: AuthenticatedPrincipal, plan_id: PlanId) -> Self {
        Self { principal, plan_id }
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
    ) -> Result<Self, PlanPreviewControlError> {
        if capacity == 0
            || max_entry_bytes == 0
            || max_total_bytes == 0
            || max_entry_bytes > max_total_bytes
        {
            return Err(PlanPreviewControlError::Retention {
                reason: "preview retention limits are invalid".into(),
            });
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

    fn replay(
        &self,
        input: &NormalizedPlanInput,
    ) -> Result<Option<ReconciliationPlan>, PlanPreviewControlError> {
        let key = PreviewKey::from_input(input)?;
        let Some(stored) = self.entries.get(&key) else {
            return Ok(None);
        };
        if stored.input != *input {
            return Err(PlanPreviewControlError::IdempotencyConflict {
                plan_id: key.plan_id,
            });
        }
        Ok(Some(stored.plan.clone()))
    }

    fn insert(
        &mut self,
        input: NormalizedPlanInput,
        plan: ReconciliationPlan,
    ) -> Result<(), PlanPreviewControlError> {
        let key = PreviewKey::from_input(&input)?;
        let plan_id = key.plan_id.clone();
        if plan.id != plan_id || plan.request_id != input.request.request_id {
            return Err(PlanPreviewControlError::Retention {
                reason: "retained preview identity does not match normalized request".into(),
            });
        }
        if plan.provider != input.request.provider
            || plan.resource != input.request.resource
            || plan.observation_capability != input.request.observation_capability
            || plan.reason != input.request.reason
        {
            return Err(PlanPreviewControlError::Retention {
                reason: "retained preview route or semantic reason does not match request".into(),
            });
        }

        if let Some(existing) = self.entries.get(&key) {
            if existing.input == input && existing.plan == plan {
                return Ok(());
            }
            return Err(PlanPreviewControlError::IdempotencyConflict { plan_id });
        }

        let retained_bytes = estimate_key_bytes(&key)
            .saturating_add(estimate_input_bytes(&input))
            .saturating_add(estimate_plan_bytes(&plan));
        if retained_bytes > self.max_entry_bytes || retained_bytes > self.max_total_bytes {
            return Err(PlanPreviewControlError::Retention {
                reason: format!(
                    "preview requires {retained_bytes} retained bytes, exceeding per-entry limit {}",
                    self.max_entry_bytes
                ),
            });
        }

        while self.entries.len() >= self.capacity
            || self.retained_bytes.saturating_add(retained_bytes) > self.max_total_bytes
        {
            let Some(oldest) = self.order.pop_front() else {
                return Err(PlanPreviewControlError::Retention {
                    reason: "preview retention accounting is inconsistent".into(),
                });
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

    fn get(
        &self,
        principal: &AuthenticatedPrincipal,
        plan_id: &PlanId,
    ) -> Option<ReconciliationPlan> {
        self.entries
            .get(&PreviewKey::new(principal.clone(), plan_id.clone()))
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

/// Transport-neutral local read/planning control plane.
///
/// This type owns authoritative observation → deterministic planning → bounded
/// preview retention sequencing. Transports authenticate callers and adapt wire
/// data, but they do not own replay, evidence, planning, or retention semantics.
#[derive(Debug)]
pub struct PlanPreviewControl {
    coordinator: ObservationCoordinator,
    previews: PreviewStore,
}

impl PlanPreviewControl {
    #[must_use]
    pub fn new(coordinator: ObservationCoordinator) -> Self {
        Self {
            coordinator,
            previews: PreviewStore::default(),
        }
    }

    pub fn provider_snapshot(&self) -> Result<ProviderSnapshot, ObservationControlError> {
        self.coordinator.provider_snapshot()
    }

    pub fn observe(
        &mut self,
        request: &ObservationRequest,
    ) -> Result<ObservationResponse, ObservationControlError> {
        self.coordinator.observe(request)
    }

    pub fn graph_snapshot(&self) -> Result<SystemGraph, ObservationControlError> {
        self.coordinator.graph_snapshot()
    }

    pub fn explain_observation(
        &self,
        resource: &linura_core::ResourceId,
    ) -> Result<ObservationExplanation, ObservationControlError> {
        self.coordinator.explain_current(resource)
    }

    pub fn plan_desired_state(
        &mut self,
        principal: AuthenticatedPrincipal,
        actor: Actor,
        request: PlanDesiredStateRequest,
    ) -> Result<PlanPreview, PlanPreviewControlError> {
        let input = NormalizedPlanInput::new(principal, request)?;
        if let Some(retained) = self.previews.replay(&input)? {
            return Ok(preview_from_plan(&retained));
        }

        let observation_request = ObservationRequest {
            provider: input.request.provider.clone(),
            resource: input.request.resource.clone(),
            capability: input.request.observation_capability.clone(),
        };
        let response = self.coordinator.observe(&observation_request)?;
        let observation = planning_observation(&response);
        let plan = DeterministicPlanner
            .plan_resource(
                input.request.request_id.clone(),
                actor,
                input.desired_resource(),
                &observation,
            )
            .map_err(|error| PlanPreviewControlError::Planning {
                reason: error.to_string(),
            })?;
        self.previews.insert(input, plan.clone())?;
        Ok(preview_from_plan(&plan))
    }

    pub(crate) fn retained_plan(
        &self,
        principal: &AuthenticatedPrincipal,
        plan_id: &PlanId,
    ) -> Result<ReconciliationPlan, PlanPreviewControlError> {
        self.previews
            .get(principal, plan_id)
            .ok_or_else(|| PlanPreviewControlError::NotRetained {
                plan_id: plan_id.clone(),
            })
    }

    pub fn get_plan_preview(
        &self,
        principal: &AuthenticatedPrincipal,
        plan_id: &PlanId,
    ) -> Result<PlanPreview, PlanPreviewControlError> {
        let plan = self.previews.get(principal, plan_id).ok_or_else(|| {
            PlanPreviewControlError::NotRetained {
                plan_id: plan_id.clone(),
            }
        })?;
        Ok(preview_from_plan(&plan))
    }

    pub fn explain_plan_preview(
        &self,
        principal: &AuthenticatedPrincipal,
        plan_id: &PlanId,
    ) -> Result<PlanPreview, PlanPreviewControlError> {
        self.get_plan_preview(principal, plan_id)
    }
}

fn validate_request(request: &PlanDesiredStateRequest) -> Result<(), PlanPreviewControlError> {
    if request.reason.summary.len() > MAX_SUMMARY_BYTES {
        return invalid_request(format!(
            "semantic summary exceeds {MAX_SUMMARY_BYTES} UTF-8 bytes"
        ));
    }
    if request.reason.summary.chars().any(char::is_control) {
        return invalid_request("semantic summary contains control characters");
    }
    if request.reason.intent_ids.len() > MAX_ORIGINS_PER_KIND
        || request.reason.requirement_ids.len() > MAX_ORIGINS_PER_KIND
        || request.reason.capability_ids.len() > MAX_ORIGINS_PER_KIND
    {
        return invalid_request(format!(
            "semantic origin kind exceeds {MAX_ORIGINS_PER_KIND} entries"
        ));
    }
    let total_origins = request
        .reason
        .intent_ids
        .len()
        .saturating_add(request.reason.requirement_ids.len())
        .saturating_add(request.reason.capability_ids.len());
    if total_origins > MAX_TOTAL_ORIGINS {
        return invalid_request(format!(
            "semantic origins exceed {MAX_TOTAL_ORIGINS} total entries"
        ));
    }
    if request.desired_state.is_empty() || request.desired_state.len() > MAX_DESIRED_ATTRIBUTES {
        return invalid_request(format!(
            "desired state must contain between 1 and {MAX_DESIRED_ATTRIBUTES} attributes"
        ));
    }

    let mut request_bytes = request
        .request_id
        .as_str()
        .len()
        .saturating_add(request.provider.as_str().len())
        .saturating_add(request.resource.as_str().len())
        .saturating_add(request.observation_capability.as_str().len())
        .saturating_add(request.reason.summary.len());
    for id in request
        .reason
        .intent_ids
        .iter()
        .map(|id| id.as_str())
        .chain(request.reason.requirement_ids.iter().map(|id| id.as_str()))
        .chain(request.reason.capability_ids.iter().map(|id| id.as_str()))
    {
        request_bytes = request_bytes.saturating_add(id.len());
    }
    for (key, value) in &request.desired_state {
        request_bytes = request_bytes
            .saturating_add(key.len())
            .saturating_add(value.len());
    }
    if request_bytes > MAX_REQUEST_BYTES {
        return invalid_request(format!(
            "normalized planning request exceeds {MAX_REQUEST_BYTES} UTF-8 bytes"
        ));
    }

    request
        .reason
        .validate()
        .map_err(|error| PlanPreviewControlError::InvalidRequest {
            reason: error.to_string(),
        })?;
    DesiredState {
        resources: vec![DesiredResource {
            provider: request.provider.clone(),
            resource: request.resource.clone(),
            observation_capability: request.observation_capability.clone(),
            state: request.desired_state.clone(),
            reason: request.reason.clone(),
        }],
    }
    .validate()
    .map_err(|error| PlanPreviewControlError::InvalidRequest {
        reason: error.to_string(),
    })?;
    Ok(())
}

fn invalid_request<T>(reason: impl Into<String>) -> Result<T, PlanPreviewControlError> {
    Err(PlanPreviewControlError::InvalidRequest {
        reason: reason.into(),
    })
}

fn planning_observation(response: &ObservationResponse) -> PlanningObservation {
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

fn estimate_key_bytes(key: &PreviewKey) -> usize {
    key.principal
        .as_str()
        .len()
        .saturating_add(key.plan_id.as_str().len())
}

fn estimate_input_bytes(input: &NormalizedPlanInput) -> usize {
    input
        .principal
        .as_str()
        .len()
        .saturating_add(input.request.request_id.as_str().len())
        .saturating_add(input.request.provider.as_str().len())
        .saturating_add(input.request.resource.as_str().len())
        .saturating_add(input.request.observation_capability.as_str().len())
        .saturating_add(semantic_reason_bytes(&input.request.reason))
        .saturating_add(
            input
                .request
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

fn semantic_reason_bytes(reason: &linura_core::SemanticReason) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;
    use linura_core::{
        ActorId, ActorKind, CapabilityId, IntentId, ProviderId, RequestId, ResourceId, RiskClass,
        SemanticReason,
    };
    use linura_observation::{ObservationAuthority, ObservedValue};

    fn actor(id: &str) -> Actor {
        Actor {
            id: ActorId::new(id).unwrap_or_else(|error| unreachable!("{error}")),
            kind: ActorKind::Service,
            interactive: false,
        }
    }

    fn principal(id: &str) -> AuthenticatedPrincipal {
        AuthenticatedPrincipal::new(id).unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn request(request_id: &str, desired: &str) -> PlanDesiredStateRequest {
        PlanDesiredStateRequest {
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

    fn input(
        principal: AuthenticatedPrincipal,
        request_id: &str,
        desired: &str,
    ) -> NormalizedPlanInput {
        NormalizedPlanInput::new(principal, request(request_id, desired))
            .unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn plan(input: &NormalizedPlanInput, actor: Actor, evidence: &str) -> ReconciliationPlan {
        let response = ObservationResponse {
            observation: linura_observation::ObservationEnvelope {
                provider: input.request.provider.clone(),
                resource: input.request.resource.clone(),
                capability: input.request.observation_capability.clone(),
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
                input.request.request_id.clone(),
                actor,
                input.desired_resource(),
                &observation,
            )
            .unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn principal_rejects_ambiguous_or_unbounded_values() {
        assert!(AuthenticatedPrincipal::new("").is_err());
        assert!(AuthenticatedPrincipal::new("unix:uid:1000\nspoof").is_err());
        assert!(AuthenticatedPrincipal::new("x".repeat(MAX_PRINCIPAL_BYTES + 1)).is_err());
    }

    #[test]
    fn normalization_rejects_oversized_summary_before_observation() {
        let mut request = request("request:test", "active");
        request.reason.summary = "x".repeat(MAX_SUMMARY_BYTES + 1);
        assert!(NormalizedPlanInput::new(principal("unix:uid:1000"), request).is_err());
    }

    #[test]
    fn stable_principal_can_replay_across_transport_actor_changes() {
        let input = input(principal("unix:uid:1000"), "request:retry", "active");
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
        let owner = principal("unix:uid:1000");
        let first_input = input(owner.clone(), "request:conflict", "active");
        let second_input = input(owner, "request:conflict", "inactive");
        let first = plan(&first_input, actor("dbus:first"), "evidence:first");
        let mut store = PreviewStore::default();
        store
            .insert(first_input, first)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert!(matches!(
            store.replay(&second_input),
            Err(PlanPreviewControlError::IdempotencyConflict { .. })
        ));
    }

    #[test]
    fn request_ids_are_namespaced_by_authenticated_principal() {
        let first_input = input(principal("unix:uid:1000"), "request:private", "active");
        let other_input = input(principal("unix:uid:1001"), "request:private", "inactive");
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
                .get(&principal("unix:uid:1000"), &plan_id)
                .unwrap_or_else(|| unreachable!("first principal preview missing"))
                .actor
                .id
                .as_str(),
            "dbus:first"
        );
        assert_eq!(
            store
                .get(&principal("unix:uid:1001"), &plan_id)
                .unwrap_or_else(|| unreachable!("second principal preview missing"))
                .actor
                .id
                .as_str(),
            "dbus:other"
        );
        assert!(store.get(&principal("unix:uid:1002"), &plan_id).is_none());
    }

    #[test]
    fn retention_is_bounded_by_aggregate_bytes() {
        let owner = principal("unix:uid:1000");
        let first_input = input(owner.clone(), "request:one", "active");
        let first = plan(&first_input, actor("dbus:first"), "evidence:first");
        let first_bytes = estimate_key_bytes(
            &PreviewKey::from_input(&first_input).unwrap_or_else(|error| unreachable!("{error}")),
        )
        .saturating_add(estimate_input_bytes(&first_input))
        .saturating_add(estimate_plan_bytes(&first));

        let second_input = input(owner, "request:two", "active");
        let second = plan(&second_input, actor("dbus:second"), "evidence:second");
        let second_bytes = estimate_key_bytes(
            &PreviewKey::from_input(&second_input).unwrap_or_else(|error| unreachable!("{error}")),
        )
        .saturating_add(estimate_input_bytes(&second_input))
        .saturating_add(estimate_plan_bytes(&second));
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
    fn oversized_single_preview_is_rejected() {
        let input = input(principal("unix:uid:1000"), "request:oversized", "active");
        let plan = plan(&input, actor("dbus:first"), "evidence:first");
        let mut store =
            PreviewStore::with_limits(8, 1, 8).unwrap_or_else(|error| unreachable!("{error}"));
        assert!(store.insert(input, plan).is_err());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn risk_projection_remains_non_executable() {
        let input = input(principal("unix:uid:1000"), "request:preview", "active");
        let preview = preview_from_plan(&plan(&input, actor("dbus:first"), "evidence:first"));
        assert_eq!(preview.prospective_risk, RiskClass::SystemMutation);
        assert!(!preview.execution_authorized);
    }
}
