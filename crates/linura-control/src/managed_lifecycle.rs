use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use linura_core::{
    Actor, ActorKind, ApprovalRequestId, PrincipalId, ProviderId, RequestId, ResourceId,
};
use linura_lifecycle::{MutationProgress, MutationStage};
use linura_policy::{ApprovalClass, PolicyDecision};
use linura_protocol::PlanDesiredStateRequest;
use linura_provider_sdk::{
    ComponentDigest, EffectDescriptor, ExecutionBinding, ExecutionDisposition, ExecutionOutcome,
    VerificationDisposition, VerificationOutcome,
};
use linura_transaction::{TransactionId, TransactionState, TransactionStore};
use sha2::{Digest, Sha256};

use crate::approval_review::PolicyAuthenticatedApprover;
use crate::{
    AuthenticatedPrincipal, DispatchPermit, DurableAuthorityCandidate, DurableAuthorityControl,
    DurableAuthorityError, DurableRecoveryOutcome, PlanPreviewControl,
};

pub const MANAGED_SYSTEMD_UNIT_PREFIX: &str = "linura-managed-";
pub const MANAGED_SYSTEMD_OPERATION: &str = "set-active-state";
pub const MANAGED_SYSTEMD_PROVIDER: &str = "systemd";
pub const MANAGED_SYSTEMD_CAPABILITY: &str = "systemd.unit.observe";
const MANAGED_APPROVAL_TTL_SECONDS: u64 = 300;
const MANAGED_REQUEST_PREFIX: &str = "request:v06:";
const MANAGED_REQUEST_DIGEST_HEX_BYTES: usize = 64;
const MAX_OPERATION_ID_BYTES: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedHumanApproval {
    principal: PrincipalId,
}

impl TrustedHumanApproval {
    #[must_use]
    pub fn from_privileged_local_boundary(principal: PrincipalId) -> Self {
        Self { principal }
    }

    #[must_use]
    pub fn principal(&self) -> &PrincipalId {
        &self.principal
    }
}

#[derive(Debug)]
pub struct AuthorizedEffect {
    effect: EffectDescriptor,
    binding: ExecutionBinding,
    permit: DispatchPermit,
}

impl AuthorizedEffect {
    #[must_use]
    pub fn effect(&self) -> &EffectDescriptor {
        &self.effect
    }

    #[must_use]
    pub fn binding(&self) -> &ExecutionBinding {
        &self.binding
    }

    #[must_use]
    pub fn into_executor_request(self) -> (EffectDescriptor, ExecutionBinding) {
        let Self {
            effect,
            binding,
            permit: _permit,
        } = self;
        (effect, binding)
    }
}

pub trait AuthorizedEffectExecutor: Debug + Send {
    fn execute_authorized(
        &mut self,
        authorization: AuthorizedEffect,
    ) -> Result<ExecutionOutcome, String>;
}

pub trait IndependentManagedVerifier: Debug + Send {
    fn verify_effect(&mut self, effect: &EffectDescriptor) -> Result<VerificationOutcome, String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedMutationReceipt {
    pub transaction_id: String,
    pub plan_id: String,
    pub resource: String,
    pub desired_active_state: String,
    pub effect_digest: String,
    pub dispatch_digest: Option<String>,
    pub execution_disposition: Option<String>,
    pub verification_disposition: String,
    pub final_state: String,
    pub recovered: bool,
    pub stages: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedLifecycleError {
    UnsupportedEffect(String),
    InvalidRequestIdentity(String),
    ApprovalBoundary(String),
    Authority(String),
    Executor(String),
    ExecutionRejected(String),
    Verification(String),
    VerificationNotSatisfied(String),
    Indeterminate(String),
    TerminalState(String),
    Reconciliation(String),
    Contract(String),
}

impl Display for ManagedLifecycleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedEffect(detail) => write!(formatter, "unsupported v0.6 effect: {detail}"),
            Self::InvalidRequestIdentity(detail) => write!(formatter, "invalid v0.6 request identity: {detail}"),
            Self::ApprovalBoundary(detail) => write!(formatter, "trusted approval boundary failed: {detail}"),
            Self::Authority(detail) => write!(formatter, "durable authority failed: {detail}"),
            Self::Executor(detail) => write!(formatter, "privileged executor failed: {detail}"),
            Self::ExecutionRejected(detail) => write!(formatter, "effect was rejected before dispatch: {detail}"),
            Self::Verification(detail) => write!(formatter, "independent verification failed: {detail}"),
            Self::VerificationNotSatisfied(detail) => write!(formatter, "independent verification did not prove intended state: {detail}"),
            Self::Indeterminate(detail) => write!(formatter, "managed mutation remains indeterminate: {detail}"),
            Self::TerminalState(detail) => write!(formatter, "durable transaction is terminal and cannot be replayed: {detail}"),
            Self::Reconciliation(detail) => write!(formatter, "post-commit reconciliation failed: {detail}"),
            Self::Contract(detail) => write!(formatter, "managed lifecycle contract failed: {detail}"),
        }
    }
}

