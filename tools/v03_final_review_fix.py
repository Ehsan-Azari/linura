#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one marker, found {count}: {old[:120]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


approval = "crates/linura-control/src/approval_review.rs"
replace_once(
    approval,
    "use std::fmt::Debug;\nuse std::time::{SystemTime, UNIX_EPOCH};\n",
    "use std::fmt::Debug;\nuse std::sync::atomic::{AtomicU64 as AuthorityAtomicU64, Ordering as AuthorityOrdering};\nuse std::time::{SystemTime, UNIX_EPOCH};\n",
)
replace_once(
    approval,
    "    RevokerNotAuthorized,\n    ClockUnavailable,\n}",
    "    RevokerNotAuthorized,\n    ClockUnavailable,\n    ClockRollback,\n}",
)
replace_once(
    approval,
    "pub struct ApprovalReviewControl {\n    clock: Box<dyn ApprovalClock>,\n    records: BTreeMap<ApprovalEvidenceId, ApprovalRecord>,",
    "pub struct ApprovalReviewControl {\n    clock: Box<dyn ApprovalClock>,\n    last_authority_unix_seconds: AuthorityAtomicU64,\n    records: BTreeMap<ApprovalEvidenceId, ApprovalRecord>,",
)
replace_once(
    approval,
    "        Self {\n            clock,\n            records: BTreeMap::new(),",
    "        Self {\n            clock,\n            last_authority_unix_seconds: AuthorityAtomicU64::new(0),\n            records: BTreeMap::new(),",
)
replace_once(
    approval,
    "    fn authority_now_unix_seconds(&self) -> Result<u64, ApprovalControlError> {\n        self.clock.now_unix_seconds()\n    }",
    "    fn authority_now_unix_seconds(&self) -> Result<u64, ApprovalControlError> {\n        let sampled = self.clock.now_unix_seconds()?;\n        let previous = self\n            .last_authority_unix_seconds\n            .fetch_max(sampled, AuthorityOrdering::SeqCst);\n        if sampled < previous {\n            return Err(ApprovalControlError::ClockRollback);\n        }\n        Ok(sampled)\n    }",
)
clock_test_marker = """    #[test]\n    fn retained_bytes_are_bounded_and_idempotent_retry_does_not_grow_store() {\n"""
clock_test = """    #[test]
    fn authority_clock_rollback_cannot_revive_expired_evidence() {
        let review = review(RiskClass::SecuritySensitive);
        let request = id(ApprovalRequestId::new("approval:request:clock-rollback"));
        let (mut control, clock) = control_with_clock(100);
        let evidence = control
            .issue(request, &review, &admin(), 200)
            .unwrap_or_else(|error| unreachable!("{error:?}"));

        clock.set(200);
        assert_eq!(
            control.validate(evidence.id(), &review),
            Ok(ApprovalValidation::Expired)
        );

        clock.set(150);
        assert_eq!(
            control.validate(evidence.id(), &review),
            Err(ApprovalControlError::ClockRollback)
        );
        assert_eq!(
            control.issue(
                id(ApprovalRequestId::new("approval:request:clock-rollback-new")),
                &review,
                &admin(),
                250,
            ),
            Err(ApprovalControlError::ClockRollback)
        );
    }

"""
replace_once(approval, clock_test_marker, clock_test + clock_test_marker)

planning = "crates/linura-dbus/src/planning.rs"
replace_once(
    planning,
    "    let decision = parse_review_decision(&decision)?;\n    let approval_class = if has_approval_class {",
    "    let decision = parse_review_decision(&decision)?;\n    if preview.status == PlanPreviewStatus::Blocked && decision != PlanReviewDecision::Blocked {\n        return Err(\"blocked plan review must carry a blocked decision\".into());\n    }\n    let approval_class = if has_approval_class {",
)
wire_test_marker = """    #[test]\n    fn wire_decoder_rejects_inconsistent_status_and_execution_authority() {\n"""
wire_test = """    fn blocked_review_wire(decision: &str) -> PlanReviewWire {
        let mut wire = protected_review_wire("read-only", "interactive-user");
        wire.6.0 = "read-only".into();
        wire.6.1 = "read-only".into();
        wire.6.2 = "blocked".into();
        wire.7 = (
            decision.into(),
            false,
            String::new(),
            "blocked by authoritative review".into(),
            false,
        );
        wire.9.push((
            "test-blocker".into(),
            "blocker".into(),
            "blocked review fixture".into(),
        ));
        wire
    }

    #[test]
    fn review_wire_decoder_requires_blocked_decision_for_blocked_status() {
        assert!(plan_review_from_wire(blocked_review_wire("blocked")).is_ok());
        assert!(plan_review_from_wire(blocked_review_wire("allow")).is_err());
        assert!(plan_review_from_wire(blocked_review_wire("deny")).is_err());
    }

"""
replace_once(planning, wire_test_marker, wire_test + wire_test_marker)

threat = "docs/threat-model.md"
replace_once(
    threat,
    "- approval issuance/validation/revocation and replay-tombstone pruning obtain current time inside Linura Control; callers cannot supply the authority clock used to keep evidence current or reopen replay IDs;\n",
    "- approval issuance/validation/revocation and replay-tombstone pruning obtain current time inside Linura Control; callers cannot supply the authority clock used to keep evidence current or reopen replay IDs;\n- authority time is monotonic within the process: a backward host wall-clock sample fails closed, so expired evidence cannot revive and replay-tombstone decisions cannot be reversed by clock rollback;\n",
)
