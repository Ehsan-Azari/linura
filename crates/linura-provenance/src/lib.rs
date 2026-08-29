#![forbid(unsafe_code)]

use linura_core::{Actor, CapabilityId, IntentId, PlanId, RequirementId, ResourceId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceKind {
    UserIntent,
    AgentInterpretation,
    PlannerDerivation,
    PolicyDecision,
    Execution,
    Verification,
    Reconciliation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvenanceRecord {
    pub id: String,
    pub kind: ProvenanceKind,
    pub actor: Actor,
    pub summary: String,
    pub intent_ids: Vec<IntentId>,
    pub requirement_ids: Vec<RequirementId>,
    pub capability_ids: Vec<CapabilityId>,
    pub resource_ids: Vec<ResourceId>,
    pub plan_id: Option<PlanId>,
    pub parent_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WhyChain {
    pub records: Vec<ProvenanceRecord>,
}

impl WhyChain {
    pub fn summary(&self) -> Vec<&str> {
        self.records
            .iter()
            .map(|record| record.summary.as_str())
            .collect()
    }
}
