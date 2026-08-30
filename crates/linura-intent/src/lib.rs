#![forbid(unsafe_code)]

use linura_core::{Actor, IntentId, ProfileId, RequirementId, SetupId};

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

/// A reusable, portable composition of intent rather than a recorded command
/// sequence or an exact filesystem snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Setup {
    pub id: SetupId,
    pub name: String,
    pub description: String,
    pub revision: u32,
    pub intent_ids: Vec<IntentId>,
    pub included_setup_ids: Vec<SetupId>,
    pub portable_constraints: Vec<String>,
    pub required_secret_refs: Vec<String>,
    pub hardware_hints: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupValidationError {
    EmptyName,
    ZeroRevision,
    EmptyComposition,
    SelfReference,
    EmptySecretReference,
}

impl Setup {
    pub fn validate(&self) -> Result<(), SetupValidationError> {
        if self.name.trim().is_empty() {
            return Err(SetupValidationError::EmptyName);
        }
        if self.revision == 0 {
            return Err(SetupValidationError::ZeroRevision);
        }
        if self.intent_ids.is_empty() && self.included_setup_ids.is_empty() {
            return Err(SetupValidationError::EmptyComposition);
        }
        if self.included_setup_ids.iter().any(|id| id == &self.id) {
            return Err(SetupValidationError::SelfReference);
        }
        if self
            .required_secret_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
        {
            return Err(SetupValidationError::EmptySecretReference);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineProfile {
    pub id: ProfileId,
    pub name: String,
    pub setup_ids: Vec<SetupId>,
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
            actor: Actor {
                id: "uid:1000".into(),
                kind: ActorKind::Human,
                interactive: true,
            },
            statement: "I no longer need Kubernetes".into(),
            status: IntentStatus::Retired,
            requirements: vec![],
            supersedes: vec![],
        };
        assert!(!intent.is_managed());
    }

    #[test]
    fn setup_requires_portable_composition_and_revision() {
        let setup = Setup {
            id: id(SetupId::new("setup:rust-development")),
            name: "Rust development".into(),
            description: "Reusable Rust development environment".into(),
            revision: 1,
            intent_ids: vec![id(IntentId::new("intent:rust-development"))],
            included_setup_ids: vec![],
            portable_constraints: vec!["stable toolchain".into()],
            required_secret_refs: vec!["credential:github".into()],
            hardware_hints: vec![],
        };
        assert_eq!(setup.validate(), Ok(()));
    }

    #[test]
    fn setup_cannot_include_itself() {
        let setup_id = id(SetupId::new("setup:self"));
        let setup = Setup {
            id: setup_id.clone(),
            name: "Self".into(),
            description: String::new(),
            revision: 1,
            intent_ids: vec![],
            included_setup_ids: vec![setup_id],
            portable_constraints: vec![],
            required_secret_refs: vec![],
            hardware_hints: vec![],
        };
        assert_eq!(setup.validate(), Err(SetupValidationError::SelfReference));
    }
}
