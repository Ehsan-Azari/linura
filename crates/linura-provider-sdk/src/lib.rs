#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};

use linura_core::{ActionPlan, Capability, CapabilityId, PlanId, ResourceId};
use linura_protocol::ActionRequest;

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

/// Authoritative observation used as an explicit input to deterministic planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub provider_id: String,
    pub resource: ResourceId,
    pub state: String,
}

/// Evidence that narrow effects were dispatched by an executor.
///
/// This is not proof that the requested machine state exists; verification is
/// intentionally a separate contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionReceipt {
    pub plan_id: PlanId,
    pub executor_id: String,
    pub summary: String,
}

/// Independent postcondition evidence for a completed execution attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReceipt {
    pub plan_id: PlanId,
    pub verifier_id: String,
    pub evidence: String,
}

pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> Vec<Capability>;
    fn observe(&self, resource: &ResourceId) -> Result<Observation, ProviderError>;
    fn plan(
        &self,
        request: &ActionRequest,
        observation: &Observation,
    ) -> Result<ActionPlan, ProviderError>;
    fn supports(&self, capability: &CapabilityId) -> bool {
        self.capabilities()
            .iter()
            .any(|candidate| &candidate.id == capability)
    }
}

/// Narrow privileged or unprivileged effect boundary.
pub trait EffectExecutor: Send + Sync {
    fn id(&self) -> &'static str;
    fn execute(&self, plan: &ActionPlan) -> Result<ExecutionReceipt, ProviderError>;
}

/// Independent verifier boundary.
///
/// A verifier consumes authoritative post-execution observation and must not
/// treat an executor's success return as sufficient proof of postconditions.
pub trait EffectVerifier: Send + Sync {
    fn id(&self) -> &'static str;
    fn verify(
        &self,
        plan: &ActionPlan,
        execution: &ExecutionReceipt,
        post_observation: &Observation,
    ) -> Result<VerificationReceipt, ProviderError>;
}
