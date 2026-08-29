#![forbid(unsafe_code)]

use linura_capability_sdk::{CapabilityCatalog, Resolution};
use linura_core::{CapabilityId, IntentId, ResourceId, SemanticReason};
use linura_intent::Intent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesiredResource {
    pub resource: ResourceId,
    pub state: Vec<(String, String)>,
    pub reason: SemanticReason,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesiredState {
    pub resources: Vec<DesiredResource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentPlan {
    pub intent_id: IntentId,
    pub capability_resolution: Resolution,
    pub desired_state: DesiredState,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanningError {
    RetiredIntent,
    MissingCapability(CapabilityId),
    Conflict(CapabilityId, CapabilityId),
}

#[derive(Clone, Debug, Default)]
pub struct DeterministicPlanner;

impl DeterministicPlanner {
    pub fn resolve_capabilities(
        &self,
        intent: &Intent,
        catalog: &CapabilityCatalog,
        requested: &[CapabilityId],
    ) -> Result<Resolution, PlanningError> {
        if !intent.is_managed() {
            return Err(PlanningError::RetiredIntent);
        }
        let resolution = catalog.resolve(requested);
        if let Some(missing) = resolution.missing.iter().next() {
            return Err(PlanningError::MissingCapability(missing.clone()));
        }
        if let Some((left, right)) = resolution.conflicts.first() {
            return Err(PlanningError::Conflict(left.clone(), right.clone()));
        }
        Ok(resolution)
    }
}
