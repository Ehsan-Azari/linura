#![forbid(unsafe_code)]

use linura_core::{ActionPlan, Capability, CapabilityId, ResourceId};
use linura_protocol::ActionRequest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderError {
    Unsupported(String),
    Unavailable(String),
    InvalidState(String),
    Internal(String),
}

pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> Vec<Capability>;
    fn observe(&self, resource: &ResourceId) -> Result<String, ProviderError>;
    fn plan(&self, request: &ActionRequest) -> Result<ActionPlan, ProviderError>;
    fn supports(&self, capability: &CapabilityId) -> bool {
        self.capabilities().iter().any(|candidate| &candidate.id == capability)
    }
}

pub trait EffectExecutor: Send + Sync {
    fn id(&self) -> &'static str;
    fn execute(&self, plan: &ActionPlan) -> Result<(), ProviderError>;
    fn verify(&self, plan: &ActionPlan) -> Result<(), ProviderError>;
}
