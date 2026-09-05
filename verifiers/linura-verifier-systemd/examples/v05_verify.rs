use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use linura_core::{CapabilityId, ProviderId, ResourceId};
use linura_observation::{ObservationAuthority, ObservationEnvelope, ObservedValue};
use linura_provider_sdk::VerificationDisposition;
use linura_verifier_systemd::{
    ACTIVE_ENTER_TIMESTAMP_MONOTONIC, SystemdRestartExpectation, SystemdRestartVerifier,
};

#[derive(Debug)]
struct ProbeError(String);

impl Display for ProbeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ProbeError {}

fn parse_u64(label: &str, value: &str) -> Result<u64, ProbeError> {
    value
        .parse::<u64>()
        .map_err(|error| ProbeError(format!("invalid {label}: {error}")))
}

fn parse_authority(value: &str) -> Result<ObservationAuthority, ProbeError> {
    match value {
        "native-api" => Ok(ObservationAuthority::NativeApi),
        "kernel" => Ok(ObservationAuthority::Kernel),
        "filesystem" => Ok(ObservationAuthority::Filesystem),
        "synthetic-test" => Ok(ObservationAuthority::SyntheticTest),
        _ => Err(ProbeError(format!(
            "unknown observation authority: {value}"
        ))),
    }
}

fn parse_disposition(value: &str) -> Result<VerificationDisposition, ProbeError> {
    match value {
        "satisfied" => Ok(VerificationDisposition::Satisfied),
        "not-satisfied" => Ok(VerificationDisposition::NotSatisfied),
        "inconclusive" => Ok(VerificationDisposition::Inconclusive),
        _ => Err(ProbeError(format!("unknown expected disposition: {value}"))),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 16 {
        return Err(Box::new(ProbeError(
            "usage: v05_verify <satisfied|not-satisfied|inconclusive> <unit> <previous-ts> <provider> <resource> <capability> <authority> <observed-at-ms> <valid-for-ms> <sequence> <id> <load-state> <active-state> <active-enter-ts> <now-ms>".into(),
        )));
    }

    let expected_disposition = parse_disposition(&args[1])?;
    let unit = &args[2];
    let expectation =
        SystemdRestartExpectation::new(unit, parse_u64("previous timestamp", &args[3])?)?;
    let active_enter_timestamp = parse_u64("active-enter timestamp", &args[14])?;
    let observation = ObservationEnvelope {
        provider: ProviderId::new(args[4].clone())?,
        resource: ResourceId::new(args[5].clone())?,
        capability: CapabilityId::new(args[6].clone())?,
        authority: parse_authority(&args[7])?,
        observed_at_unix_ms: parse_u64("observed-at timestamp", &args[8])?,
        valid_for_ms: parse_u64("validity window", &args[9])?,
        sequence: parse_u64("sequence", &args[10])?,
        attributes: BTreeMap::from([
            ("id".into(), ObservedValue::Text(args[11].clone())),
            ("load_state".into(), ObservedValue::Text(args[12].clone())),
            ("active_state".into(), ObservedValue::Text(args[13].clone())),
            (
                ACTIVE_ENTER_TIMESTAMP_MONOTONIC.into(),
                ObservedValue::U64(active_enter_timestamp),
            ),
        ]),
    };
    let outcome = SystemdRestartVerifier.verify_at(
        &expectation,
        &observation,
        parse_u64("verification time", &args[15])?,
    );
    if outcome.disposition != expected_disposition {
        return Err(Box::new(ProbeError(format!(
            "verification disposition mismatch: expected {expected_disposition:?}, got {:?}: {}",
            outcome.disposition, outcome.detail
        ))));
    }

    let label = match outcome.disposition {
        VerificationDisposition::Satisfied => "satisfied",
        VerificationDisposition::NotSatisfied => "not-satisfied",
        VerificationDisposition::Inconclusive => "inconclusive",
    };
    println!("verification={label}");
    println!("detail={}", outcome.detail);
    Ok(())
}
