#![forbid(unsafe_code)]

//! Public, non-privileged Linura SDK surface.
//!
//! This crate intentionally exposes stable domain and protocol types used by
//! clients and integrations. It does not expose Linura Control internals,
//! policy-engine implementation details, providers, or privileged executors.

pub use linura_capability_sdk::{
    CapabilityBlueprint, CapabilityCatalog, CapabilityRelation, CapabilityRelationKind,
    DesiredResourceBlueprint, Resolution,
};
pub use linura_core::{
    ActionPlan, Actor, ActorId, ActorKind, AuthorityClass, Capability, CapabilityId, IntentId,
    PlanId, ProfileId, ProviderId, RequestId, RequirementId, ResourceId, RiskClass, SemanticReason,
    SetupId, SupportLevel, ValidationError, WorkflowId,
};
pub use linura_dbus::{Control1Client as LocalControlClient, TransportError as LocalControlError};
pub use linura_graph::{
    Edge, EdgeKind, Node, NodeId, ObservationRecordOutcome, RemovalImpact, SystemGraph,
};
pub use linura_intent::{
    Intent, IntentProposal, IntentStatus, MachineProfile, Requirement, RequirementKind, Setup,
    SetupValidationError,
};
pub use linura_observation::{
    FreshnessState, ObservationAuthority, ObservationEnvelope, ObservationValidationError,
    ObservedValue, ProviderAvailability, ProviderHealth,
};
pub use linura_protocol::{
    ActionRequest, CapabilitySnapshot, ExplainResponse, ExplainTarget, IntentCommand,
    ObservationExplanation, ObservationRequest, ObservationResponse, ObservationSystemSnapshot,
    PROTOCOL_MAJOR, PlanResponse, PortableProfileExport, PortableSetupExport,
    ProfileAdoptionRequest, ProfileAdoptionResponse, ProtocolVersion, ProviderSnapshot,
    SetupAdoptionRequest, SetupAdoptionResponse, SystemSnapshot,
};
pub use linura_provenance::{ProvenanceKind, ProvenanceRecord, WhyChain};
