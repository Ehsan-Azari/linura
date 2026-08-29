#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BootstrapStage {
    Preflight,
    DiskLayout,
    Encryption,
    BaseSystem,
    PlatformPackages,
    Bootloader,
    SecurityBaseline,
    SnapshotBaseline,
    UserProvisioning,
    FirstBootReady,
}

impl BootstrapStage {
    pub const ORDERED: [Self; 10] = [
        Self::Preflight,
        Self::DiskLayout,
        Self::Encryption,
        Self::BaseSystem,
        Self::PlatformPackages,
        Self::Bootloader,
        Self::SecurityBaseline,
        Self::SnapshotBaseline,
        Self::UserProvisioning,
        Self::FirstBootReady,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapMode {
    Interactive,
    NonInteractive,
    Recovery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallSecurityPolicy {
    pub require_disk_encryption: bool,
    pub inbound_firewall_default_deny: bool,
    pub ssh_enabled_initially: bool,
    pub untrusted_package_sources_enabled: bool,
}

impl Default for InstallSecurityPolicy {
    fn default() -> Self {
        Self {
            require_disk_encryption: true,
            inbound_firewall_default_deny: true,
            ssh_enabled_initially: false,
            untrusted_package_sources_enabled: false,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BootstrapLedger {
    completed: BTreeSet<BootstrapStage>,
}

impl BootstrapLedger {
    pub fn mark_completed(&mut self, stage: BootstrapStage) {
        self.completed.insert(stage);
    }

    #[must_use]
    pub fn is_completed(&self, stage: BootstrapStage) -> bool {
        self.completed.contains(&stage)
    }

    #[must_use]
    pub fn next_stage(&self) -> Option<BootstrapStage> {
        BootstrapStage::ORDERED
            .into_iter()
            .find(|stage| !self.completed.contains(stage))
    }

    pub fn validate_prefix(&self) -> Result<(), BootstrapError> {
        let mut saw_gap = false;
        for stage in BootstrapStage::ORDERED {
            if self.completed.contains(&stage) {
                if saw_gap {
                    return Err(BootstrapError::OutOfOrder(stage));
                }
            } else {
                saw_gap = true;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BootstrapError {
    OutOfOrder(BootstrapStage),
}

impl Display for BootstrapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfOrder(stage) => write!(
                f,
                "bootstrap ledger contains out-of-order completed stage: {stage:?}"
            ),
        }
    }
}

impl std::error::Error for BootstrapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_security_policy_is_fail_closed() {
        let policy = InstallSecurityPolicy::default();
        assert!(policy.require_disk_encryption);
        assert!(policy.inbound_firewall_default_deny);
        assert!(!policy.ssh_enabled_initially);
        assert!(!policy.untrusted_package_sources_enabled);
    }

    #[test]
    fn ledger_resumes_at_first_incomplete_stage() {
        let mut ledger = BootstrapLedger::default();
        ledger.mark_completed(BootstrapStage::Preflight);
        ledger.mark_completed(BootstrapStage::DiskLayout);
        assert_eq!(ledger.next_stage(), Some(BootstrapStage::Encryption));
        assert_eq!(ledger.validate_prefix(), Ok(()));
    }

    #[test]
    fn ledger_rejects_gaps() {
        let mut ledger = BootstrapLedger::default();
        ledger.mark_completed(BootstrapStage::Preflight);
        ledger.mark_completed(BootstrapStage::Encryption);
        assert_eq!(
            ledger.validate_prefix(),
            Err(BootstrapError::OutOfOrder(BootstrapStage::Encryption))
        );
    }
}
