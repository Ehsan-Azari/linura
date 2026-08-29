#![forbid(unsafe_code)]

/// Canonical successful mutation lifecycle.
///
/// These stages are architectural invariants for managed mutations. A concrete
/// runtime may stop early on failure or denial, but it must not reorder or skip
/// stages on the successful path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationStage {
    RequestIntent,
    Observe,
    Plan,
    Validate,
    Authorize,
    Prepare,
    Execute,
    Verify,
    Commit,
    Audit,
    Reconcile,
}

pub const MUTATION_STAGES: [MutationStage; 11] = [
    MutationStage::RequestIntent,
    MutationStage::Observe,
    MutationStage::Plan,
    MutationStage::Validate,
    MutationStage::Authorize,
    MutationStage::Prepare,
    MutationStage::Execute,
    MutationStage::Verify,
    MutationStage::Commit,
    MutationStage::Audit,
    MutationStage::Reconcile,
];

impl MutationStage {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestIntent => "request-intent",
            Self::Observe => "observe",
            Self::Plan => "plan",
            Self::Validate => "validate",
            Self::Authorize => "authorize",
            Self::Prepare => "prepare",
            Self::Execute => "execute",
            Self::Verify => "verify",
            Self::Commit => "commit",
            Self::Audit => "audit",
            Self::Reconcile => "reconcile",
        }
    }

    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::RequestIntent => Some(Self::Observe),
            Self::Observe => Some(Self::Plan),
            Self::Plan => Some(Self::Validate),
            Self::Validate => Some(Self::Authorize),
            Self::Authorize => Some(Self::Prepare),
            Self::Prepare => Some(Self::Execute),
            Self::Execute => Some(Self::Verify),
            Self::Verify => Some(Self::Commit),
            Self::Commit => Some(Self::Audit),
            Self::Audit => Some(Self::Reconcile),
            Self::Reconcile => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationProgress {
    completed: Vec<MutationStage>,
}

impl Default for MutationProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl MutationProgress {
    #[must_use]
    pub fn new() -> Self {
        Self {
            completed: vec![MutationStage::RequestIntent],
        }
    }

    #[must_use]
    pub fn current(&self) -> MutationStage {
        self.completed
            .last()
            .copied()
            .unwrap_or(MutationStage::RequestIntent)
    }

    #[must_use]
    pub fn completed(&self) -> &[MutationStage] {
        &self.completed
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.current() == MutationStage::Reconcile
    }

    pub fn advance(&mut self, attempted: MutationStage) -> Result<(), MutationTransitionError> {
        let current = self.current();
        let expected = current.next();
        if expected == Some(attempted) {
            self.completed.push(attempted);
            Ok(())
        } else {
            Err(MutationTransitionError {
                current,
                expected,
                attempted,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MutationTransitionError {
    pub current: MutationStage,
    pub expected: Option<MutationStage>,
    pub attempted: MutationStage,
}

/// System lifecycle events are separate from the per-mutation lifecycle above.
/// They describe typed workflows around bootstrap, update, reconciliation and
/// recovery boundaries.
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
        if self.id.trim().is_empty() {
            return Err("workflow id cannot be empty");
        }
        if self.steps.is_empty() {
            return Err("lifecycle workflow must have at least one typed step");
        }
        if self
            .steps
            .iter()
            .any(|step| step.capability.trim().is_empty() || step.operation.trim().is_empty())
        {
            return Err("lifecycle steps require capability and operation");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_mutation_lifecycle_has_eleven_ordered_stages() {
        assert_eq!(MUTATION_STAGES.len(), 11);
        assert_eq!(MUTATION_STAGES[0], MutationStage::RequestIntent);
        assert_eq!(MUTATION_STAGES[10], MutationStage::Reconcile);
    }

    #[test]
    fn mutation_progress_rejects_skipped_stages() {
        let mut progress = MutationProgress::new();
        assert_eq!(
            progress.advance(MutationStage::Plan),
            Err(MutationTransitionError {
                current: MutationStage::RequestIntent,
                expected: Some(MutationStage::Observe),
                attempted: MutationStage::Plan,
            })
        );
    }

    #[test]
    fn mutation_progress_can_complete_the_canonical_path() {
        let mut progress = MutationProgress::new();
        for stage in MUTATION_STAGES.iter().copied().skip(1) {
            assert!(progress.advance(stage).is_ok());
        }
        assert!(progress.is_complete());
        assert_eq!(progress.completed(), MUTATION_STAGES.as_slice());
    }

    #[test]
    fn arbitrary_empty_hooks_are_not_valid_workflows() {
        let workflow = LifecycleWorkflow {
            id: "hook".into(),
            event: LifecycleEvent::AfterUpdate,
            steps: vec![],
        };
        assert!(workflow.validate().is_err());
    }
}