impl std::error::Error for ManagedLifecycleError {}

impl From<DurableAuthorityError> for ManagedLifecycleError {
    fn from(error: DurableAuthorityError) -> Self {
        Self::Authority(error.to_string())
    }
}

pub fn managed_request_id(
    operation_id: &str,
    request: &PlanDesiredStateRequest,
) -> Result<RequestId, ManagedLifecycleError> {
    validate_operation_id(operation_id)?;
    let digest = managed_request_digest(operation_id, request);
    RequestId::new(format!("{MANAGED_REQUEST_PREFIX}{operation_id}:{digest}"))
        .map_err(|error| ManagedLifecycleError::InvalidRequestIdentity(error.to_string()))
}

#[derive(Debug)]
pub struct ManagedLifecycleControl<S>
where
    S: TransactionStore,
{
    previews: PlanPreviewControl,
    authority: DurableAuthorityControl<S>,
}

impl<S> ManagedLifecycleControl<S>
where
    S: TransactionStore,
{
    pub fn new(
        previews: PlanPreviewControl,
        store: S,
        authority_signer: linura_transaction::TransactionAuthoritySigner,
    ) -> Result<Self, ManagedLifecycleError> {
        Ok(Self {
            previews,
            authority: DurableAuthorityControl::new(store, authority_signer)?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn converge_systemd_active_state<E, V>(
        &mut self,
        principal: AuthenticatedPrincipal,
        actor: Actor,
        request: PlanDesiredStateRequest,
        approval: &TrustedHumanApproval,
        executor: &mut E,
        verifier: &mut V,
    ) -> Result<ManagedMutationReceipt, ManagedLifecycleError>
    where
        E: AuthorizedEffectExecutor,
        V: IndependentManagedVerifier,
    {
        validate_public_request(&request)?;
        validate_request_identity(&request)?;
        if actor.kind != ActorKind::Human || !actor.interactive {
            return Err(ManagedLifecycleError::ApprovalBoundary(
                "v0.6 managed mutation requires an authenticated interactive human actor".into(),
            ));
        }

        let principal_id = PrincipalId::new(principal.as_str().to_owned())
            .map_err(|error| ManagedLifecycleError::Contract(error.to_string()))?;
        if approval.principal() != &principal_id {
            return Err(ManagedLifecycleError::ApprovalBoundary(
                "the approving administrator must be the authenticated mutation principal".into(),
            ));
        }
        let transaction_id = TransactionId::for_namespace(&principal_id, &request.request_id);
        let effect = effect_from_request(&request)?;

        match self.authority.snapshot(&transaction_id) {
            Ok(_) => {
                return self.resume_existing(
                    principal,
                    actor,
                    request,
                    approval,
                    effect,
                    transaction_id,
                    verifier,
                );
            }
            Err(DurableAuthorityError::Transaction(detail))
                if detail == "durable transaction not found" => {}
            Err(error) => return Err(error.into()),
        }

        let candidate = match self.authority.candidate(
            &mut self.previews,
            principal.clone(),
            actor.clone(),
            request.clone(),
        ) {
            Ok(candidate) => candidate,
            Err(DurableAuthorityError::CandidateNotMutation) => {
                return Err(ManagedLifecycleError::UnsupportedEffect(
                    "requested state already holds; no external effect is necessary".into(),
                ));
            }
            Err(error) => return Err(error.into()),
        };
        let canonical_effect = effect_from_candidate(&candidate)?;
        if canonical_effect != effect {
            return Err(ManagedLifecycleError::Contract(
                "trusted plan effect differs from the exact public request effect".into(),
            ));
        }
        let approval_evidence_id = self.authorize_candidate(&candidate, approval)?;
        let mut prepared = self.authority.prepare(candidate, approval_evidence_id)?;

        let mut progress = MutationProgress::new();
        advance(&mut progress, MutationStage::Observe)?;
        advance(&mut progress, MutationStage::Plan)?;
        advance(&mut progress, MutationStage::Validate)?;
        advance(&mut progress, MutationStage::Authorize)?;
        advance(&mut progress, MutationStage::Prepare)?;

        let plan_id = prepared.binding().plan_id().as_str().to_owned();
        let permit = self.authority.handoff(&principal, &mut prepared)?;
        let authorization = authorized_effect(effect.clone(), permit)?;
        let dispatch_digest = authorization.binding().dispatch_digest.to_hex();
        let execution = executor
            .execute_authorized(authorization)
            .map_err(ManagedLifecycleError::Executor)?;
        if execution.dispatch_digest.to_hex() != dispatch_digest {
            return Err(ManagedLifecycleError::Contract(
                "executor returned a dispatch digest different from the authorized handoff".into(),
            ));
        }
        match execution.disposition {
            ExecutionDisposition::RejectedBeforeDispatch => {
                return Err(ManagedLifecycleError::ExecutionRejected(execution.detail));
            }
            ExecutionDisposition::Dispatched | ExecutionDisposition::Indeterminate => {
                advance(&mut progress, MutationStage::Execute)?;
            }
        }

        let verification = verifier
            .verify_effect(&effect)
            .map_err(ManagedLifecycleError::Verification)?;
        match verification.disposition {
            VerificationDisposition::Satisfied => advance(&mut progress, MutationStage::Verify)?,
            VerificationDisposition::NotSatisfied => {
                return Err(ManagedLifecycleError::VerificationNotSatisfied(verification.detail));
            }
            VerificationDisposition::Inconclusive => {
                return Err(ManagedLifecycleError::Indeterminate(verification.detail));
            }
        }

        let verified = match self.authority.recover_indeterminate(
            &mut self.previews,
            principal.clone(),
            actor.clone(),
            request.clone(),
            None,
        )? {
            DurableRecoveryOutcome::Verified(verified) => verified,
            DurableRecoveryOutcome::Reprepared(_) => {
                return Err(ManagedLifecycleError::Indeterminate(
                    "state changed after independent verification; authority was reprepared and requires a new explicit invocation"
                        .into(),
                ));
            }
            DurableRecoveryOutcome::Blocked(snapshot) => {
                return Err(ManagedLifecycleError::TerminalState(format!(
                    "recovery blocked transaction {}",
                    snapshot.transaction_id.as_str()
                )));
            }
            DurableRecoveryOutcome::StillIndeterminate(snapshot) => {
                return Err(ManagedLifecycleError::Indeterminate(format!(
                    "transaction {} remains indeterminate after verification",
                    snapshot.transaction_id.as_str()
                )));
            }
        };
        let committed = self.authority.commit_verified(&principal, verified)?;
        advance(&mut progress, MutationStage::Commit)?;
        self.authority.integrity_check()?;
        advance(&mut progress, MutationStage::Audit)?;
        self.reconcile(&principal, &actor, &request, &effect, verifier)?;
        advance(&mut progress, MutationStage::Reconcile)?;

        Ok(receipt(
            &transaction_id,
            &plan_id,
            &effect,
            Some(&execution),
            &verification,
            &committed.state,
            false,
            &progress,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn resume_existing<V>(
        &mut self,
        principal: AuthenticatedPrincipal,
        actor: Actor,
        request: PlanDesiredStateRequest,
        approval: &TrustedHumanApproval,
        effect: EffectDescriptor,
        transaction_id: TransactionId,
        verifier: &mut V,
    ) -> Result<ManagedMutationReceipt, ManagedLifecycleError>
    where
        V: IndependentManagedVerifier,
    {
        let snapshot = self.authority.snapshot(&transaction_id)?;
        match snapshot.state {
            TransactionState::Prepared => Err(ManagedLifecycleError::Indeterminate(format!(
                "{} is still prepared after a failed pre-dispatch authority use; restart the trusted control composition root so restart recovery can retire it before retry",
                transaction_id.as_str()
            ))),
            TransactionState::Indeterminate => self.finish_indeterminate(
                principal,
                actor,
                request,
                approval,
                effect,
                transaction_id,
                snapshot.binding_digest.as_str().to_owned(),
                verifier,
            ),
            TransactionState::Verified => {
                let verification = verifier
                    .verify_effect(&effect)
                    .map_err(ManagedLifecycleError::Verification)?;
                if verification.disposition != VerificationDisposition::Satisfied {
                    return Err(ManagedLifecycleError::Indeterminate(format!(
                        "verified durable state is not independently re-proven: {}",
                        verification.detail
                    )));
                }
                let verified = self.authority.resume_verified(&principal, &transaction_id)?;
                let committed = self.authority.commit_verified(&principal, verified)?;
                let mut progress = progress_through(MutationStage::Commit)?;
                self.authority.integrity_check()?;
                advance(&mut progress, MutationStage::Audit)?;
                self.reconcile(&principal, &actor, &request, &effect, verifier)?;
                advance(&mut progress, MutationStage::Reconcile)?;
                Ok(receipt(
                    &transaction_id,
                    snapshot.binding_digest.as_str(),
                    &effect,
                    None,
                    &verification,
                    &committed.state,
                    true,
                    &progress,
                ))
            }
            TransactionState::Committed => {
                let verification = verifier
                    .verify_effect(&effect)
                    .map_err(ManagedLifecycleError::Verification)?;
                if verification.disposition != VerificationDisposition::Satisfied {
                    return Err(ManagedLifecycleError::Reconciliation(format!(
                        "committed state no longer satisfies its managed postcondition: {}. Use a new operation id to authorize a new convergence.",
                        verification.detail
                    )));
                }
                let mut progress = progress_through(MutationStage::Commit)?;
                self.authority.integrity_check()?;
                advance(&mut progress, MutationStage::Audit)?;
                self.reconcile(&principal, &actor, &request, &effect, verifier)?;
                advance(&mut progress, MutationStage::Reconcile)?;
                Ok(receipt(
                    &transaction_id,
                    snapshot.binding_digest.as_str(),
                    &effect,
                    None,
                    &verification,
                    &snapshot.state,
                    true,
                    &progress,
                ))
            }
            TransactionState::Aborted | TransactionState::RecoveryBlocked => {
                Err(ManagedLifecycleError::TerminalState(format!(
                    "{} is {}; use a new operation id after reviewing durable evidence",
                    transaction_id.as_str(),
                    snapshot.state.as_str()
                )))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_indeterminate<V>(
        &mut self,
        principal: AuthenticatedPrincipal,
        actor: Actor,
        request: PlanDesiredStateRequest,
        approval: &TrustedHumanApproval,
        effect: EffectDescriptor,
        transaction_id: TransactionId,
        plan_id: String,
        verifier: &mut V,
    ) -> Result<ManagedMutationReceipt, ManagedLifecycleError>
    where
        V: IndependentManagedVerifier,
    {
        let verification = verifier
            .verify_effect(&effect)
            .map_err(ManagedLifecycleError::Verification)?;

        if verification.disposition == VerificationDisposition::Inconclusive {
            return Err(ManagedLifecycleError::Indeterminate(verification.detail));
        }

        let approval_evidence_id = if verification.disposition == VerificationDisposition::NotSatisfied
        {
            let candidate = self.authority.candidate(
                &mut self.previews,
                principal.clone(),
                actor.clone(),
                request.clone(),
            )?;
            let canonical_effect = effect_from_candidate(&candidate)?;
            if canonical_effect != effect {
                return Err(ManagedLifecycleError::Contract(
                    "recovery candidate substituted the originally bound effect".into(),
                ));
            }
            self.authorize_candidate(&candidate, approval)?
        } else {
            None
        };

        let recovery = self.authority.recover_indeterminate(
            &mut self.previews,
            principal.clone(),
            actor.clone(),
            request.clone(),
            approval_evidence_id,
        )?;

        match recovery {
            DurableRecoveryOutcome::Verified(verified) => {
                if verification.disposition != VerificationDisposition::Satisfied {
                    return Err(ManagedLifecycleError::Indeterminate(
                        "fresh Control observation reached intended state after the independent verifier did not; durable state is verified but commit is withheld until a later independent re-verification"
                            .into(),
                    ));
                }
                let committed = self.authority.commit_verified(&principal, verified)?;
                let mut progress = progress_through(MutationStage::Commit)?;
                self.authority.integrity_check()?;
                advance(&mut progress, MutationStage::Audit)?;
                self.reconcile(&principal, &actor, &request, &effect, verifier)?;
                advance(&mut progress, MutationStage::Reconcile)?;
                Ok(receipt(
                    &transaction_id,
                    &plan_id,
                    &effect,
                    None,
                    &verification,
                    &committed.state,
                    true,
                    &progress,
                ))
            }
            DurableRecoveryOutcome::Reprepared(_) => Err(ManagedLifecycleError::Indeterminate(
                "independent verification proves the intended effect absent; durable authority is safely reprepared but execution requires a new explicit invocation"
                    .into(),
            )),
            DurableRecoveryOutcome::Blocked(snapshot) => {
                Err(ManagedLifecycleError::TerminalState(format!(
                    "recovery found conflicting state and blocked {}",
                    snapshot.transaction_id.as_str()
                )))
            }
            DurableRecoveryOutcome::StillIndeterminate(snapshot) => {
                Err(ManagedLifecycleError::Indeterminate(format!(
                    "{} remains indeterminate; no replay was attempted",
                    snapshot.transaction_id.as_str()
                )))
            }
        }
    }

    fn authorize_candidate(
        &mut self,
        candidate: &DurableAuthorityCandidate,
        approval: &TrustedHumanApproval,
    ) -> Result<Option<linura_core::ApprovalEvidenceId>, ManagedLifecycleError> {
        match candidate.review().decision() {
            PolicyDecision::Allow => Ok(None),
            PolicyDecision::RequireApproval { class, .. } => {
                if *class != ApprovalClass::Administrator {
                    return Err(ManagedLifecycleError::ApprovalBoundary(format!(
                        "v0.6 narrow effect requires unsupported approval class {class:?}"
                    )));
                }
                let approver = PolicyAuthenticatedApprover::new(
                    approval.principal().clone(),
                    ActorKind::Human,
                    BTreeSet::from([*class]),
                );
                let approval_request = ApprovalRequestId::new(format!(
                    "approval:v06:{}",
                    candidate.plan_id().as_str()
                ))
                .map_err(|error| ManagedLifecycleError::Contract(error.to_string()))?;
                let expires_at = now_unix_seconds()?
                    .checked_add(MANAGED_APPROVAL_TTL_SECONDS)
                    .ok_or_else(|| ManagedLifecycleError::Contract("approval clock overflow".into()))?;
                let evidence = self.authority.issue_approval(
                    approval_request,
                    candidate,
                    &approver,
                    expires_at,
                )?;
                Ok(Some(evidence.id().clone()))
            }
            PolicyDecision::Deny { reason } | PolicyDecision::Blocked { reason } => {
                Err(ManagedLifecycleError::ApprovalBoundary(reason.clone()))
            }
        }
    }

    fn reconcile<V>(
        &mut self,
        principal: &AuthenticatedPrincipal,
        actor: &Actor,
        request: &PlanDesiredStateRequest,
        effect: &EffectDescriptor,
        verifier: &mut V,
    ) -> Result<(), ManagedLifecycleError>
    where
        V: IndependentManagedVerifier,
    {
        let verification = verifier
            .verify_effect(effect)
            .map_err(ManagedLifecycleError::Verification)?;
        if verification.disposition != VerificationDisposition::Satisfied {
            return Err(ManagedLifecycleError::Reconciliation(verification.detail));
        }
        let (plan, observation) = self
            .previews
            .authority_candidate(principal.clone(), actor.clone(), request.clone())
            .map_err(|error| ManagedLifecycleError::Reconciliation(error.to_string()))?;
        observation
            .validate(&plan.provider, &plan.resource, &plan.observation_capability)
            .map_err(|error| ManagedLifecycleError::Reconciliation(error.to_string()))?;
        if plan.status != linura_planner::PlanStatus::NoChange || !plan.changes.is_empty() {
            return Err(ManagedLifecycleError::Reconciliation(
                "fresh authoritative planning does not converge to no-change".into(),
            ));
        }
        Ok(())
    }
}

fn validate_public_request(request: &PlanDesiredStateRequest) -> Result<(), ManagedLifecycleError> {
    let _ = effect_from_request(request)?;
    Ok(())
}

fn effect_from_request(
    request: &PlanDesiredStateRequest,
) -> Result<EffectDescriptor, ManagedLifecycleError> {
    if request.provider.as_str() != MANAGED_SYSTEMD_PROVIDER
        || request.observation_capability.as_str() != MANAGED_SYSTEMD_CAPABILITY
    {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "only the authoritative systemd unit observation route is supported".into(),
        ));
    }
    effect_from_parts(
        request.provider.clone(),
        request.resource.clone(),
        &request.desired_state,
    )
}

fn effect_from_candidate(
    candidate: &DurableAuthorityCandidate,
) -> Result<EffectDescriptor, ManagedLifecycleError> {
    let subject = candidate.review().subject();
    if subject.provider().as_str() != MANAGED_SYSTEMD_PROVIDER
        || subject.capability().as_str() != MANAGED_SYSTEMD_CAPABILITY
        || subject.changes().len() != 1
    {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "trusted plan is outside the single systemd active-state capability".into(),
        ));
    }
    let change = &subject.changes()[0];
    if change.key != "active_state" {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "trusted plan contains a mutation other than active_state".into(),
        ));
    }
    let desired = BTreeMap::from([(change.key.clone(), change.desired.clone())]);
    effect_from_parts(
        subject.provider().clone(),
        subject.resource().clone(),
        &desired,
    )
}

fn effect_from_parts(
    provider: ProviderId,
    resource: ResourceId,
    desired_state: &BTreeMap<String, String>,
) -> Result<EffectDescriptor, ManagedLifecycleError> {
    let unit = managed_unit_from_resource(resource.as_str())?;
    if desired_state.len() != 1 {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "v0.6 accepts exactly one desired-state attribute".into(),
        ));
    }
    let desired = desired_state
        .get("active_state")
        .ok_or_else(|| ManagedLifecycleError::UnsupportedEffect("active_state is required".into()))?;
    if !matches!(desired.as_str(), "active" | "inactive") {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "active_state must be exactly active or inactive".into(),
        ));
    }
    EffectDescriptor::new(
        provider,
        resource,
        MANAGED_SYSTEMD_OPERATION,
        format!("unit={unit}\nactive_state={desired}\n").into_bytes(),
    )
    .map_err(|error| ManagedLifecycleError::Contract(error.to_string()))
}

