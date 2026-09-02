use linura_core::{PrincipalId, ValidationError};
use linura_planner::{PlanFindingLevel, PlanStatus, ReconciliationPlan};
use linura_policy::{
    PolicySubject, ReviewFindingLevel, ReviewPlanStatus, ReviewedChange, ReviewedFinding,
};

use crate::AuthenticatedPrincipal;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicySubjectError {
    InvalidPrincipal(String),
}

/// Derive the internal policy-review subject from Linura's canonical
/// reconciliation plan and the already-authenticated control principal.
///
/// This is intentionally owned by `linura-control`: transports authenticate,
/// the planner plans, policy evaluates, and Control binds those boundaries.
pub fn policy_subject_from_plan(
    principal: &AuthenticatedPrincipal,
    plan: &ReconciliationPlan,
) -> Result<PolicySubject, PolicySubjectError> {
    let principal = PrincipalId::new(principal.as_str().to_owned()).map_err(map_principal_error)?;

    Ok(PolicySubject::new(
        principal,
        plan.id.clone(),
        plan.request_id.clone(),
        plan.actor.clone(),
        plan.provider.clone(),
        plan.resource.clone(),
        plan.observation_capability.clone(),
        plan.reason.clone(),
        plan.observed_evidence_id.clone(),
        plan.prospective_risk,
        map_status(plan.status),
        plan.changes
            .iter()
            .map(|change| ReviewedChange {
                key: change.key.clone(),
                current: change.current.clone(),
                desired: change.desired.clone(),
            })
            .collect(),
        plan.findings
            .iter()
            .map(|finding| ReviewedFinding {
                code: finding.code.clone(),
                level: map_finding_level(finding.level),
                message: finding.message.clone(),
            })
            .collect(),
    ))
}

fn map_principal_error(error: ValidationError) -> PolicySubjectError {
    PolicySubjectError::InvalidPrincipal(error.to_string())
}

const fn map_status(status: PlanStatus) -> ReviewPlanStatus {
    match status {
        PlanStatus::NoChange => ReviewPlanStatus::NoChange,
        PlanStatus::ChangeProposed => ReviewPlanStatus::ChangeProposed,
        PlanStatus::Blocked => ReviewPlanStatus::Blocked,
    }
}

const fn map_finding_level(level: PlanFindingLevel) -> ReviewFindingLevel {
    match level {
        PlanFindingLevel::Pass => ReviewFindingLevel::Pass,
        PlanFindingLevel::Warning => ReviewFindingLevel::Warning,
        PlanFindingLevel::Blocker => ReviewFindingLevel::Blocker,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use linura_core::{Actor, ActorId, ActorKind, IntentId, RequestId, SemanticReason};
    use linura_planner::{DesiredResource, DeterministicPlanner, PlanningFreshness, PlanningObservation};

    use super::*;

    #[test]
    fn canonical_plan_projects_exact_review_material() {
        let request_id = RequestId::new("request:review")
            .unwrap_or_else(|error| unreachable!("{error}"));
        let actor = Actor {
            id: ActorId::new("actor:human").unwrap_or_else(|error| unreachable!("{error}")),
            kind: ActorKind::Human,
            interactive: true,
        };
        let provider = linura_core::ProviderId::new("systemd")
            .unwrap_or_else(|error| unreachable!("{error}"));
        let resource = linura_core::ResourceId::new("systemd:unit:test.service")
            .unwrap_or_else(|error| unreachable!("{error}"));
        let capability = linura_core::CapabilityId::new("systemd.unit.observe")
            .unwrap_or_else(|error| unreachable!("{error}"));
        let desired = DesiredResource {
            provider: provider.clone(),
            resource: resource.clone(),
            observation_capability: capability.clone(),
            state: BTreeMap::from([("active_state".into(), "active".into())]),
            reason: SemanticReason {
                summary: "keep test active".into(),
                intent_ids: vec![IntentId::new("intent:test")
                    .unwrap_or_else(|error| unreachable!("{error}"))],
                requirement_ids: vec![],
                capability_ids: vec![],
            },
        };
        let observation = PlanningObservation {
            provider,
            resource,
            observation_capability: capability,
            authority: "authoritative".into(),
            evidence_id: "evidence:review".into(),
            freshness: PlanningFreshness::Current,
            attributes: BTreeMap::from([("active_state".into(), "inactive".into())]),
        };
        let plan = DeterministicPlanner
            .plan_resource(request_id, actor, desired, &observation)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let principal = AuthenticatedPrincipal::new("uid:1000")
            .unwrap_or_else(|error| unreachable!("{error}"));

        let subject = policy_subject_from_plan(&principal, &plan)
            .unwrap_or_else(|error| unreachable!("{error:?}"));

        assert_eq!(subject.principal().as_str(), "uid:1000");
        assert_eq!(subject.plan_id(), &plan.id);
        assert_eq!(subject.observed_evidence_id(), "evidence:review");
        assert_eq!(subject.reason(), &plan.reason);
        assert_eq!(subject.changes().len(), plan.changes.len());
        assert_eq!(subject.findings().len(), plan.findings.len());
        assert_eq!(subject.status(), ReviewPlanStatus::ChangeProposed);
    }
}
