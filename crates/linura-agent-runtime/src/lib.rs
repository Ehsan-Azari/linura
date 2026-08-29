#![forbid(unsafe_code)]

use linura_core::Actor;
use linura_intent::IntentProposal;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpecialistRole {
    Coordinator,
    Hardware,
    Security,
    Developer,
    Desktop,
    Productivity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentContext {
    pub actor: Actor,
    pub offline: bool,
    pub allowed_specialists: Vec<SpecialistRole>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentError {
    Unavailable(String),
    InvalidProposal(String),
    ProviderFailure(String),
}

pub trait IntentInterpreter: Send + Sync {
    fn id(&self) -> &'static str;
    fn propose(&self, context: &AgentContext, statement: &str) -> Result<IntentProposal, AgentError>;
}

pub trait Specialist: Send + Sync {
    fn role(&self) -> SpecialistRole;
    fn advise(&self, context: &AgentContext, proposal: &IntentProposal) -> Result<Vec<String>, AgentError>;
}

/// Agent runtimes may propose structured intents and advice only. They receive no
/// privileged executor handle and cannot apply system effects directly.
#[derive(Default)]
pub struct AgentRuntime {
    interpreters: Vec<Box<dyn IntentInterpreter>>,
    specialists: Vec<Box<dyn Specialist>>,
}

impl std::fmt::Debug for AgentRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuntime")
            .field("interpreter_count", &self.interpreters.len())
            .field("specialist_count", &self.specialists.len())
            .finish()
    }
}

impl AgentRuntime {
    pub fn register_interpreter(&mut self, interpreter: Box<dyn IntentInterpreter>) {
        self.interpreters.push(interpreter);
    }

    pub fn register_specialist(&mut self, specialist: Box<dyn Specialist>) {
        self.specialists.push(specialist);
    }
}