fn managed_unit_from_resource(resource: &str) -> Result<&str, ManagedLifecycleError> {
    let unit = resource
        .strip_prefix("systemd:unit:")
        .ok_or_else(|| ManagedLifecycleError::UnsupportedEffect("resource is not a systemd unit".into()))?;
    if unit.is_empty() || unit.len() > 255 || !unit.ends_with(".service") || !unit.is_ascii() {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "unit must be a bounded ASCII .service name".into(),
        ));
    }
    let suffix = unit
        .strip_prefix(MANAGED_SYSTEMD_UNIT_PREFIX)
        .ok_or_else(|| ManagedLifecycleError::UnsupportedEffect(
            "unit is outside the linura-managed- namespace".into(),
        ))?;
    let slug = suffix.strip_suffix(".service").ok_or_else(|| {
        ManagedLifecycleError::UnsupportedEffect("managed unit suffix is not canonical".into())
    })?;
    if slug.is_empty()
        || slug.len() > 96
        || !slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        || slug.starts_with('-')
        || slug.ends_with('-')
        || slug.contains("--")
    {
        return Err(ManagedLifecycleError::UnsupportedEffect(
            "managed unit name is not canonical".into(),
        ));
    }
    Ok(unit)
}

fn authorized_effect(
    effect: EffectDescriptor,
    permit: DispatchPermit,
) -> Result<AuthorizedEffect, ManagedLifecycleError> {
    let binding = ExecutionBinding::new(
        permit.transaction_id().as_str(),
        permit.generation(),
        permit.state_version(),
        ComponentDigest::parse_hex(permit.binding_digest().hex())
            .map_err(|error| ManagedLifecycleError::Contract(error.to_string()))?,
        ComponentDigest::parse_hex(permit.authority_use_digest().hex())
            .map_err(|error| ManagedLifecycleError::Contract(error.to_string()))?,
        &effect,
    )
    .map_err(|error| ManagedLifecycleError::Contract(error.to_string()))?;
    Ok(AuthorizedEffect {
        effect,
        binding,
        permit,
    })
}

