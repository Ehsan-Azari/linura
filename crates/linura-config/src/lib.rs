#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ownership {
    PackageOwned,
    UserOwned,
    LinuraManaged,
    ExternallyManaged,
    Generated,
    Ephemeral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriftDisposition {
    Accept,
    Reconcile,
    RequireApproval,
    ReportOnly,
    Ignore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedResource {
    pub resource_id: String,
    pub ownership: Ownership,
    pub desired_digest: Option<String>,
    pub observed_digest: Option<String>,
}

impl ManagedResource {
    #[must_use]
    pub fn is_drifted(&self) -> bool {
        matches!((&self.desired_digest, &self.observed_digest), (Some(a), Some(b)) if a != b)
    }

    #[must_use]
    pub fn drift_disposition(&self) -> DriftDisposition {
        if !self.is_drifted() {
            return DriftDisposition::Accept;
        }
        match self.ownership {
            Ownership::LinuraManaged => DriftDisposition::RequireApproval,
            Ownership::PackageOwned => DriftDisposition::ReportOnly,
            Ownership::UserOwned | Ownership::ExternallyManaged => DriftDisposition::ReportOnly,
            Ownership::Generated => DriftDisposition::Reconcile,
            Ownership::Ephemeral => DriftDisposition::Ignore,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnershipRegistry {
    resources: BTreeMap<String, ManagedResource>,
}

impl OwnershipRegistry {
    pub fn register(&mut self, resource: ManagedResource) -> Option<ManagedResource> {
        self.resources.insert(resource.resource_id.clone(), resource)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ManagedResource> {
        self.resources.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linura_managed_drift_is_not_silently_overwritten() {
        let resource = ManagedResource {
            resource_id: "file:/etc/example".into(), ownership: Ownership::LinuraManaged,
            desired_digest: Some("sha256:a".into()), observed_digest: Some("sha256:b".into()),
        };
        assert_eq!(resource.drift_disposition(), DriftDisposition::RequireApproval);
    }
}
