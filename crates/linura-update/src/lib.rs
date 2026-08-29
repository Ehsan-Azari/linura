#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateStage {
    AcquireLock,
    Preflight,
    DiskSpace,
    Snapshot,
    PackageTransaction,
    Migrations,
    Reconcile,
    RestartAssessment,
    Verify,
    Complete,
    RecoveryRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdatePolicy {
    pub minimum_free_bytes: u64,
    pub require_snapshot_when_available: bool,
    pub inhibit_suspend: bool,
    pub verify_after_restart: bool,
}

impl Default for UpdatePolicy {
    fn default() -> Self {
        Self {
            minimum_free_bytes: 10 * 1024 * 1024 * 1024,
            require_snapshot_when_available: true,
            inhibit_suspend: true,
            verify_after_restart: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateState {
    pub stage: UpdateStage,
    pub snapshot_id: Option<String>,
    pub transaction_id: Option<String>,
    pub recovery_reason: Option<String>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            stage: UpdateStage::AcquireLock,
            snapshot_id: None,
            transaction_id: None,
            recovery_reason: None,
        }
    }
}

impl UpdateState {
    pub fn transition(&mut self, next: UpdateStage) -> Result<(), UpdateError> {
        if next == UpdateStage::RecoveryRequired {
            self.stage = next;
            return Ok(());
        }
        if !valid_transition(self.stage, next) {
            return Err(UpdateError::InvalidTransition {
                from: self.stage,
                to: next,
            });
        }
        self.stage = next;
        Ok(())
    }

    pub fn require_recovery(&mut self, reason: impl Into<String>) {
        self.stage = UpdateStage::RecoveryRequired;
        self.recovery_reason = Some(reason.into());
    }
}

fn valid_transition(from: UpdateStage, to: UpdateStage) -> bool {
    use UpdateStage as S;
    matches!(
        (from, to),
        (S::AcquireLock, S::Preflight)
            | (S::Preflight, S::DiskSpace)
            | (S::DiskSpace, S::Snapshot)
            | (S::Snapshot, S::PackageTransaction)
            | (S::PackageTransaction, S::Migrations)
            | (S::Migrations, S::Reconcile)
            | (S::Reconcile, S::RestartAssessment)
            | (S::RestartAssessment, S::Verify)
            | (S::Verify, S::Complete)
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeUpgradeDecision {
    AllowLinuraCoordinator,
    AllowBreakGlass,
    DenyDirectUpgrade,
}

#[must_use]
pub fn direct_upgrade_decision(
    coordinator_owned: bool,
    break_glass: bool,
) -> NativeUpgradeDecision {
    if coordinator_owned {
        NativeUpgradeDecision::AllowLinuraCoordinator
    } else if break_glass {
        NativeUpgradeDecision::AllowBreakGlass
    } else {
        NativeUpgradeDecision::DenyDirectUpgrade
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateError {
    InvalidTransition { from: UpdateStage, to: UpdateStage },
}

impl Display for UpdateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid update transition {from:?} -> {to:?}")
            }
        }
    }
}

impl std::error::Error for UpdateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_happy_path_is_ordered() {
        let mut state = UpdateState::default();
        for stage in [
            UpdateStage::Preflight,
            UpdateStage::DiskSpace,
            UpdateStage::Snapshot,
            UpdateStage::PackageTransaction,
            UpdateStage::Migrations,
            UpdateStage::Reconcile,
            UpdateStage::RestartAssessment,
            UpdateStage::Verify,
            UpdateStage::Complete,
        ] {
            assert_eq!(state.transition(stage), Ok(()));
        }
        assert_eq!(state.stage, UpdateStage::Complete);
    }

    #[test]
    fn direct_upgrade_is_fail_closed_without_explicit_context() {
        assert_eq!(
            direct_upgrade_decision(false, false),
            NativeUpgradeDecision::DenyDirectUpgrade
        );
        assert_eq!(
            direct_upgrade_decision(true, false),
            NativeUpgradeDecision::AllowLinuraCoordinator
        );
        assert_eq!(
            direct_upgrade_decision(false, true),
            NativeUpgradeDecision::AllowBreakGlass
        );
    }
}
