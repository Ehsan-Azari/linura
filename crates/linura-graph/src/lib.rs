#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use linura_core::{CapabilityId, IntentId, ProviderId, RequirementId, ResourceId, SetupId};
use linura_observation::{FreshnessState, ObservationEnvelope};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NodeId {
    Intent(IntentId),
    Setup(SetupId),
    Requirement(RequirementId),
    Capability(CapabilityId),
    Provider(ProviderId),
    Resource(ResourceId),
    Evidence(String),
    Workflow(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeKind {
    Requires,
    Provides,
    Conflicts,
    Replaces,
    Recommends,
    Optional,
    Owns,
    SharedBy,
    DerivedFrom,
    Realizes,
    ObservedBy,
    EvidenceFor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationRecordOutcome {
    CurrentStateUpdated,
    HistoricalEvidenceRetained,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SystemGraph {
    nodes: BTreeMap<NodeId, Node>,
    edges: Vec<Edge>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RemovalImpact {
    pub removable: BTreeSet<NodeId>,
    pub retained_shared: BTreeSet<NodeId>,
    pub conflicts: Vec<Edge>,
}

impl SystemGraph {
    pub fn upsert_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Remove a node and every incident edge.
    ///
    /// This is used by bounded live observation history so evicted evidence
    /// cannot leave dangling causal edges in the in-memory graph.
    pub fn remove_node(&mut self, id: &NodeId) -> Option<Node> {
        let removed = self.nodes.remove(id);
        if removed.is_some() {
            self.edges.retain(|edge| &edge.from != id && &edge.to != id);
        }
        removed
    }

    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn add_edge(&mut self, edge: Edge) {
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
    }

    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn record_observation(
        &mut self,
        observation: &ObservationEnvelope,
        freshness: FreshnessState,
    ) -> ObservationRecordOutcome {
        let provider = NodeId::Provider(observation.provider.clone());
        let resource = NodeId::Resource(observation.resource.clone());
        let capability = NodeId::Capability(observation.capability.clone());
        let evidence = NodeId::Evidence(observation.evidence_id());

        self.upsert_node(Node {
            id: provider.clone(),
            attributes: BTreeMap::from([("kind".into(), "provider".into())]),
        });
        self.upsert_node(Node {
            id: capability.clone(),
            attributes: BTreeMap::from([("kind".into(), "capability".into())]),
        });
        self.upsert_node(Node {
            id: evidence.clone(),
            attributes: BTreeMap::from([
                ("kind".into(), "observation-evidence".into()),
                ("provider".into(), observation.provider.as_str().into()),
                ("resource".into(), observation.resource.as_str().into()),
                ("capability".into(), observation.capability.as_str().into()),
                ("authority".into(), observation.authority.as_str().into()),
                (
                    "observed_at_unix_ms".into(),
                    observation.observed_at_unix_ms.to_string(),
                ),
                ("sequence".into(), observation.sequence.to_string()),
                ("freshness".into(), freshness.as_str().into()),
            ]),
        });

        self.add_edge(Edge {
            from: provider.clone(),
            to: capability.clone(),
            kind: EdgeKind::Provides,
            reason: "provider declares observation capability".into(),
        });
        self.add_edge(Edge {
            from: resource.clone(),
            to: provider,
            kind: EdgeKind::ObservedBy,
            reason: "resource has evidence from this authoritative provider".into(),
        });
        self.add_edge(Edge {
            from: capability,
            to: resource.clone(),
            kind: EdgeKind::Realizes,
            reason: "observation capability realizes this resource view".into(),
        });
        self.add_edge(Edge {
            from: evidence,
            to: resource.clone(),
            kind: EdgeKind::EvidenceFor,
            reason: "observation envelope is evidence for the resource state".into(),
        });

        let should_update = freshness == FreshnessState::Current
            && self
                .nodes
                .get(&resource)
                .is_none_or(|current| observation_is_newer(current, observation));

        if should_update {
            let mut resource_attributes = BTreeMap::from([
                ("kind".into(), "resource".into()),
                ("provider".into(), observation.provider.as_str().into()),
                ("capability".into(), observation.capability.as_str().into()),
                (
                    "observed_at_unix_ms".into(),
                    observation.observed_at_unix_ms.to_string(),
                ),
                ("sequence".into(), observation.sequence.to_string()),
                ("freshness".into(), freshness.as_str().into()),
                ("authority".into(), observation.authority.as_str().into()),
                ("evidence_id".into(), observation.evidence_id()),
            ]);
            for (key, value) in &observation.attributes {
                resource_attributes.insert(format!("observed.{key}"), value.to_string());
            }
            self.upsert_node(Node {
                id: resource,
                attributes: resource_attributes,
            });
            ObservationRecordOutcome::CurrentStateUpdated
        } else {
            self.nodes.entry(resource.clone()).or_insert_with(|| Node {
                id: resource,
                attributes: BTreeMap::from([("kind".into(), "resource".into())]),
            });
            ObservationRecordOutcome::HistoricalEvidenceRetained
        }
    }

    #[must_use]
    pub fn conflicts_for(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| {
                edge.kind == EdgeKind::Conflicts && (&edge.from == node || &edge.to == node)
            })
            .collect()
    }

    #[must_use]
    pub fn reasons_for(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| {
                &edge.to == node
                    && matches!(
                        edge.kind,
                        EdgeKind::DerivedFrom
                            | EdgeKind::Realizes
                            | EdgeKind::Requires
                            | EdgeKind::Owns
                            | EdgeKind::EvidenceFor
                    )
            })
            .collect()
    }

    #[must_use]
    pub fn removal_impact(&self, retired_origin: &NodeId) -> RemovalImpact {
        let mut origins_by_target: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for edge in &self.edges {
            if matches!(
                edge.kind,
                EdgeKind::DerivedFrom | EdgeKind::Owns | EdgeKind::Requires | EdgeKind::Realizes
            ) {
                origins_by_target
                    .entry(edge.to.clone())
                    .or_default()
                    .insert(edge.from.clone());
            }
        }

        let mut impact = RemovalImpact::default();
        for (target, origins) in origins_by_target {
            if origins.contains(retired_origin) {
                if origins.len() == 1 {
                    impact.removable.insert(target);
                } else {
                    impact.retained_shared.insert(target);
                }
            }
        }
        impact.conflicts = self
            .edges
            .iter()
            .filter(|edge| {
                edge.kind == EdgeKind::Conflicts
                    && (&edge.from == retired_origin || &edge.to == retired_origin)
            })
            .cloned()
            .collect();
        impact
    }
}

fn observation_is_newer(current: &Node, incoming: &ObservationEnvelope) -> bool {
    let current_time = current
        .attributes
        .get("observed_at_unix_ms")
        .and_then(|value| value.parse::<u64>().ok());
    let current_sequence = current
        .attributes
        .get("sequence")
        .and_then(|value| value.parse::<u64>().ok());
    let current_provider = current.attributes.get("provider").map(String::as_str);

    match (current_provider, current_time, current_sequence) {
        // Native observers own a monotonic sequence for their lifetime. Prefer
        // that sequence over realtime timestamps so an NTP/admin clock step
        // backwards cannot make genuinely newer evidence look historical.
        (Some(provider), _, Some(sequence)) if provider == incoming.provider.as_str() => {
            incoming.sequence > sequence
        }
        (_, Some(time), Some(sequence)) => {
            (incoming.observed_at_unix_ms, incoming.sequence) > (time, sequence)
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linura_core::ValidationError;
    use linura_observation::{ObservationAuthority, ObservedValue};

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn observation(time: u64, sequence: u64, state: &str) -> ObservationEnvelope {
        ObservationEnvelope {
            provider: id(ProviderId::new("systemd")),
            resource: id(ResourceId::new("systemd:unit:sshd.service")),
            capability: id(CapabilityId::new("systemd.unit.observe")),
            authority: ObservationAuthority::NativeApi,
            observed_at_unix_ms: time,
            valid_for_ms: 100,
            sequence,
            attributes: BTreeMap::from([(
                "active_state".into(),
                ObservedValue::Text(state.into()),
            )]),
        }
    }

    #[test]
    fn shared_resource_is_retained_when_one_origin_is_retired() {
        let ai = NodeId::Intent(id(IntentId::new("intent:ai")));
        let gaming = NodeId::Intent(id(IntentId::new("intent:gaming")));
        let driver = NodeId::Resource(id(ResourceId::new("package:nvidia-utils")));
        let mut graph = SystemGraph::default();
        graph.add_edge(Edge {
            from: ai.clone(),
            to: driver.clone(),
            kind: EdgeKind::Owns,
            reason: "GPU compute".into(),
        });
        graph.add_edge(Edge {
            from: gaming,
            to: driver.clone(),
            kind: EdgeKind::Owns,
            reason: "gaming GPU".into(),
        });
        let impact = graph.removal_impact(&ai);
        assert!(impact.retained_shared.contains(&driver));
        assert!(!impact.removable.contains(&driver));
    }

    #[test]
    fn setup_can_be_a_causal_origin() {
        let setup = NodeId::Setup(id(SetupId::new("setup:rust-development")));
        let intent = NodeId::Intent(id(IntentId::new("intent:rust-development")));
        let mut graph = SystemGraph::default();
        graph.add_edge(Edge {
            from: setup.clone(),
            to: intent.clone(),
            kind: EdgeKind::DerivedFrom,
            reason: "intent adopted from reusable setup".into(),
        });
        assert_eq!(graph.reasons_for(&intent).len(), 1);
        assert_eq!(graph.reasons_for(&intent)[0].from, setup);
    }

    #[test]
    fn observation_populates_resource_provider_capability_and_evidence_nodes() {
        let observation = observation(10, 1, "active");
        let mut graph = SystemGraph::default();
        assert_eq!(
            graph.record_observation(&observation, FreshnessState::Current),
            ObservationRecordOutcome::CurrentStateUpdated
        );

        let resource = NodeId::Resource(observation.resource.clone());
        let node = graph
            .node(&resource)
            .unwrap_or_else(|| unreachable!("resource must exist"));
        assert_eq!(
            node.attributes.get("freshness").map(String::as_str),
            Some("current")
        );
        assert_eq!(
            node.attributes
                .get("observed.active_state")
                .map(String::as_str),
            Some("active")
        );
        assert_eq!(graph.nodes().count(), 4);
        assert_eq!(graph.edges().len(), 4);
        assert!(graph.reasons_for(&resource).iter().any(|edge| {
            edge.kind == EdgeKind::Realizes
                && edge.from == NodeId::Capability(observation.capability.clone())
        }));
    }

    #[test]
    fn removing_evidence_also_removes_its_incident_edges() {
        let observation = observation(10, 1, "active");
        let evidence = NodeId::Evidence(observation.evidence_id());
        let mut graph = SystemGraph::default();
        assert_eq!(
            graph.record_observation(&observation, FreshnessState::Current),
            ObservationRecordOutcome::CurrentStateUpdated
        );
        let edge_count = graph.edges().len();
        assert!(graph.remove_node(&evidence).is_some());
        assert!(graph.node(&evidence).is_none());
        assert!(graph.edges().len() < edge_count);
        assert!(
            graph
                .edges()
                .iter()
                .all(|edge| edge.from != evidence && edge.to != evidence)
        );
    }

    #[test]
    fn same_provider_sequence_survives_realtime_clock_rollback() {
        let before_clock_step = observation(2_000, 1, "active");
        let after_clock_step = observation(1_000, 2, "inactive");
        let mut graph = SystemGraph::default();

        assert_eq!(
            graph.record_observation(&before_clock_step, FreshnessState::Current),
            ObservationRecordOutcome::CurrentStateUpdated
        );
        assert_eq!(
            graph.record_observation(&after_clock_step, FreshnessState::Current),
            ObservationRecordOutcome::CurrentStateUpdated
        );

        let resource = NodeId::Resource(after_clock_step.resource.clone());
        let node = graph
            .node(&resource)
            .unwrap_or_else(|| unreachable!("resource must exist"));
        assert_eq!(
            node.attributes
                .get("observed.active_state")
                .map(String::as_str),
            Some("inactive")
        );
        assert_eq!(
            node.attributes.get("sequence").map(String::as_str),
            Some("2")
        );
        assert_eq!(
            node.attributes
                .get("observed_at_unix_ms")
                .map(String::as_str),
            Some("1000")
        );
    }

    #[test]
    fn older_or_untrusted_evidence_cannot_replace_current_resource_state() {
        let newer = observation(2_000, 2, "active");
        let older = observation(1_000, 1, "inactive");
        let stale_newer = observation(3_000, 3, "failed");
        let mut graph = SystemGraph::default();

        assert_eq!(
            graph.record_observation(&newer, FreshnessState::Current),
            ObservationRecordOutcome::CurrentStateUpdated
        );
        assert_eq!(
            graph.record_observation(&older, FreshnessState::Current),
            ObservationRecordOutcome::HistoricalEvidenceRetained
        );
        assert_eq!(
            graph.record_observation(&stale_newer, FreshnessState::Stale),
            ObservationRecordOutcome::HistoricalEvidenceRetained
        );

        let resource = NodeId::Resource(newer.resource.clone());
        let node = graph
            .node(&resource)
            .unwrap_or_else(|| unreachable!("resource must exist"));
        assert_eq!(
            node.attributes
                .get("observed.active_state")
                .map(String::as_str),
            Some("active")
        );
        assert_eq!(
            node.attributes.get("sequence").map(String::as_str),
            Some("2")
        );
        assert_eq!(graph.nodes().count(), 6);
    }
}
