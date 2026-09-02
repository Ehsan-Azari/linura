#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/linura-control/src/approval_review.rs")
text = path.read_text(encoding="utf-8")


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one occurrence, found {count}: {old[:120]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "use std::collections::BTreeMap;\n",
    "use std::collections::BTreeMap;\n"
    "use std::fmt::Debug;\n"
    "use std::time::{SystemTime, UNIX_EPOCH};\n",
)

replace_once(
    """    EvidenceNotFound,
    RevokerNotAuthorized,
}
""",
    """    EvidenceNotFound,
    RevokerNotAuthorized,
    ClockUnavailable,
}

trait ApprovalClock: Debug + Send + Sync {
    fn now_unix_seconds(&self) -> Result<u64, ApprovalControlError>;
}

#[derive(Debug, Default)]
struct SystemApprovalClock;

impl ApprovalClock for SystemApprovalClock {
    fn now_unix_seconds(&self) -> Result<u64, ApprovalControlError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|_| ApprovalControlError::ClockUnavailable)
    }
}
""",
)

replace_once(
    """#[derive(Debug, Default)]
pub struct ApprovalReviewControl {
    records: BTreeMap<ApprovalEvidenceId, ApprovalRecord>,
""",
    """#[derive(Debug)]
pub struct ApprovalReviewControl {
    clock: Box<dyn ApprovalClock>,
    records: BTreeMap<ApprovalEvidenceId, ApprovalRecord>,
""",
)

replace_once(
    """}

impl ApprovalReviewControl {
    #[must_use]
    pub fn len(&self) -> usize {
""",
    """}

impl Default for ApprovalReviewControl {
    fn default() -> Self {
        Self::with_clock(Box::new(SystemApprovalClock))
    }
}

impl ApprovalReviewControl {
    fn with_clock(clock: Box<dyn ApprovalClock>) -> Self {
        Self {
            clock,
            records: BTreeMap::new(),
            requests: BTreeMap::new(),
            tombstones: BTreeMap::new(),
            next_evidence_sequence: 0,
            total_accounted_bytes: 0,
            tombstone_accounted_bytes: 0,
        }
    }

    fn authority_now_unix_seconds(&self) -> Result<u64, ApprovalControlError> {
        self.clock.now_unix_seconds()
    }

    #[must_use]
    pub fn len(&self) -> usize {
""",
)

replace_once(
    """    pub fn issue(
        &mut self,
        request_id: ApprovalRequestId,
        review: &TrustedPolicyReview,
        approver: &PolicyAuthenticatedApprover,
        issued_at_unix_seconds: u64,
        expires_at_unix_seconds: u64,
    ) -> Result<PolicyApprovalEvidence, ApprovalControlError> {
        self.reclaim_inactive(issued_at_unix_seconds)?;
""",
    """    /// Issue approval evidence using Control-owned current time.
    ///
    /// The caller may request an absolute expiry, but cannot supply the
    /// authority clock used for issuance, reclamation, validation, revocation,
    /// or replay-tombstone pruning.
    pub fn issue(
        &mut self,
        request_id: ApprovalRequestId,
        review: &TrustedPolicyReview,
        approver: &PolicyAuthenticatedApprover,
        expires_at_unix_seconds: u64,
    ) -> Result<PolicyApprovalEvidence, ApprovalControlError> {
        let issued_at_unix_seconds = self.authority_now_unix_seconds()?;
        self.issue_at(
            request_id,
            review,
            approver,
            issued_at_unix_seconds,
            expires_at_unix_seconds,
        )
    }

    fn issue_at(
        &mut self,
        request_id: ApprovalRequestId,
        review: &TrustedPolicyReview,
        approver: &PolicyAuthenticatedApprover,
        issued_at_unix_seconds: u64,
        expires_at_unix_seconds: u64,
    ) -> Result<PolicyApprovalEvidence, ApprovalControlError> {
        self.reclaim_inactive(issued_at_unix_seconds)?;
""",
)

replace_once(
    """                && existing.evidence.approver() == approver.principal()
                && existing.evidence.issued_at_unix_seconds() == issued_at_unix_seconds
                && existing.evidence.expires_at_unix_seconds() == expires_at_unix_seconds
""",
    """                && existing.evidence.approver() == approver.principal()
                && existing.evidence.expires_at_unix_seconds() == expires_at_unix_seconds
""",
)

replace_once(
    """    #[must_use]
    pub fn validate(
        &self,
        evidence_id: &ApprovalEvidenceId,
        review: &TrustedPolicyReview,
        now_unix_seconds: u64,
    ) -> ApprovalValidation {
        let Some(record) = self.records.get(evidence_id) else {
            return ApprovalValidation::BindingMismatch;
        };
        validate_policy_approval(
            &record.evidence,
            review,
            now_unix_seconds,
            record.revocation.as_ref(),
        )
    }

    pub fn revoke(
        &mut self,
        evidence_id: &ApprovalEvidenceId,
        revoker: &PolicyAuthenticatedApprover,
        revoked_at_unix_seconds: u64,
    ) -> Result<(), ApprovalControlError> {
        let Some(record) = self.records.get_mut(evidence_id) else {
""",
    """    /// Validate retained approval against Control-owned current time and
    /// authoritative revocation state.
    pub fn validate(
        &self,
        evidence_id: &ApprovalEvidenceId,
        review: &TrustedPolicyReview,
    ) -> Result<ApprovalValidation, ApprovalControlError> {
        let now_unix_seconds = self.authority_now_unix_seconds()?;
        Ok(self.validate_at(evidence_id, review, now_unix_seconds))
    }

    fn validate_at(
        &self,
        evidence_id: &ApprovalEvidenceId,
        review: &TrustedPolicyReview,
        now_unix_seconds: u64,
    ) -> ApprovalValidation {
        let Some(record) = self.records.get(evidence_id) else {
            return ApprovalValidation::BindingMismatch;
        };
        validate_policy_approval(
            &record.evidence,
            review,
            now_unix_seconds,
            record.revocation.as_ref(),
        )
    }

    /// Revoke retained approval using Control-owned current time.
    pub fn revoke(
        &mut self,
        evidence_id: &ApprovalEvidenceId,
        revoker: &PolicyAuthenticatedApprover,
    ) -> Result<(), ApprovalControlError> {
        let revoked_at_unix_seconds = self.authority_now_unix_seconds()?;
        self.revoke_at(evidence_id, revoker, revoked_at_unix_seconds)
    }

    fn revoke_at(
        &mut self,
        evidence_id: &ApprovalEvidenceId,
        revoker: &PolicyAuthenticatedApprover,
        revoked_at_unix_seconds: u64,
    ) -> Result<(), ApprovalControlError> {
        let Some(record) = self.records.get_mut(evidence_id) else {
""",
)

