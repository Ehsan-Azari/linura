use linura_core::{
    Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass, SemanticReason, ValidationError,
};
use linura_policy::{
    ApprovalClass, BaselinePolicy, PolicyDecision, PolicyEngine, PolicySubject, ReviewPlanStatus,
    ReviewedChange,
};

fn id<T>(result: Result<T, ValidationError>) -> T {
    result.unwrap_or_else(|error| unreachable!("{error}"))
}

fn subject(kind: ActorKind, risk: RiskClass) -> PolicySubject {
    PolicySubject::try_new(
        id(PrincipalId::new("uid:1000")),
        id(PlanId::new("plan:approval-strength")),
        id(RequestId::new("request:approval-strength")),
        Actor {
            id: id(ActorId::new("actor:approval-strength")),
            kind,
            interactive: kind == ActorKind::Human,
        },
        id(ProviderId::new("systemd")),
        id(ResourceId::new("systemd:unit:test.service")),
        id(CapabilityId::new("systemd.unit.observe")),
        SemanticReason {
            summary: "exercise approval-strength contract".into(),
            intent_ids: vec![id(IntentId::new("intent:approval-strength"))],
            requirement_ids: vec![],
            capability_ids: vec![],
        },
        "evidence:approval-strength".into(),
        risk,
        ReviewPlanStatus::ChangeProposed,
        vec![ReviewedChange {
            key: "active_state".into(),
            current: Some("inactive".into()),
            desired: "active".into(),
        }],
        vec![],
    )
    .unwrap_or_else(|error| unreachable!("{error}"))
}

#[test]
fn protected_approval_strength_is_actor_invariant() {
    let policy = BaselinePolicy::default();
    let local_actor_kinds = [ActorKind::Human, ActorKind::Service, ActorKind::Agent];
    let protected_risks = [
        (RiskClass::SystemMutation, ApprovalClass::InteractiveUser),
        (RiskClass::SecuritySensitive, ApprovalClass::Administrator),
        (RiskClass::Destructive, ApprovalClass::DestructiveAction),
    ];

    for actor_kind in local_actor_kinds {
        for (risk, expected_class) in protected_risks {
            let evaluation = policy.evaluate(&subject(actor_kind, risk));
            match evaluation.decision {
                PolicyDecision::RequireApproval { class, .. } => {
                    assert_eq!(
                        class, expected_class,
                        "actor {actor_kind:?} with risk {risk:?} weakened or changed approval class"
                    );
                }
                decision => panic!(
                    "actor {actor_kind:?} with risk {risk:?} must require {expected_class:?}, got {decision:?}"
                ),
            }
        }
    }
}

#[test]
fn remote_actor_cannot_use_protected_approval_path() {
    let policy = BaselinePolicy::default();
    for risk in [
        RiskClass::SystemMutation,
        RiskClass::SecuritySensitive,
        RiskClass::Destructive,
    ] {
        let evaluation = policy.evaluate(&subject(ActorKind::Remote, risk));
        assert!(
            matches!(evaluation.decision, PolicyDecision::Deny { .. }),
            "remote actor with risk {risk:?} must remain denied"
        );
    }
}
