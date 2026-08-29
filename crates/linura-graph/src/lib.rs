#![forbid(unsafe_code)]

use linura_core::{CapabilityId, IntentId, RequirementId, ResourceId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NodeId {
    Intent(IntentId),
    Requirement(RequirementId),
    Capability(CapabilityId),
    Resource(ResourceId),
    Workflow(String),
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SystemGraph {
    edges: Vec<Edge>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RemovalImpact {
    pub removable: BTreeSet<NodeId>,
    pub retained_shared: BTreeSet<NodeId>,
    pub conflicts: Vec<Edge>,
}

impl SystemGraph {
    pub fn add_edge(&mut self, edge: Edge) {
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn conflicts_for(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| {
                edge.kind == EdgeKind::Conflicts && (&edge.from == node || &edge.to == node)
            })
            .collect()
    }

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
                    )
            })
            .collect()
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use linura_core::ValidationError;

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
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
}
