#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};

use linura_core::{CapabilityId, ProviderId, ResourceId};
use linura_graph::{ObservationRecordOutcome, SystemGraph};
use linura_observation::{
    FreshnessState, ObservationEnvelope, ObservationValidationError, ProviderAvailability,
};
use linura_protocol::{
    ObservationExplanation, ObservationRequest, ObservationResponse, ObservationSystemSnapshot,
    ProviderSnapshot,
};
use linura_provider_sdk::{Observer, ProviderError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationControlError {
    DuplicateProvider {
        provider: ProviderId,
    },
    ProviderIdentityMismatch {
        expected: ProviderId,
        actual: ProviderId,
    },
    CapabilityProviderMismatch {
        provider: ProviderId,
        capability: CapabilityId,
    },
    UnknownProvider {
        provider: ProviderId,
    },
    ProviderUnavailable {
        provider: ProviderId,
        reason: Option<String>,
    },
    UnsupportedCapability {
        provider: ProviderId,
        capability: CapabilityId,
    },
    Provider(ProviderError),
    InvalidObservation(ObservationValidationError),
    SupersededEvidence {
        resource: ResourceId,
        evidence_id: String,
    },
    ObservationUnavailable {
        resource: ResourceId,
    },
}

impl Display for ObservationControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateProvider { provider } => {
                write!(f, "observer {} is already registered", provider.as_str())
            }
            Self::ProviderIdentityMismatch { expected, actual } => write!(
                f,
                "observer identity mismatch: expected {}, got {}",
                expected.as_str(),
                actual.as_str()
            ),
            Self::CapabilityProviderMismatch {
                provider,
                capability,
            } => write!(
                f,
                "capability {} is not bound to observer {}",
                capability.as_str(),
                provider.as_str()
            ),
            Self::UnknownProvider { provider } => {
                write!(f, "unknown observation provider {}", provider.as_str())
            }
            Self::ProviderUnavailable { provider, reason } => {
                write!(
                    f,
                    "observation provider {} is unavailable",
                    provider.as_str()
                )?;
                if let Some(reason) = reason {
                    write!(f, ": {reason}")?;
                }
                Ok(())
            }
            Self::UnsupportedCapability {
                provider,
                capability,
            } => write!(
                f,
                "provider {} does not support observation capability {}",
                provider.as_str(),
                capability.as_str()
            ),
            Self::Provider(error) => Display::fmt(error, f),
            Self::InvalidObservation(error) => Display::fmt(error, f),
            Self::SupersededEvidence {
                resource,
                evidence_id,
            } => write!(
                f,
                "observation {evidence_id} for {} is older than the current authoritative state",
                resource.as_str()
            ),
            Self::ObservationUnavailable { resource } => write!(
                f,
                "no authoritative observation is available for {}",
                resource.as_str()
            ),
        }
    }
}

impl std::error::Error for ObservationControlError {}

/// Read-only coordinator for authoritative observations.
///
/// This type deliberately owns no mutation providers or executors. It validates
/// provider identity, freshness and monotonicity before evidence becomes current
/// graph state.
pub struct ObservationCoordinator {
    observers: BTreeMap<ProviderId, Box<dyn Observer>>,
    graph: SystemGraph,
    latest: BTreeMap<ResourceId, ObservationEnvelope>,
}

impl Debug for ObservationCoordinator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservationCoordinator")
            .field("observer_count", &self.observers.len())
            .field("graph_node_count", &self.graph.nodes().count())
            .field("graph_edge_count", &self.graph.edges().len())
            .field("latest_observation_count", &self.latest.len())
            .finish()
    }
}

