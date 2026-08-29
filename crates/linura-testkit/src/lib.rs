#![forbid(unsafe_code)]

use linura_core::{ActionPlan, Capability, ResourceId};
use linura_protocol::ActionRequest;
use linura_provider_sdk::{Observation, Provider, ProviderError};

#[derive(Clone, Debug)]
pub struct FakeProvider {
    pub capabilities: Vec<Capability>,
    pub observation: String,
    pub plan: ActionPlan,
    pub fail_observe: bool,
    pub fail_plan: bool,
}

impl Provider for FakeProvider {
    fn id(&self) -> &'static str {
        "test.fake"
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    fn observe(&self, resource: &ResourceId) -> Result<Observation, ProviderError> {
        if self.fail_observe {
            Err(ProviderError::Unavailable(
                "injected observe failure".into(),
            ))
        } else {
            Ok(Observation {
                provider_id: self.id().into(),
                resource: resource.clone(),
                state: self.observation.clone(),
            })
        }
    }

    fn plan(
        &self,
        _request: &ActionRequest,
        _observation: &Observation,
    ) -> Result<ActionPlan, ProviderError> {
        if self.fail_plan {
            Err(ProviderError::Unavailable("injected plan failure".into()))
        } else {
            Ok(self.plan.clone())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureInjector {
    fail_at_step: Option<usize>,
}

impl FailureInjector {
    #[must_use]
    pub fn never() -> Self {
        Self { fail_at_step: None }
    }

    #[must_use]
    pub fn at(step: usize) -> Self {
        Self {
            fail_at_step: Some(step),
        }
    }

    #[must_use]
    pub fn should_fail(&self, step: usize) -> bool {
        self.fail_at_step == Some(step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_injector_is_deterministic() {
        let injector = FailureInjector::at(2);
        assert!(!injector.should_fail(1));
        assert!(injector.should_fail(2));
    }
}
