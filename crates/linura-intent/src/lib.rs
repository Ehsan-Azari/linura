#![forbid(unsafe_code)]

use linura_core::{Actor, IntentId, ProfileId, RequirementId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentStatus {
    Proposed,
    Active,
    Suspended,
    Superseded,
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementKind {
    Goal,
    Constraint,
    Preference,
    Prohibition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    pub id: RequirementId,
    pub kind: RequirementKind,
    pub statement: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Intent {
    pub id: IntentId,
    pub actor: Actor,
    pub statement: String,
    pub status: IntentStatus,
    pub requirements: Vec<Requirement>,
    pub supersedes: Vec<IntentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentProposal {
    pub intent: Intent,
    pub interpretation_notes: Vec<String>,
    pub ambiguities: Vec<String>,
    pub confidence_basis: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineProfile {
    pub id: ProfileId,
    pub name: String,
    pub intent_ids: Vec<IntentId>,
    pub portable_constraints: Vec<String>,
    pub hardware_hints: Vec<String>,
}

impl Intent {
    pub fn is_managed(&self) -> bool {
        matches!(self.status, IntentStatus::Active | IntentStatus::Suspended)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linura_core::{ActorKind, ValidationError};

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn retired_intent_is_not_managed() {
        let intent = Intent {
            id: id(IntentId::new("intent:test")),
            actor: Actor { id: "uid:1000".into(), kind: ActorKind::Human, interactive: true },
            statement: "I no longer need Kubernetes".into(),
            status: IntentStatus::Retired,
            requirements: vec![],
            supersedes: vec![],
        };
        assert!(!intent.is_managed());
    }
}
