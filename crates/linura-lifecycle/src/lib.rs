#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleEvent {
    BeforeBootstrap,
    AfterBootstrap,
    BeforeUpdate,
    AfterUpdate,
    BeforeReconcile,
    AfterReconcile,
    BeforeRecovery,
    AfterRecovery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleStep {
    pub capability: String,
    pub operation: String,
    pub required_permissions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleWorkflow {
    pub id: String,
    pub event: LifecycleEvent,
    pub steps: Vec<LifecycleStep>,
}

impl LifecycleWorkflow {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() { return Err("workflow id cannot be empty"); }
        if self.steps.is_empty() { return Err("lifecycle workflow must have at least one typed step"); }
        if self.steps.iter().any(|step| step.capability.trim().is_empty() || step.operation.trim().is_empty()) {
            return Err("lifecycle steps require capability and operation");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_empty_hooks_are_not_valid_workflows() {
        let workflow = LifecycleWorkflow { id: "hook".into(), event: LifecycleEvent::AfterUpdate, steps: vec![] };
        assert!(workflow.validate().is_err());
    }
}