fn validate_operation_id(operation_id: &str) -> Result<(), ManagedLifecycleError> {
    if operation_id.is_empty()
        || operation_id.len() > MAX_OPERATION_ID_BYTES
        || !operation_id.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        || operation_id.starts_with('-')
        || operation_id.ends_with('-')
        || operation_id.contains("--")
    {
        return Err(ManagedLifecycleError::InvalidRequestIdentity(
            "operation id must be 1..64 lowercase ASCII letters/digits/hyphens in canonical form"
                .into(),
        ));
    }
    Ok(())
}

fn validate_request_identity(request: &PlanDesiredStateRequest) -> Result<(), ManagedLifecycleError> {
    let value = request.request_id.as_str();
    let rest = value.strip_prefix(MANAGED_REQUEST_PREFIX).ok_or_else(|| {
        ManagedLifecycleError::InvalidRequestIdentity(format!(
            "request id must begin with {MANAGED_REQUEST_PREFIX}"
        ))
    })?;
    let (operation_id, supplied_digest) = rest.rsplit_once(':').ok_or_else(|| {
        ManagedLifecycleError::InvalidRequestIdentity(
            "request id must contain an operation id and body digest".into(),
        )
    })?;
    validate_operation_id(operation_id)?;
    if supplied_digest.len() != MANAGED_REQUEST_DIGEST_HEX_BYTES
        || !supplied_digest.as_bytes().iter().all(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
        })
    {
        return Err(ManagedLifecycleError::InvalidRequestIdentity(
            "request body digest must be 64 lowercase hexadecimal digits".into(),
        ));
    }
    let expected = managed_request_digest(operation_id, request);
    if supplied_digest != expected {
        return Err(ManagedLifecycleError::InvalidRequestIdentity(
            "request id does not bind the exact provider/resource/capability/reason/desired-state body"
                .into(),
        ));
    }
    Ok(())
}

