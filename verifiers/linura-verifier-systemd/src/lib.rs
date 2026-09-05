#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use linura_core::{CapabilityId, ProviderId, ResourceId};
use linura_observation::{
    FreshnessState, ObservationAuthority, ObservationEnvelope, ObservedValue,
};
use linura_provider_sdk::{IndependentVerifier, VerificationDisposition, VerificationOutcome};

pub const SYSTEMD_PROVIDER: &str = "systemd";
pub const SYSTEMD_CAPABILITY: &str = "systemd.unit.observe";
pub const SYSTEMD_RESOURCE_PREFIX: &str = "systemd:unit:";
pub const QUALIFICATION_UNIT_PREFIX: &str = "linura-v05-qualification-";
pub const ACTIVE_ENTER_TIMESTAMP_MONOTONIC: &str = "active_enter_timestamp_monotonic";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemdRestartExpectation {
    pub unit: String,
    pub previous_active_enter_timestamp_monotonic: u64,
}

impl SystemdRestartExpectation {
    pub fn new(
        unit: impl Into<String>,
        previous_active_enter_timestamp_monotonic: u64,
    ) -> Result<Self, VerifierError> {
        let unit = unit.into();
        validate_qualification_unit(&unit)?;
        if previous_active_enter_timestamp_monotonic == 0 {
            return Err(VerifierError::InvalidExpectation(
                "pre-restart activation timestamp must be non-zero".into(),
            ));
        }
        Ok(Self {
            unit,
            previous_active_enter_timestamp_monotonic,
        })
    }

    pub fn resource(&self) -> Result<ResourceId, VerifierError> {
        ResourceId::new(format!("{SYSTEMD_RESOURCE_PREFIX}{}", self.unit))
            .map_err(|error| VerifierError::InvalidExpectation(error.to_string()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifierError {
    InvalidExpectation(String),
    Clock(String),
}

impl Display for VerifierError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidExpectation(detail) => write!(f, "invalid restart expectation: {detail}"),
            Self::Clock(detail) => write!(f, "verification clock failed: {detail}"),
        }
    }
}

impl std::error::Error for VerifierError {}

/// Pure v0.5 restart verifier.
///
/// The verifier deliberately owns no D-Bus connection and performs no observation itself. It
/// consumes only an expected postcondition and a canonical authoritative `ObservationEnvelope`
/// produced independently of the privileged executor. Executor acknowledgements are therefore
/// structurally unable to become machine-state proof.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemdRestartVerifier;

impl SystemdRestartVerifier {
    pub fn verify_at(
        &self,
        expectation: &SystemdRestartExpectation,
        observation: &ObservationEnvelope,
        now_unix_ms: u64,
    ) -> VerificationOutcome {
        let resource = match expectation.resource() {
            Ok(resource) => resource,
            Err(_) => return inconclusive("invalid verifier expectation"),
        };
        if observation
            .validate(&static_provider_id(), &resource, &static_capability_id())
            .is_err()
        {
            return inconclusive("observation identity or structural contract mismatch");
        }
        if observation.authority != ObservationAuthority::NativeApi {
            return inconclusive("verification requires native systemd authority");
        }
        if observation.freshness_at(now_unix_ms) != FreshnessState::Current {
            return inconclusive("verification observation is stale or from the future");
        }

        match text_attribute(observation, "id") {
            Some(id) if id == expectation.unit => {}
            Some(_) => return inconclusive("native observation unit identity mismatch"),
            None => return inconclusive("native observation is missing canonical unit identity"),
        }
        match text_attribute(observation, "load_state") {
            Some("loaded") => {}
            Some(_) => return not_satisfied("systemd unit is not loaded"),
            None => return inconclusive("native observation is missing load state"),
        }
        match text_attribute(observation, "active_state") {
            Some("active") => {}
            Some(_) => return not_satisfied("systemd unit is not active after restart"),
            None => return inconclusive("native observation is missing active state"),
        }
        let Some(ObservedValue::U64(timestamp)) =
            observation.attributes.get(ACTIVE_ENTER_TIMESTAMP_MONOTONIC)
        else {
            return inconclusive("native observation is missing monotonic activation timestamp");
        };
        if *timestamp <= expectation.previous_active_enter_timestamp_monotonic {
            return not_satisfied("systemd activation timestamp did not advance");
        }

        satisfied("fresh native systemd state proves the fixture restarted")
    }
}

impl IndependentVerifier for SystemdRestartVerifier {
    type Expectation = SystemdRestartExpectation;

