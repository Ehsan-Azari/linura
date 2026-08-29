#![forbid(unsafe_code)]

use linura_core::{ActionPlan, Actor, Capability, CapabilityId, IntentId, RequestId, ResourceId};
use linura_graph::{RemovalImpact, SystemGraph};
use linura_intent::{Intent, IntentProposal, MachineProfile};
use linura_provenance::WhyChain;

pub const PROTOCOL_MAJOR: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub product_version: &'static str,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self { major: PROTOCOL_MAJOR, product_version: env!("CARGO_PKG_VERSION") }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilitySnapshot {
    pub profile_id: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRequest {
    pub request_id: RequestId,
    pub actor: Actor,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub operation: String,
    pub intent_ids: Vec<IntentId>,
    pub parameters: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanResponse {
    Planned(ActionPlan),
    Unsupported { reason: String },
    Invalid { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentCommand {
    Propose(IntentProposal),
    Activate(IntentId),
    Suspend(IntentId),
    Retire(IntentId),
    Supersede { old: IntentId, new: Intent },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExplainTarget {
    Intent(IntentId),
    Resource(ResourceId),
    Capability(CapabilityId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplainResponse {
    pub target: ExplainTarget,
    pub why: WhyChain,
    pub removal_impact: Option<RemovalImpact>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemSnapshot {
    pub graph: SystemGraph,
    pub active_intents: Vec<Intent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableProfileExport {
    pub profile: MachineProfile,
    pub format_version: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileAdoptionRequest {
    pub actor: Actor,
    pub profile: MachineProfile,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileAdoptionResponse {
    pub imported_intent_ids: Vec<IntentId>,
    pub warnings: Vec<String>,
    pub requires_plan: bool,
}