fn managed_request_digest(operation_id: &str, request: &PlanDesiredStateRequest) -> String {
    let mut hasher = Sha256::new();
    put_digest_field(&mut hasher, b"linura.managed-request.v0.6.v1");
    put_digest_field(&mut hasher, operation_id.as_bytes());
    put_digest_field(&mut hasher, request.provider.as_str().as_bytes());
    put_digest_field(&mut hasher, request.resource.as_str().as_bytes());
    put_digest_field(
        &mut hasher,
        request.observation_capability.as_str().as_bytes(),
    );
    put_digest_field(&mut hasher, request.reason.summary.as_bytes());
    for id in &request.reason.intent_ids {
        put_digest_field(&mut hasher, b"intent");
        put_digest_field(&mut hasher, id.as_str().as_bytes());
    }
    for id in &request.reason.requirement_ids {
        put_digest_field(&mut hasher, b"requirement");
        put_digest_field(&mut hasher, id.as_str().as_bytes());
    }
    for id in &request.reason.capability_ids {
        put_digest_field(&mut hasher, b"capability");
        put_digest_field(&mut hasher, id.as_str().as_bytes());
    }
    for (key, value) in &request.desired_state {
        put_digest_field(&mut hasher, key.as_bytes());
        put_digest_field(&mut hasher, value.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn put_digest_field(hasher: &mut Sha256, value: &[u8]) {
    let len = u64::try_from(value.len()).unwrap_or(u64::MAX);
    hasher.update(len.to_be_bytes());
    hasher.update(value);
}

fn now_unix_seconds() -> Result<u64, ManagedLifecycleError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| ManagedLifecycleError::Contract("system clock is before unix epoch".into()))
}

fn advance(
    progress: &mut MutationProgress,
    stage: MutationStage,
) -> Result<(), ManagedLifecycleError> {
    progress
        .advance(stage)
        .map_err(|error| ManagedLifecycleError::Contract(format!("invalid lifecycle transition: {error:?}")))
}

fn progress_through(last: MutationStage) -> Result<MutationProgress, ManagedLifecycleError> {
    let mut progress = MutationProgress::new();
    for stage in linura_lifecycle::MUTATION_STAGES.iter().copied().skip(1) {
        advance(&mut progress, stage)?;
        if stage == last {
            break;
        }
    }
    Ok(progress)
}

fn receipt(
    transaction_id: &TransactionId,
    plan_id: &str,
    effect: &EffectDescriptor,
    execution: Option<&ExecutionOutcome>,
    verification: &VerificationOutcome,
    final_state: &TransactionState,
    recovered: bool,
    progress: &MutationProgress,
) -> ManagedMutationReceipt {
    let desired_active_state = String::from_utf8_lossy(&effect.canonical_payload)
        .lines()
        .find_map(|line| line.strip_prefix("active_state="))
        .unwrap_or("unknown")
        .to_owned();
    ManagedMutationReceipt {
        transaction_id: transaction_id.as_str().to_owned(),
        plan_id: plan_id.to_owned(),
        resource: effect.resource.as_str().to_owned(),
        desired_active_state,
        effect_digest: effect.digest().to_hex(),
        dispatch_digest: execution.map(|value| value.dispatch_digest.to_hex()),
        execution_disposition: execution.map(|value| execution_name(value.disposition).to_owned()),
        verification_disposition: verification_name(verification.disposition).to_owned(),
        final_state: final_state.as_str().to_owned(),
        recovered,
        stages: progress
            .completed()
            .iter()
            .map(|stage| stage.as_str().to_owned())
            .collect(),
    }
}

const fn execution_name(disposition: ExecutionDisposition) -> &'static str {
    match disposition {
        ExecutionDisposition::RejectedBeforeDispatch => "rejected-before-dispatch",
        ExecutionDisposition::Dispatched => "dispatched",
        ExecutionDisposition::Indeterminate => "indeterminate",
    }
}

