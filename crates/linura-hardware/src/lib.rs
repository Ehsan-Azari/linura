#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceTier {
    Unknown,
    FixtureOnly,
    VirtualMachine,
    CommunityHardware,
    MaintainerHardware,
    ReleaseQualified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareEvidence {
    pub component: String,
    pub vendor: String,
    pub model: String,
    pub tier: EvidenceTier,
    pub profile: String,
    pub evidence_reference: Option<String>,
}

impl HardwareEvidence {
    #[must_use]
    pub fn release_qualified(&self) -> bool {
        self.tier >= EvidenceTier::ReleaseQualified
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HardwareSnapshot {
    pub components: Vec<HardwareEvidence>,
}

impl HardwareSnapshot {
    #[must_use]
    pub fn weakest_tier(&self) -> EvidenceTier {
        self.components.iter().map(|item| item.tier).min().unwrap_or(EvidenceTier::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_snapshot_is_unknown() {
        assert_eq!(HardwareSnapshot::default().weakest_tier(), EvidenceTier::Unknown);
    }
}
