#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};

use linura_core::{Capability, CapabilityId, ProviderId, ResourceId};
use linura_observation::{ObservationEnvelope, ProviderHealth};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderError {
    Unsupported(String),
    Unavailable(String),
    InvalidState(String),
    Internal(String),
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (kind, detail) = match self {
            Self::Unsupported(detail) => ("unsupported", detail),
            Self::Unavailable(detail) => ("unavailable", detail),
            Self::InvalidState(detail) => ("invalid state", detail),
            Self::Internal(detail) => ("internal error", detail),
        };
        write!(f, "provider {kind}: {detail}")
    }
}

impl std::error::Error for ProviderError {}

/// Read-only authoritative observation provider.
///
/// Implementing `Observer` grants no planning, policy, approval, execution or
/// verification authority. One call is a bounded provider-backed probe that
/// returns Linura's canonical authoritative [`ObservationEnvelope`].
///
/// Cross-provider fan-out, global retry policy, deadlines, cancellation, query
/// coalescing, cache policy, backpressure and aggregate resource budgets belong
/// to Linura's control-plane orchestration rather than to an individual observer.
/// Transport-specific handles and values must not escape through this trait.
pub trait Observer: Send + Sync {
    fn observer_id(&self) -> ProviderId;
    fn observation_capabilities(&self) -> Vec<Capability>;
    fn health(&self) -> ProviderHealth;
    fn resources(&self) -> Result<Vec<ResourceId>, ProviderError>;
    fn observe_authoritative(
        &self,
        resource: &ResourceId,
        capability: &CapabilityId,
    ) -> Result<ObservationEnvelope, ProviderError>;

    fn supports_observation(&self, capability: &CapabilityId) -> bool {
        self.observation_capabilities()
            .iter()
            .any(|candidate| &candidate.id == capability)
    }
}

/// Privileged executor and independent verifier contracts are intentionally not
/// speculated here before their roadmap milestones. The narrow executor package
/// scaffolds remain in-tree, but v0.4 must first establish the durable prepared
/// transaction/binding model and v0.5 must qualify executor/verifier interfaces
/// against that exact model before they become provider-SDK authority contracts.