marker = "#[cfg(test)]\nmod tests {\n"
if text.count(marker) != 1:
    raise SystemExit("test module marker mismatch")
production, tests = text.split(marker, 1)

tests = tests.replace(
    "    use std::collections::BTreeSet;\n",
    "    use std::collections::BTreeSet;\n"
    "    use std::sync::Arc;\n"
    "    use std::sync::atomic::{AtomicU64, Ordering};\n",
    1,
)

id_helper = """    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!(\"{error}\"))
    }

"""
clock_helper = """    fn id<T>(result: Result<T, ValidationError>) -> T {
        result.unwrap_or_else(|error| unreachable!(\"{error}\"))
    }

    #[derive(Clone, Debug)]
    struct TestClock {
        now_unix_seconds: Arc<AtomicU64>,
    }

    impl TestClock {
        fn new(now_unix_seconds: u64) -> Self {
            Self {
                now_unix_seconds: Arc::new(AtomicU64::new(now_unix_seconds)),
            }
        }

        fn set(&self, now_unix_seconds: u64) {
            self.now_unix_seconds
                .store(now_unix_seconds, Ordering::SeqCst);
        }
    }

    impl ApprovalClock for TestClock {
        fn now_unix_seconds(&self) -> Result<u64, ApprovalControlError> {
            Ok(self.now_unix_seconds.load(Ordering::SeqCst))
        }
    }

    fn control_with_clock(now_unix_seconds: u64) -> (ApprovalReviewControl, TestClock) {
        let clock = TestClock::new(now_unix_seconds);
        let control = ApprovalReviewControl::with_clock(Box::new(clock.clone()));
        (control, clock)
    }

"""
if tests.count(id_helper) != 1:
    raise SystemExit("id helper mismatch")
tests = tests.replace(id_helper, clock_helper, 1)

# Existing deterministic tests use private trusted-time helpers. Production
# APIs obtain current time exclusively from the Control-owned clock.
tests = tests.replace(".issue(", ".issue_at(")
tests = tests.replace(".validate(", ".validate_at(")
tests = tests.replace(".revoke(", ".revoke_at(")

insert_before = """    #[test]
    fn retained_bytes_are_bounded_and_idempotent_retry_does_not_grow_store() {
"""
new_test = """    #[test]
    fn public_authority_api_uses_control_owned_time() {
        let review = review(RiskClass::SecuritySensitive);
        let request = id(ApprovalRequestId::new(\"approval:request:authority-clock\"));
        let (mut control, clock) = control_with_clock(100);

        let evidence = control
            .issue(request.clone(), &review, &admin(), 200)
            .unwrap_or_else(|error| unreachable!(\"{error:?}\"));
        assert_eq!(evidence.issued_at_unix_seconds(), 100);
        assert_eq!(evidence.expires_at_unix_seconds(), 200);

        clock.set(101);
        let retry = control
            .issue(request.clone(), &review, &admin(), 200)
            .unwrap_or_else(|error| unreachable!(\"{error:?}\"));
        assert_eq!(retry, evidence);

        clock.set(150);
        assert_eq!(
            control.validate(evidence.id(), &review),
            Ok(ApprovalValidation::Satisfied)
        );

        clock.set(200);
        assert_eq!(
            control.validate(evidence.id(), &review),
            Ok(ApprovalValidation::Expired)
        );

        let _ = control
            .issue_at(
                id(ApprovalRequestId::new(\"approval:request:authority-clock-trigger\")),
                &review,
                &admin(),
                200,
                250,
            )
            .unwrap_or_else(|error| unreachable!(\"{error:?}\"));
        assert!(control.get(evidence.id()).is_none());
        assert_eq!(
            control.issue_at(request, &review, &admin(), 200, 250),
            Err(ApprovalControlError::IdempotencyConflict)
        );
    }

"""
if tests.count(insert_before) != 1:
    raise SystemExit("retained test marker mismatch")
tests = tests.replace(insert_before, new_test + insert_before, 1)

text = production + marker + tests

threat = Path("docs/threat-model.md")
threat_text = threat.read_text(encoding="utf-8")
old_bullet = "- approval evidence is authenticated, scoped to an explicit approval requirement, and checked for approver constraints, expiry and revocation at use time;\n"
new_bullet = (
    old_bullet
    + "- approval issuance/validation/revocation and replay-tombstone pruning obtain current time inside Linura Control; callers cannot supply the authority clock used to keep evidence current or reopen replay IDs;\n"
)
if threat_text.count(old_bullet) != 1:
    raise SystemExit("threat-model approval-time marker mismatch")
threat_text = threat_text.replace(old_bullet, new_bullet, 1)

path.write_text(text, encoding="utf-8")
threat.write_text(threat_text, encoding="utf-8")
