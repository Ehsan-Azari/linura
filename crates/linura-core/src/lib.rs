#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};

macro_rules! typed_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
                let value = value.into();
                validate_token($label, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

typed_id!(RequestId, "request id");
typed_id!(PlanId, "plan id");
typed_id!(IntentId, "intent id");
typed_id!(SetupId, "setup id");
typed_id!(RequirementId, "requirement id");
typed_id!(ResourceId, "resource id");
typed_id!(CapabilityId, "capability id");
typed_id!(ProviderId, "provider id");
typed_id!(WorkflowId, "workflow id");
typed_id!(ProfileId, "profile id");

fn validate_token(label: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Empty(label));
    }
    if value.len() > 256 {
        return Err(ValidationError::TooLong(label));
    }
    if value.chars().any(char::is_control) {
        return Err(ValidationError::ControlCharacter(label));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorKind {
    Human,
    Service,
    Agent,
    Remote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Actor {
    pub id: String,
    pub kind: ActorKind,
    pub interactive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportLevel {
    Supported,
    Unsupported,
    Degraded,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    pub id: CapabilityId,
    pub support: SupportLevel,
    pub provider: Option<ProviderId>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RiskClass {
    ReadOnly,
    UserState,
    SystemMutation,
    SecuritySensitive,
    Destructive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityClass {
    Observed,
    Declared,
    Inferred,
    Proposed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preconditions {
    pub statements: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Compensation {
    None,
    Effect(Box<Effect>),
    Manual { instructions: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Effect {
    pub id: String,
    pub executor: String,
    pub operation: String,
    pub arguments: Vec<(String, String)>,
    pub compensation: Compensation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verification {
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticReason {
    pub summary: String,
    pub intent_ids: Vec<IntentId>,
    pub requirement_ids: Vec<RequirementId>,
    pub capability_ids: Vec<CapabilityId>,
}

impl SemanticReason {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.summary.trim().is_empty() {
            return Err(ValidationError::Empty("semantic reason"));
        }
        if self.intent_ids.is_empty()
            && self.requirement_ids.is_empty()
            && self.capability_ids.is_empty()
        {
            return Err(ValidationError::MissingSemanticOrigin);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPlan {
    pub id: PlanId,
    pub request_id: RequestId,
    pub actor: Actor,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub risk: RiskClass,
    pub reason: SemanticReason,
    pub preconditions: Preconditions,
    pub effects: Vec<Effect>,
    pub verification: Vec<Verification>,
}

impl ActionPlan {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.reason.validate()?;
        if self.effects.is_empty() {
            return Err(ValidationError::NoEffects);
        }
        if self.verification.is_empty() {
            return Err(ValidationError::NoVerification);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    Empty(&'static str),
    TooLong(&'static str),
    ControlCharacter(&'static str),
    MissingSemanticOrigin,
    NoEffects,
    NoVerification,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty(label) => write!(f, "{label} cannot be empty"),
            Self::TooLong(label) => write!(f, "{label} is too long"),
            Self::ControlCharacter(label) => write!(f, "{label} contains control characters"),
            Self::MissingSemanticOrigin => f.write_str(
                "managed state must retain an intent, requirement, or capability origin",
            ),
            Self::NoEffects => f.write_str("an action plan must contain at least one effect"),
            Self::NoVerification => f.write_str("an action plan must contain verification"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor() -> Actor {
        Actor {
            id: "uid:1000".into(),
            kind: ActorKind::Human,
            interactive: true,
        }
    }

    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn identifiers_reject_control_characters() {
        assert!(RequestId::new("bad\nvalue").is_err());
    }

    #[test]
    fn managed_plan_requires_semantic_origin() {
        let plan = ActionPlan {
            id: id(PlanId::new("plan-1")),
            request_id: id(RequestId::new("req-1")),
            actor: actor(),
            resource: id(ResourceId::new("service:sshd")),
            capability: id(CapabilityId::new("remote.ssh")),
            risk: RiskClass::SystemMutation,
            reason: SemanticReason {
                summary: "enable SSH".into(),
                intent_ids: vec![],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            preconditions: Preconditions { statements: vec![] },
            effects: vec![Effect {
                id: "enable".into(),
                executor: "systemd".into(),
                operation: "set-enabled".into(),
                arguments: vec![("unit".into(), "sshd.service".into())],
                compensation: Compensation::None,
            }],
            verification: vec![Verification {
                description: "enabled state observed".into(),
            }],
        };
        assert_eq!(plan.validate(), Err(ValidationError::MissingSemanticOrigin));
    }
}
