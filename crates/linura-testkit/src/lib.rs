#![forbid(unsafe_code)]

use linura_core::{Capability, CapabilityId, ProviderId, ResourceId};
use linura_observation::{ObservationEnvelope, ProviderHealth};
use linura_provider_sdk::{Observer, ProviderError};

/// Deterministic read-only observer fixture.
///
/// The returned envelope is intentionally supplied by the test so callers can
/// exercise provider/resource/capability substitution and stale-evidence paths.
#[derive(Clone, Debug)]
pub struct FakeObserver {
    pub provider: ProviderId,
    pub capabilities: Vec<Capability>,
    pub health: ProviderHealth,
    pub resources: Vec<ResourceId>,
    pub envelope: ObservationEnvelope,
    pub fail_resources: bool,
    pub fail_observe: bool,
}

impl Observer for FakeObserver {
    fn observer_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn observation_capabilities(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    fn health(&self) -> ProviderHealth {
        self.health.clone()
    }

    fn resources(&self) -> Result<Vec<ResourceId>, ProviderError> {
        if self.fail_resources {
            Err(ProviderError::Unavailable(
                "injected resource discovery failure".into(),
            ))
        } else {
            Ok(self.resources.clone())
        }
    }

    fn observe_authoritative(
        &self,
        _resource: &ResourceId,
        _capability: &CapabilityId,
    ) -> Result<ObservationEnvelope, ProviderError> {
        if self.fail_observe {
            Err(ProviderError::Unavailable(
                "injected authoritative observation failure".into(),
            ))
        } else {
            Ok(self.envelope.clone())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureInjector {
    fail_at_step: Option<usize>,
}

impl FailureInjector {
    #[must_use]
    pub fn never() -> Self {
        Self { fail_at_step: None }
    }

    #[must_use]
    pub fn at(step: usize) -> Self {
        Self {
            fail_at_step: Some(step),
        }
    }

    #[must_use]
    pub fn should_fail(&self, step: usize) -> bool {
        self.fail_at_step == Some(step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_injector_is_deterministic() {
        let injector = FailureInjector::at(2);
        assert!(!injector.should_fail(1));
        assert!(injector.should_fail(2));
    }
}