impl Default for ObservationCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationCoordinator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            observers: BTreeMap::new(),
            graph: SystemGraph::default(),
            latest: BTreeMap::new(),
        }
    }

    pub fn register_observer(
        &mut self,
        observer: Box<dyn Observer>,
    ) -> Result<(), ObservationControlError> {
        let provider = observer.observer_id();
        if self.observers.contains_key(&provider) {
            return Err(ObservationControlError::DuplicateProvider { provider });
        }
        validate_observer_identity(observer.as_ref(), &provider)?;
        self.observers.insert(provider, observer);
        Ok(())
    }

    pub fn provider_snapshot(&self) -> Result<ProviderSnapshot, ObservationControlError> {
        let mut providers = Vec::with_capacity(self.observers.len());
        let mut capabilities = Vec::new();
        for (provider, observer) in &self.observers {
            validate_observer_identity(observer.as_ref(), provider)?;
            providers.push(observer.health());
            capabilities.extend(observer.observation_capabilities());
        }
        providers.sort_by(|left, right| left.provider.as_str().cmp(right.provider.as_str()));
        capabilities.sort_by(|left, right| {
            left.id.as_str().cmp(right.id.as_str()).then_with(|| {
                left.provider
                    .as_ref()
                    .map(ProviderId::as_str)
                    .cmp(&right.provider.as_ref().map(ProviderId::as_str))
            })
        });
        Ok(ProviderSnapshot {
            providers,
            capabilities,
        })
    }

    pub fn observe(
        &mut self,
        request: &ObservationRequest,
        now_unix_ms: u64,
    ) -> Result<ObservationResponse, ObservationControlError> {
        let observer = self.observers.get(&request.provider).ok_or_else(|| {
            ObservationControlError::UnknownProvider {
                provider: request.provider.clone(),
            }
        })?;
        validate_observer_identity(observer.as_ref(), &request.provider)?;

        let health = observer.health();
        if health.availability == ProviderAvailability::Unavailable {
            return Err(ObservationControlError::ProviderUnavailable {
                provider: request.provider.clone(),
                reason: health.reason,
            });
        }
        if !observer.supports_observation(&request.capability) {
            return Err(ObservationControlError::UnsupportedCapability {
                provider: request.provider.clone(),
                capability: request.capability.clone(),
            });
        }

        let observation = observer
            .observe_authoritative(&request.resource, &request.capability)
            .map_err(ObservationControlError::Provider)?;
        observation
            .validate(&request.provider, &request.resource, &request.capability)
            .map_err(ObservationControlError::InvalidObservation)?;
        observation
            .require_current(now_unix_ms)
            .map_err(ObservationControlError::InvalidObservation)?;

        let outcome = self
            .graph
            .record_observation(&observation, FreshnessState::Current);
        if outcome == ObservationRecordOutcome::HistoricalEvidenceRetained {
            return Err(ObservationControlError::SupersededEvidence {
                resource: request.resource.clone(),
                evidence_id: observation.evidence_id(),
            });
        }
        self.latest
            .insert(request.resource.clone(), observation.clone());
        Ok(ObservationResponse {
            observation,
            freshness: FreshnessState::Current,
        })
    }

    pub fn system_snapshot(&self) -> Result<ObservationSystemSnapshot, ObservationControlError> {
        Ok(ObservationSystemSnapshot {
            graph: self.graph.clone(),
            providers: self.provider_snapshot()?,
        })
    }

    pub fn explain(
        &self,
        resource: &ResourceId,
        now_unix_ms: u64,
    ) -> Result<ObservationExplanation, ObservationControlError> {
        let observation = self.latest.get(resource).ok_or_else(|| {
            ObservationControlError::ObservationUnavailable {
                resource: resource.clone(),
            }
        })?;
        Ok(ObservationExplanation {
            resource: observation.resource.clone(),
            provider: observation.provider.clone(),
            capability: observation.capability.clone(),
            freshness: observation.freshness_at(now_unix_ms),
            evidence_id: observation.evidence_id(),
            authority: observation.authority.as_str().into(),
        })
    }

    #[must_use]
    pub fn graph(&self) -> &SystemGraph {
        &self.graph
    }

    #[must_use]
    pub fn latest_observation(&self, resource: &ResourceId) -> Option<&ObservationEnvelope> {
        self.latest.get(resource)
    }
}