    fn verify(
        &self,
        expectation: &Self::Expectation,
        observation: &ObservationEnvelope,
    ) -> VerificationOutcome {
        match now_unix_ms() {
            Ok(now) => self.verify_at(expectation, observation, now),
            Err(_) => inconclusive("verification clock is unavailable"),
        }
    }
}

fn validate_qualification_unit(unit: &str) -> Result<(), VerifierError> {
    if unit.is_empty() || unit.len() > 255 || !unit.ends_with(".service") || !unit.is_ascii() {
        return Err(VerifierError::InvalidExpectation(
            "unit must be a bounded ASCII .service name".into(),
        ));
    }
    let Some(suffix) = unit.strip_prefix(QUALIFICATION_UNIT_PREFIX) else {
        return Err(VerifierError::InvalidExpectation(
            "unit is outside the v0.5 qualification namespace".into(),
        ));
    };
    let Some(slug) = suffix.strip_suffix(".service") else {
        return Err(VerifierError::InvalidExpectation(
            "unit suffix is not canonical".into(),
        ));
    };
    if slug.is_empty()
        || slug.len() > 96
        || !slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        || slug.starts_with('-')
        || slug.ends_with('-')
        || slug.contains("--")
    {
        return Err(VerifierError::InvalidExpectation(
            "qualification fixture name is not canonical".into(),
        ));
    }
    Ok(())
}

fn text_attribute<'a>(observation: &'a ObservationEnvelope, name: &str) -> Option<&'a str> {
    match observation.attributes.get(name) {
        Some(ObservedValue::Text(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn static_provider_id() -> ProviderId {
    match ProviderId::new(SYSTEMD_PROVIDER) {
        Ok(value) => value,
        Err(error) => unreachable!("static systemd provider id is invalid: {error}"),
    }
}

fn static_capability_id() -> CapabilityId {
    match CapabilityId::new(SYSTEMD_CAPABILITY) {
        Ok(value) => value,
        Err(error) => unreachable!("static systemd capability id is invalid: {error}"),
    }
}

fn now_unix_ms() -> Result<u64, VerifierError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| VerifierError::Clock(error.to_string()))?;
    u64::try_from(duration.as_millis())
        .map_err(|_| VerifierError::Clock("unix millisecond clock overflowed".into()))
}

fn satisfied(detail: &str) -> VerificationOutcome {
    VerificationOutcome {
        disposition: VerificationDisposition::Satisfied,
        detail: detail.into(),
    }
}

fn not_satisfied(detail: &str) -> VerificationOutcome {
    VerificationOutcome {
        disposition: VerificationDisposition::NotSatisfied,
        detail: detail.into(),
    }
}

fn inconclusive(detail: &str) -> VerificationOutcome {
    VerificationOutcome {
        disposition: VerificationDisposition::Inconclusive,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn id<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
        match value {
            Ok(value) => value,
            Err(error) => unreachable!("{error:?}"),
        }
    }

    fn observation(
        unit: &str,
        observed_at: u64,
        authority: ObservationAuthority,
        timestamp: Option<u64>,
    ) -> ObservationEnvelope {
        let mut attributes = BTreeMap::from([
            ("id".into(), ObservedValue::Text(unit.into())),
            ("load_state".into(), ObservedValue::Text("loaded".into())),
            ("active_state".into(), ObservedValue::Text("active".into())),
            ("sub_state".into(), ObservedValue::Text("running".into())),
        ]);
        if let Some(timestamp) = timestamp {
            attributes.insert(
                ACTIVE_ENTER_TIMESTAMP_MONOTONIC.into(),
                ObservedValue::U64(timestamp),
            );
        }
        ObservationEnvelope {
            provider: static_provider_id(),
            resource: id(ResourceId::new(format!("{SYSTEMD_RESOURCE_PREFIX}{unit}"))),
            capability: static_capability_id(),
            authority,
            observed_at_unix_ms: observed_at,
            valid_for_ms: 2_000,
            sequence: 1,
            attributes,
        }
    }

    #[test]
    fn fresh_native_advanced_timestamp_satisfies_postcondition() {
        let unit = "linura-v05-qualification-restart.service";
        let expectation = id(SystemdRestartExpectation::new(unit, 100));
        let result = SystemdRestartVerifier.verify_at(
            &expectation,
            &observation(unit, 1_000, ObservationAuthority::NativeApi, Some(101)),
            1_500,
        );
        assert_eq!(result.disposition, VerificationDisposition::Satisfied);
    }

    #[test]
    fn unchanged_timestamp_or_inactive_state_is_not_satisfied() {
        let unit = "linura-v05-qualification-restart.service";
        let expectation = id(SystemdRestartExpectation::new(unit, 100));
        let verifier = SystemdRestartVerifier;
        assert_eq!(
            verifier
                .verify_at(
                    &expectation,
                    &observation(unit, 1_000, ObservationAuthority::NativeApi, Some(100)),
                    1_500,
                )
                .disposition,
            VerificationDisposition::NotSatisfied
        );
        let mut inactive = observation(unit, 1_000, ObservationAuthority::NativeApi, Some(101));
        inactive.attributes.insert(
            "active_state".into(),
            ObservedValue::Text("inactive".into()),
        );
        assert_eq!(
            verifier
                .verify_at(&expectation, &inactive, 1_500)
                .disposition,
            VerificationDisposition::NotSatisfied
        );
    }

    #[test]
    fn stale_future_non_native_and_missing_timestamp_are_inconclusive() {
        let unit = "linura-v05-qualification-restart.service";
        let expectation = id(SystemdRestartExpectation::new(unit, 100));
        let verifier = SystemdRestartVerifier;
        let cases = [
            observation(unit, 1_000, ObservationAuthority::NativeApi, Some(101)),
            observation(unit, 5_000, ObservationAuthority::NativeApi, Some(101)),
            observation(unit, 1_000, ObservationAuthority::SyntheticTest, Some(101)),
            observation(unit, 1_000, ObservationAuthority::NativeApi, None),
        ];
        let now = [4_000, 1_500, 1_500, 1_500];
        for (candidate, now) in cases.iter().zip(now) {
            assert_eq!(
                verifier.verify_at(&expectation, candidate, now).disposition,
                VerificationDisposition::Inconclusive
            );
        }
    }

    #[test]
    fn wrong_resource_provider_or_capability_is_inconclusive() {
        let unit = "linura-v05-qualification-restart.service";
        let expectation = id(SystemdRestartExpectation::new(unit, 100));
        let verifier = SystemdRestartVerifier;
        let mut wrong_resource =
            observation(unit, 1_000, ObservationAuthority::NativeApi, Some(101));
        wrong_resource.resource = id(ResourceId::new(
            "systemd:unit:linura-v05-qualification-other.service",
        ));
        assert_eq!(
            verifier
                .verify_at(&expectation, &wrong_resource, 1_500)
                .disposition,
            VerificationDisposition::Inconclusive
        );
        let mut wrong_provider =
            observation(unit, 1_000, ObservationAuthority::NativeApi, Some(101));
        wrong_provider.provider = id(ProviderId::new("other"));
        assert_eq!(
            verifier
                .verify_at(&expectation, &wrong_provider, 1_500)
                .disposition,
            VerificationDisposition::Inconclusive
        );
        let mut wrong_capability =
            observation(unit, 1_000, ObservationAuthority::NativeApi, Some(101));
        wrong_capability.capability = id(CapabilityId::new("systemd.other"));
        assert_eq!(
            verifier
                .verify_at(&expectation, &wrong_capability, 1_500)
                .disposition,
            VerificationDisposition::Inconclusive
        );
    }

    #[test]
    fn qualification_expectation_rejects_wrong_namespace_and_zero_baseline() {
        assert!(SystemdRestartExpectation::new("sshd.service", 1).is_err());
        assert!(
            SystemdRestartExpectation::new("linura-v05-qualification-restart.service", 0,).is_err()
        );
        assert!(
            SystemdRestartExpectation::new("linura-v05-qualification-restart.service", 1,).is_ok()
        );
    }
}
