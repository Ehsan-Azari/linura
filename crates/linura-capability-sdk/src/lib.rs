#![forbid(unsafe_code)]

use linura_core::CapabilityId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityRelationKind {
    Requires,
    Provides,
    Conflicts,
    Replaces,
    Recommends,
    Optional,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRelation {
    pub kind: CapabilityRelationKind,
    pub capability: CapabilityId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityBlueprint {
    pub id: CapabilityId,
    pub title: String,
    pub relations: Vec<CapabilityRelation>,
    pub desired_resources: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapabilityCatalog {
    blueprints: BTreeMap<CapabilityId, CapabilityBlueprint>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Resolution {
    pub selected: BTreeSet<CapabilityId>,
    pub conflicts: Vec<(CapabilityId, CapabilityId)>,
    pub missing: BTreeSet<CapabilityId>,
}

impl CapabilityCatalog {
    pub fn register(&mut self, blueprint: CapabilityBlueprint) {
        self.blueprints.insert(blueprint.id.clone(), blueprint);
    }

    pub fn resolve(&self, requested: &[CapabilityId]) -> Resolution {
        let mut result = Resolution::default();
        let mut pending: Vec<CapabilityId> = requested.to_vec();
        while let Some(id) = pending.pop() {
            if !result.selected.insert(id.clone()) {
                continue;
            }
            let Some(blueprint) = self.blueprints.get(&id) else {
                result.missing.insert(id);
                continue;
            };
            for relation in &blueprint.relations {
                match relation.kind {
                    CapabilityRelationKind::Requires => pending.push(relation.capability.clone()),
                    CapabilityRelationKind::Conflicts
                        if result.selected.contains(&relation.capability) =>
                    {
                        result
                            .conflicts
                            .push((id.clone(), relation.capability.clone()));
                    }
                    _ => {}
                }
            }
        }
        for selected in &result.selected {
            if let Some(blueprint) = self.blueprints.get(selected) {
                for relation in &blueprint.relations {
                    if relation.kind == CapabilityRelationKind::Conflicts
                        && result.selected.contains(&relation.capability)
                    {
                        let pair = (selected.clone(), relation.capability.clone());
                        let reverse = (relation.capability.clone(), selected.clone());
                        if !result.conflicts.contains(&pair) && !result.conflicts.contains(&reverse)
                        {
                            result.conflicts.push(pair);
                        }
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linura_core::ValidationError;

    fn id(result: Result<CapabilityId, ValidationError>) -> CapabilityId {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn required_capabilities_are_selected() {
        let ai = id(CapabilityId::new("development.ai"));
        let python = id(CapabilityId::new("development.python"));
        let mut catalog = CapabilityCatalog::default();
        catalog.register(CapabilityBlueprint {
            id: ai.clone(),
            title: "AI development".into(),
            relations: vec![CapabilityRelation {
                kind: CapabilityRelationKind::Requires,
                capability: python.clone(),
            }],
            desired_resources: vec![],
        });
        catalog.register(CapabilityBlueprint {
            id: python.clone(),
            title: "Python".into(),
            relations: vec![],
            desired_resources: vec![],
        });
        let resolution = catalog.resolve(&[ai]);
        assert!(resolution.selected.contains(&python));
        assert!(resolution.missing.is_empty());
    }
}
