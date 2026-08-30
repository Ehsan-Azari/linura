#![forbid(unsafe_code)]

use linura_core::{
    ActionPlan, Actor, Capability, CapabilityId, IntentId, ProviderId, RequestId, ResourceId,
    SetupId,
};
use linura_graph::{RemovalImpact, SystemGraph};
use linura_intent::{Intent, IntentProposal, MachineProfile, Setup};
use linura_observation::{FreshnessState, ObservationEnvelope, ProviderHealth};
use linura_provenance::WhyChain;

pub const PROTOCOL_MAJOR: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub product_version: &'static str,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self {
            major: PROTOCOL_MAJOR,
            product_version: env!("CARGO_PKG_VERSION"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilitySnapshot {
    pub profile_id: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSnapshot {
    pub providers: Vec<ProviderHealth>,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationRequest {
    pub provider: ProviderId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationResponse {
    pub observation: ObservationEnvelope,
    pub freshness: FreshnessState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationSystemSnapshot {
    pub graph: SystemGraph,
    pub providers: ProviderSnapshot,
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
    Planned(Box<ActionPlan>),
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
    Setup(SetupId),
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
pub struct ObservationExplanation {
    pub resource: ResourceId,
    pub provider: ProviderId,
    pub capability: CapabilityId,
    pub freshness: FreshnessState,
    pub evidence_id: String,
    pub authority: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemSnapshot {
    pub graph: SystemGraph,
    pub active_intents: Vec<Intent>,
}

/// Self-contained portable setup bundle. It carries the reusable setup graph and
/// the intent definitions needed to adopt it on the same or another machine.
/// Secret values are never embedded; setup definitions carry only secret refs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableSetupExport {
    pub root_setup_id: SetupId,
    pub setups: Vec<Setup>,
    pub intents: Vec<Intent>,
    pub format_version: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupAdoptionRequest {
    pub actor: Actor,
    pub bundle: PortableSetupExport,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupAdoptionResponse {
    pub imported_setup_ids: Vec<SetupId>,
    pub imported_intent_ids: Vec<IntentId>,
    pub missing_secret_refs: Vec<String>,
    pub warnings: Vec<String>,
    pub requires_plan: bool,
}

/// A portable machine profile export is self-contained: the profile references
/// setup/intent IDs while this bundle carries those definitions for replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableProfileExport {
    pub profile: MachineProfile,
    pub setups: Vec<Setup>,
    pub intents: Vec<Intent>,
    pub format_version: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileAdoptionRequest {
    pub actor: Actor,
    pub bundle: PortableProfileExport,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileAdoptionResponse {
    pub imported_setup_ids: Vec<SetupId>,
    pub imported_intent_ids: Vec<IntentId>,
    pub missing_secret_refs: Vec<String>,
    pub warnings: Vec<String>,
    pub requires_plan: bool,
}
