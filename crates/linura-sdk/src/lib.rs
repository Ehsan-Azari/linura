#![forbid(unsafe_code)]

//! Public, non-privileged Linura SDK surface.
//!
//! This crate intentionally exposes stable domain and protocol types used by
//! clients and integrations. It does not expose Linura Control internals,
//! policy-engine implementation details, providers, or privileged executors.

pub use linura_capability_sdk::{
    CapabilityBlueprint, CapabilityCatalog, CapabilityRelation, CapabilityRelationKind, Resolution,
};
pub use linura_core::{
    ActionPlan, Actor, ActorKind, AuthorityClass, Capability, CapabilityId, IntentId, PlanId,
    ProfileId, RequestId, RequirementId, ResourceId, RiskClass, SemanticReason, SupportLevel,
    ValidationError, WorkflowId,
};
pub use linura_graph::{Edge, EdgeKind, NodeId, RemovalImpact, SystemGraph};
pub use linura_intent::{
    Intent, IntentProposal, IntentStatus, MachineProfile, Requirement, RequirementKind,
};
pub use linura_protocol::{
    ActionRequest, CapabilitySnapshot, ExplainResponse, ExplainTarget, IntentCommand,
    PlanResponse, PortableProfileExport, ProfileAdoptionRequest, ProfileAdoptionResponse,
    ProtocolVersion, SystemSnapshot, PROTOCOL_MAJOR,
};
pub use linura_provenance::{ProvenanceKind, ProvenanceRecord, WhyChain};