fn validate_observer_identity(
    observer: &dyn Observer,
    expected: &ProviderId,
) -> Result<(), ObservationControlError> {
    let health = observer.health();
    if &health.provider != expected {
        return Err(ObservationControlError::ProviderIdentityMismatch {
            expected: expected.clone(),
            actual: health.provider,
        });
    }
    for capability in observer.observation_capabilities() {
        if capability.provider.as_ref() != Some(expected) {
            return Err(ObservationControlError::CapabilityProviderMismatch {
                provider: expected.clone(),
                capability: capability.id,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::Mutex;

    use super::*;
    use linura_core::{Capability, SupportLevel, ValidationError};
    use linura_observation::{ObservationAuthority, ObservedValue, ProviderHealth};

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    struct QueueObserver {
        provider: ProviderId,
        capability: Capability,
        health: ProviderHealth,
        envelopes: Mutex<VecDeque<ObservationEnvelope>>,
    }

    impl QueueObserver {
        fn new(envelopes: Vec<ObservationEnvelope>) -> Self {
            let provider = id(ProviderId::new("systemd"));
            let capability_id = id(CapabilityId::new("systemd.unit.observe"));
            Self {
                capability: Capability {
                    id: capability_id,
                    support: SupportLevel::Supported,
                    provider: Some(provider.clone()),
                    reason: None,
                },
                health: ProviderHealth {
                    provider: provider.clone(),
                    availability: ProviderAvailability::Available,
                    reason: None,
                },
                provider,
                envelopes: Mutex::new(envelopes.into()),
            }
        }
    }

    impl Observer for QueueObserver {
        fn observer_id(&self) -> ProviderId {
            self.provider.clone()
        }

        fn observation_capabilities(&self) -> Vec<Capability> {
            vec![self.capability.clone()]
        }

        fn health(&self) -> ProviderHealth {
            self.health.clone()
        }

        fn resources(&self) -> Result<Vec<ResourceId>, ProviderError> {
            Ok(vec![id(ResourceId::new("systemd:unit:sshd.service"))])
        }

        fn observe_authoritative(
            &self,
            _resource: &ResourceId,
            _capability: &CapabilityId,
        ) -> Result<ObservationEnvelope, ProviderError> {
            let mut envelopes = self
                .envelopes
                .lock()
                .map_err(|_| ProviderError::Internal("observer queue poisoned".into()))?;
            envelopes
                .pop_front()
                .ok_or_else(|| ProviderError::Unavailable("no queued observation".into()))
        }
    }

    fn envelope(observed_at_unix_ms: u64, sequence: u64, state: &str) -> ObservationEnvelope {
        ObservationEnvelope {
            provider: id(ProviderId::new("systemd")),
            resource: id(ResourceId::new("systemd:unit:sshd.service")),
            capability: id(CapabilityId::new("systemd.unit.observe")),
            authority: ObservationAuthority::NativeApi,
            observed_at_unix_ms,
            valid_for_ms: 1_000,
            sequence,
            attributes: BTreeMap::from([(
                "active_state".into(),
                ObservedValue::Text(state.into()),
            )]),
        }
    }

    fn request() -> ObservationRequest {
        ObservationRequest {
            provider: id(ProviderId::new("systemd")),
            resource: id(ResourceId::new("systemd:unit:sshd.service")),
            capability: id(CapabilityId::new("systemd.unit.observe")),
        }
    }

    #[test]
    fn records_only_valid_current_authoritative_state() {
        let mut coordinator = ObservationCoordinator::new();
        let observer = QueueObserver::new(vec![envelope(1_000, 1, "active")]);
        assert_eq!(coordinator.register_observer(Box::new(observer)), Ok(()));
        let response = coordinator
            .observe(&request(), 1_100)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(response.freshness, FreshnessState::Current);
        let explanation = coordinator
            .explain(&request().resource, 1_100)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(explanation.provider.as_str(), "systemd");
        assert_eq!(coordinator.graph().nodes().count(), 4);
    }

    #[test]
    fn late_older_evidence_cannot_replace_current_state() {
        let mut coordinator = ObservationCoordinator::new();
        let observer = QueueObserver::new(vec![
            envelope(1_000, 2, "active"),
            envelope(900, 1, "inactive"),
        ]);
        assert_eq!(coordinator.register_observer(Box::new(observer)), Ok(()));
        let request = request();
        assert!(coordinator.observe(&request, 1_100).is_ok());
        let second = coordinator.observe(&request, 1_100);
        assert!(matches!(
            second,
            Err(ObservationControlError::SupersededEvidence { .. })
        ));
        let latest = coordinator
            .latest_observation(&request.resource)
            .unwrap_or_else(|| unreachable!("current observation must remain"));
        assert_eq!(latest.sequence, 2);
    }

    #[test]
    fn unavailable_provider_fails_closed() {
        let mut coordinator = ObservationCoordinator::new();
        let mut observer = QueueObserver::new(vec![envelope(1_000, 1, "active")]);
        observer.health.availability = ProviderAvailability::Unavailable;
        observer.health.reason = Some("system bus service absent".into());
        assert_eq!(coordinator.register_observer(Box::new(observer)), Ok(()));
        assert!(matches!(
            coordinator.observe(&request(), 1_100),
            Err(ObservationControlError::ProviderUnavailable { .. })
        ));
    }

    #[test]
    fn registration_rejects_unbound_capability_identity() {
        let mut observer = QueueObserver::new(vec![envelope(1_000, 1, "active")]);
        observer.capability.provider = Some(id(ProviderId::new("spoofed")));
        let mut coordinator = ObservationCoordinator::new();
        assert!(matches!(
            coordinator.register_observer(Box::new(observer)),
            Err(ObservationControlError::CapabilityProviderMismatch { .. })
        ));
    }
}