const fn verification_name(disposition: VerificationDisposition) -> &'static str {
    match disposition {
        VerificationDisposition::Satisfied => "satisfied",
        VerificationDisposition::NotSatisfied => "not-satisfied",
        VerificationDisposition::Inconclusive => "inconclusive",
    }
}

#[cfg(test)]
mod tests {
    use linura_core::{CapabilityId, SemanticReason};

    use super::*;

    fn request(resource: &str, desired: &str) -> PlanDesiredStateRequest {
        let mut request = PlanDesiredStateRequest {
            request_id: RequestId::new("request:v06:placeholder")
                .unwrap_or_else(|error| unreachable!("{error}")),
            provider: ProviderId::new("systemd")
                .unwrap_or_else(|error| unreachable!("{error}")),
            resource: ResourceId::new(resource)
                .unwrap_or_else(|error| unreachable!("{error}")),
            observation_capability: CapabilityId::new("systemd.unit.observe")
                .unwrap_or_else(|error| unreachable!("{error}")),
            reason: SemanticReason {
                summary: "v0.6 test".into(),
                intent_ids: vec![],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
            desired_state: BTreeMap::from([("active_state".into(), desired.into())]),
        };
        request.request_id = managed_request_id("test-operation", &request)
            .unwrap_or_else(|error| unreachable!("{error}"));
        request
    }

    #[test]
    fn public_effect_is_exactly_one_reserved_systemd_active_state() {
        let effect = effect_from_request(&request(
            "systemd:unit:linura-managed-example.service",
            "active",
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(effect.operation, MANAGED_SYSTEMD_OPERATION);
        assert_eq!(
            effect.canonical_payload,
            b"unit=linura-managed-example.service\nactive_state=active\n"
        );
        assert!(effect_from_request(&request("systemd:unit:sshd.service", "active")).is_err());
        assert!(effect_from_request(&request(
            "systemd:unit:linura-managed-example.service",
            "failed",
        ))
        .is_err());
    }

    #[test]
    fn request_id_binds_operation_and_exact_body() {
        let request = request(
            "systemd:unit:linura-managed-example.service",
            "active",
        );
        assert!(validate_request_identity(&request).is_ok());

        let mut substituted = request.clone();
        substituted
            .desired_state
            .insert("active_state".into(), "inactive".into());
        assert!(validate_request_identity(&substituted).is_err());

        let mut other_operation = request.clone();
        other_operation.request_id = managed_request_id("other-operation", &other_operation)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_ne!(request.request_id, other_operation.request_id);
    }

    #[test]
    fn lifecycle_progress_helper_never_skips_canonical_order() {
        let progress = progress_through(MutationStage::Reconcile)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert!(progress.is_complete());
        assert_eq!(progress.completed(), linura_lifecycle::MUTATION_STAGES.as_slice());
    }
}
