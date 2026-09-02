#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/linura-dbus/src/planning.rs")
text = path.read_text(encoding="utf-8")


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one occurrence, found {count}: {old[:140]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "pub(crate) fn plan_review_from_wire(wire: PlanReviewWire) -> Result<PlanReview, String> {\n",
    """fn required_approval_class_for_reviewed_risk(
    risk: RiskClass,
) -> Option<PlanReviewApprovalClass> {
    match risk {
        RiskClass::ReadOnly | RiskClass::UserState => None,
        RiskClass::SystemMutation => Some(PlanReviewApprovalClass::InteractiveUser),
        RiskClass::SecuritySensitive => Some(PlanReviewApprovalClass::Administrator),
        RiskClass::Destructive => Some(PlanReviewApprovalClass::DestructiveAction),
    }
}

pub(crate) fn plan_review_from_wire(wire: PlanReviewWire) -> Result<PlanReview, String> {
""",
)

replace_once(
    """    match decision {
        PlanReviewDecision::RequireApproval => {
            if approval_class.is_none() || decision_reason.trim().is_empty() {
                return Err(\"require-approval review lacks class or reason\".into());
            }
        }
        PlanReviewDecision::Deny | PlanReviewDecision::Blocked => {
            if approval_class.is_some() || decision_reason.trim().is_empty() {
                return Err(\"deny/blocked review has inconsistent approval metadata\".into());
            }
        }
        PlanReviewDecision::Allow => {
            if approval_class.is_some() {
                return Err(\"allow review unexpectedly carries an approval class\".into());
            }
        }
    }

    let planner_risk_floor = parse_risk(&planner_risk_floor)?;
    let reviewed_risk = parse_risk(&reviewed_risk)?;
""",
    """    let planner_risk_floor = parse_risk(&planner_risk_floor)?;
    let reviewed_risk = parse_risk(&reviewed_risk)?;
    let required_approval_class = required_approval_class_for_reviewed_risk(reviewed_risk);
    match decision {
        PlanReviewDecision::RequireApproval => {
            if approval_class != required_approval_class || decision_reason.trim().is_empty() {
                return Err(
                    \"require-approval review class does not match the reviewed risk\".into(),
                );
            }
            if required_approval_class.is_none() {
                return Err(\"require-approval review uses an unprotected risk class\".into());
            }
        }
        PlanReviewDecision::Deny | PlanReviewDecision::Blocked => {
            if approval_class.is_some() || decision_reason.trim().is_empty() {
                return Err(\"deny/blocked review has inconsistent approval metadata\".into());
            }
        }
        PlanReviewDecision::Allow => {
            if approval_class.is_some() || required_approval_class.is_some() {
                return Err(\"allow review conflicts with the reviewed risk approval floor\".into());
            }
        }
    }
""",
)

insert_before = """    #[test]
    fn wire_decoder_rejects_inconsistent_status_and_execution_authority() {
"""
new_tests = """    fn protected_review_wire(
        reviewed_risk: &str,
        approval_class: &str,
    ) -> PlanReviewWire {
        let preview_wire = plan_preview_wire(&preview());
        (
            preview_wire.0,
            \"uid:1000\".into(),
            preview_wire.1,
            preview_wire.2,
            preview_wire.3,
            preview_wire.4,
            (
                \"system-mutation\".into(),
                reviewed_risk.into(),
                preview_wire.6,
                \"policy:baseline\".into(),
                \"policy:baseline:v1\".into(),
            ),
            (
                \"require-approval\".into(),
                true,
                approval_class.into(),
                \"trusted policy requires approval\".into(),
                false,
            ),
            preview_wire.8,
            preview_wire.9,
        )
    }

    #[test]
    fn review_wire_decoder_enforces_approval_strength_from_reviewed_risk() {
        assert!(plan_review_from_wire(protected_review_wire(
            \"system-mutation\",
            \"interactive-user\"
        ))
        .is_ok());
        assert!(plan_review_from_wire(protected_review_wire(
            \"security-sensitive\",
            \"administrator\"
        ))
        .is_ok());
        assert!(plan_review_from_wire(protected_review_wire(
            \"destructive\",
            \"destructive-action\"
        ))
        .is_ok());

        assert!(plan_review_from_wire(protected_review_wire(
            \"security-sensitive\",
            \"interactive-user\"
        ))
        .is_err());
        assert!(plan_review_from_wire(protected_review_wire(
            \"destructive\",
            \"administrator\"
        ))
        .is_err());
    }

"""
if text.count(insert_before) != 1:
    raise SystemExit("wire decoder test marker mismatch")
text = text.replace(insert_before, new_tests + insert_before, 1)

path.write_text(text, encoding="utf-8")
