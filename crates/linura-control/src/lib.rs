#![forbid(unsafe_code)]

use std::fmt::{Debug, Display, Formatter};

use linura_core::{ActionPlan, PlanId};
use linura_graph::SystemGraph;
use linura_intent::Intent;
use linura_lifecycle::{MutationProgress, MutationStage, MutationTransitionError};
use linura_policy::{ApprovalClass, PolicyDecision, PolicyEngine};
use linura_protocol::{ActionRequest, PlanResponse};
use linura_provider_sdk::{
    ExecutionReceipt, Observation, Provider, ProviderError, VerificationReceipt,
};

/// Evidence that the policy decision has been satisfied at the authority boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationEvidence {
    PolicyAllow,
    Approval {
        class: ApprovalClass,
        approver: String,
        mechanism: String,
    },
}

impl AuthorizationEvidence {
    #[must_use]
    pub fn satisfies(&self, decision: &PolicyDecision) -> bool {
        match (self, decision) {
            (Self::PolicyAllow, PolicyDecision::Allow) => true,
            (
                Self::Approval { class, .. },
                PolicyDecision::RequireApproval {
                    class: required, ..
                },
            ) => class == required,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedMutation {
    pub plan_id: PlanId,
    pub durable_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    pub plan_id: PlanId,
    pub durable_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditReceipt {
    pub plan_id: PlanId,
    pub durable_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationReceipt {
    pub plan_id: PlanId,
    pub durable_ref: String,
}

/// Completed successful-path evidence for the canonical eleven-stage lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationOutcome {
    pub plan: ActionPlan,
    pub pre_observation: Observation,
    pub policy_decision: PolicyDecision,
    pub authorization: AuthorizationEvidence,
    pub prepared: PreparedMutation,
    pub execution: ExecutionReceipt,
    pub post_observation: Observation,
    pub verification: VerificationReceipt,
    pub commit: CommitReceipt,
    pub audit: AuditReceipt,
    pub reconciliation: ReconciliationReceipt,
    pub completed_stages: Vec<MutationStage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationRuntimeError {
    pub message: String,
}

impl MutationRuntimeError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for MutationRuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MutationRuntimeError {}

/// Runtime ports for the stages that require concrete OS, persistence, approval,
/// audit and reconciliation implementations.
///
/// `linura-control` owns stage ordering. Implementations provide stage behavior;
/// they do not get to reorder the authority lifecycle.
pub trait MutationRuntime {
    fn authorize(
        &mut self,
        plan: &ActionPlan,
        decision: &PolicyDecision,
    ) -> Result<AuthorizationEvidence, MutationRuntimeError>;

    fn prepare(
        &mut self,
        plan: &ActionPlan,
        authorization: &AuthorizationEvidence,
    ) -> Result<PreparedMutation, MutationRuntimeError>;

    fn execute(
        &mut self,
        plan: &ActionPlan,
        prepared: &PreparedMutation,
    ) -> Result<ExecutionReceipt, MutationRuntimeError>;

    fn verify(
        &mut self,
        plan: &ActionPlan,
        execution: &ExecutionReceipt,
        post_observation: &Observation,
    ) -> Result<VerificationReceipt, MutationRuntimeError>;

    fn commit(
        &mut self,
        plan: &ActionPlan,
        prepared: &PreparedMutation,
        verification: &VerificationReceipt,
    ) -> Result<CommitReceipt, MutationRuntimeError>;

    fn audit(
        &mut self,
        plan: &ActionPlan,
        decision: &PolicyDecision,
        authorization: &AuthorizationEvidence,
        execution: &ExecutionReceipt,
        verification: &VerificationReceipt,
        commit: &CommitReceipt,
    ) -> Result<AuditReceipt, MutationRuntimeError>;

    fn reconcile(
        &mut self,
        plan: &ActionPlan,
        commit: &CommitReceipt,
    ) -> Result<ReconciliationReceipt, MutationRuntimeError>;

    /// Append failure/denial/indeterminate evidence for an unsuccessful path.
    /// The original mutation error remains authoritative if failure auditing
    /// itself is unavailable; recovery must retry durable audit delivery.
    fn audit_failure(
        &mut self,
        request: &ActionRequest,
        stage: MutationStage,
        reason: &str,
    ) -> Result<(), MutationRuntimeError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationError {
    Unsupported {
        reason: String,
    },
    Provider {
        stage: MutationStage,
        error: ProviderError,
    },
    Invalid {
        stage: MutationStage,
        reason: String,
    },
    PolicyDenied {
        reason: String,
    },
    Runtime {
        stage: MutationStage,
        error: MutationRuntimeError,
    },
    Lifecycle(MutationTransitionError),
}

impl MutationError {
    #[must_use]
    pub const fn stage(&self) -> MutationStage {
        match self {
            Self::Unsupported { .. } => MutationStage::RequestIntent,
            Self::Provider { stage, .. }
            | Self::Invalid { stage, .. }
            | Self::Runtime { stage, .. } => *stage,
            Self::PolicyDenied { .. } => MutationStage::Authorize,
            Self::Lifecycle(error) => error.attempted,
        }
    }
}

impl Display for MutationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { reason } => write!(f, "unsupported mutation: {reason}"),
            Self::Provider { stage, error } => write!(f, "{} failed: {error}", stage.as_str()),
            Self::Invalid { stage, reason } => write!(f, "{} invalid: {reason}", stage.as_str()),
            Self::PolicyDenied { reason } => write!(f, "authorization denied: {reason}"),
            Self::Runtime { stage, error } => write!(f, "{} failed: {error}", stage.as_str()),
            Self::Lifecycle(error) => write!(
                f,
                "invalid lifecycle transition from {} to {}",
                error.current.as_str(),
                error.attempted.as_str()
            ),
        }
    }
}

impl std::error::Error for MutationError {}

/// Linura's local authority/control plane.
///
/// This component is deliberately independent of model providers. Agents and
/// other clients may request or propose work, but only this authority boundary
/// may turn validated requests into policy-evaluated plans and ordered managed
/// mutations.
pub struct ControlPlane<P> {
    policy: P,
    providers: Vec<Box<dyn Provider>>,
    graph: SystemGraph,
    intents: Vec<Intent>,
}

impl<P: Debug> Debug for ControlPlane<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlPlane")
            .field("policy", &self.policy)
            .field("provider_count", &self.providers.len())
            .field("intent_count", &self.intents.len())
            .field("graph_edge_count", &self.graph.edges().len())
            .finish()
    }
}

impl<P: PolicyEngine> ControlPlane<P> {
    #[must_use]
    pub fn new(policy: P) -> Self {
        Self {
            policy,
            providers: Vec::new(),
            graph: SystemGraph::default(),
            intents: Vec::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Box<dyn Provider>) {
        self.providers.push(provider);
    }

    #[must_use]
    pub fn graph(&self) -> &SystemGraph {
        &self.graph
    }

    #[must_use]
    pub fn intents(&self) -> &[Intent] {
        &self.intents
    }

    /// Read-only planning preview. Observation is intentionally performed before
    /// planning so providers cannot construct plans from assumed machine state.
    pub fn plan(
        &self,
        request: &ActionRequest,
    ) -> Result<(PlanResponse, Option<PolicyDecision>), ProviderError> {
        let Some(provider) = self.provider_for(request) else {
            return Ok((
                PlanResponse::Unsupported {
                    reason: "no provider supports capability".into(),
                },
                None,
            ));
        };

        let observation = provider.observe(&request.resource)?;
        validate_observation(provider, request, &observation)?;
        let plan = provider.plan(request, &observation)?;
        if let Err(error) = plan.validate() {
            return Ok((
                PlanResponse::Invalid {
                    reason: error.to_string(),
                },
                None,
            ));
        }

        let decision = self.policy.evaluate(&plan);
        Ok((PlanResponse::Planned(Box::new(plan)), Some(decision)))
    }

    /// Execute the canonical successful mutation lifecycle.
    ///
    /// This method is the orchestration contract for `0.0.0`. Concrete runtime
    /// implementations are deliberately injected through `MutationRuntime` so
    /// persistence, approvals, executors, verification, audit and reconciliation
    /// can mature independently without changing stage order.
    pub fn apply<R: MutationRuntime>(
        &self,
        request: &ActionRequest,
        runtime: &mut R,
    ) -> Result<MutationOutcome, MutationError> {
        let result = self.apply_inner(request, runtime);
        if let Err(error) = &result {
            let reason = error.to_string();
            let _ = runtime.audit_failure(request, error.stage(), &reason);
        }
        result
    }

    fn apply_inner<R: MutationRuntime>(
        &self,
        request: &ActionRequest,
        runtime: &mut R,
    ) -> Result<MutationOutcome, MutationError> {
        let mut progress = MutationProgress::new();
        let Some(provider) = self.provider_for(request) else {
            return Err(MutationError::Unsupported {
                reason: "no provider supports capability".into(),
            });
        };

        advance(&mut progress, MutationStage::Observe)?;
        let pre_observation =
            provider
                .observe(&request.resource)
                .map_err(|error| MutationError::Provider {
                    stage: MutationStage::Observe,
                    error,
                })?;
        validate_observation(provider, request, &pre_observation).map_err(|error| {
            MutationError::Provider {
                stage: MutationStage::Observe,
                error,
            }
        })?;

        advance(&mut progress, MutationStage::Plan)?;
        let plan =
            provider
                .plan(request, &pre_observation)
                .map_err(|error| MutationError::Provider {
                    stage: MutationStage::Plan,
                    error,
                })?;

        advance(&mut progress, MutationStage::Validate)?;
        plan.validate().map_err(|error| MutationError::Invalid {
            stage: MutationStage::Validate,
            reason: error.to_string(),
        })?;

        advance(&mut progress, MutationStage::Authorize)?;
        let policy_decision = self.policy.evaluate(&plan);
        if let PolicyDecision::Deny { reason } = &policy_decision {
            return Err(MutationError::PolicyDenied {
                reason: reason.clone(),
            });
        }
        let authorization = runtime
            .authorize(&plan, &policy_decision)
            .map_err(|error| MutationError::Runtime {
                stage: MutationStage::Authorize,
                error,
            })?;
        if !authorization.satisfies(&policy_decision) {
            return Err(MutationError::Invalid {
                stage: MutationStage::Authorize,
                reason: "authorization evidence does not satisfy the policy decision".into(),
            });
        }
        validate_authorization(&authorization)?;

        advance(&mut progress, MutationStage::Prepare)?;
        let prepared =
            runtime
                .prepare(&plan, &authorization)
                .map_err(|error| MutationError::Runtime {
                    stage: MutationStage::Prepare,
                    error,
                })?;
        validate_plan_ref(MutationStage::Prepare, &plan.id, &prepared.plan_id)?;
        validate_durable_ref(MutationStage::Prepare, &prepared.durable_ref)?;

        advance(&mut progress, MutationStage::Execute)?;
        let execution =
            runtime
                .execute(&plan, &prepared)
                .map_err(|error| MutationError::Runtime {
                    stage: MutationStage::Execute,
                    error,
                })?;
        validate_plan_ref(MutationStage::Execute, &plan.id, &execution.plan_id)?;
        validate_nonempty(
            MutationStage::Execute,
            "executor id",
            &execution.executor_id,
        )?;
        validate_nonempty(
            MutationStage::Execute,
            "execution summary",
            &execution.summary,
        )?;

        advance(&mut progress, MutationStage::Verify)?;
        let post_observation =
            provider
                .observe(&request.resource)
                .map_err(|error| MutationError::Provider {
                    stage: MutationStage::Verify,
                    error,
                })?;
        validate_observation(provider, request, &post_observation).map_err(|error| {
            MutationError::Provider {
                stage: MutationStage::Verify,
                error,
            }
        })?;
        let verification = runtime
            .verify(&plan, &execution, &post_observation)
            .map_err(|error| MutationError::Runtime {
                stage: MutationStage::Verify,
                error,
            })?;
        validate_plan_ref(MutationStage::Verify, &plan.id, &verification.plan_id)?;
        validate_nonempty(
            MutationStage::Verify,
            "verifier id",
            &verification.verifier_id,
        )?;
        validate_nonempty(
            MutationStage::Verify,
            "verification evidence",
            &verification.evidence,
        )?;

        advance(&mut progress, MutationStage::Commit)?;
        let commit = runtime
            .commit(&plan, &prepared, &verification)
            .map_err(|error| MutationError::Runtime {
                stage: MutationStage::Commit,
                error,
            })?;
        validate_plan_ref(MutationStage::Commit, &plan.id, &commit.plan_id)?;
        validate_durable_ref(MutationStage::Commit, &commit.durable_ref)?;

        advance(&mut progress, MutationStage::Audit)?;
        let audit = runtime
            .audit(
                &plan,
                &policy_decision,
                &authorization,
                &execution,
                &verification,
                &commit,
            )
            .map_err(|error| MutationError::Runtime {
                stage: MutationStage::Audit,
                error,
            })?;
        validate_plan_ref(MutationStage::Audit, &plan.id, &audit.plan_id)?;
        validate_durable_ref(MutationStage::Audit, &audit.durable_ref)?;

        advance(&mut progress, MutationStage::Reconcile)?;
        let reconciliation =
            runtime
                .reconcile(&plan, &commit)
                .map_err(|error| MutationError::Runtime {
                    stage: MutationStage::Reconcile,
                    error,
                })?;
        validate_plan_ref(MutationStage::Reconcile, &plan.id, &reconciliation.plan_id)?;
        validate_durable_ref(MutationStage::Reconcile, &reconciliation.durable_ref)?;

        Ok(MutationOutcome {
            plan,
            pre_observation,
            policy_decision,
            authorization,
            prepared,
            execution,
            post_observation,
            verification,
            commit,
            audit,
            reconciliation,
            completed_stages: progress.completed().to_vec(),
        })
    }

    fn provider_for(&self, request: &ActionRequest) -> Option<&dyn Provider> {
        self.providers
            .iter()
            .find(|provider| provider.supports(&request.capability))
            .map(Box::as_ref)
    }
}

fn advance(progress: &mut MutationProgress, stage: MutationStage) -> Result<(), MutationError> {
    progress.advance(stage).map_err(MutationError::Lifecycle)
}

fn validate_observation(
    provider: &dyn Provider,
    request: &ActionRequest,
    observation: &Observation,
) -> Result<(), ProviderError> {
    if observation.provider_id != provider.id() {
        return Err(ProviderError::InvalidState(
            "observation provider identity does not match selected provider".into(),
        ));
    }
    if observation.resource != request.resource {
        return Err(ProviderError::InvalidState(
            "observation resource does not match requested resource".into(),
        ));
    }
    Ok(())
}

fn validate_authorization(authorization: &AuthorizationEvidence) -> Result<(), MutationError> {
    if let AuthorizationEvidence::Approval {
        approver,
        mechanism,
        ..
    } = authorization
    {
        validate_nonempty(MutationStage::Authorize, "approver", approver)?;
        validate_nonempty(MutationStage::Authorize, "approval mechanism", mechanism)?;
    }
    Ok(())
}

fn validate_nonempty(
    stage: MutationStage,
    label: &'static str,
    value: &str,
) -> Result<(), MutationError> {
    if value.trim().is_empty() {
        Err(MutationError::Invalid {
            stage,
            reason: format!("{label} cannot be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_plan_ref(
    stage: MutationStage,
    expected: &PlanId,
    actual: &PlanId,
) -> Result<(), MutationError> {
    if expected == actual {
        Ok(())
    } else {
        Err(MutationError::Invalid {
            stage,
            reason: "stage receipt references a different plan".into(),
        })
    }
}

fn validate_durable_ref(stage: MutationStage, value: &str) -> Result<(), MutationError> {
    if value.trim().is_empty() {
        Err(MutationError::Invalid {
            stage,
            reason: "durable stage reference cannot be empty".into(),
        })
    } else {
        Ok(())
    }
}
