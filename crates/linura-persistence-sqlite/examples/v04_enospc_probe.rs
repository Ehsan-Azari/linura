#![forbid(unsafe_code)]

use std::env;
use std::path::Path;
use std::process::ExitCode;

use linura_core::{
    CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass, ValidationError,
};
use linura_persistence_sqlite::{SqliteIntegrityKey, SqliteTransactionStore};
use linura_transaction::{
    AbortRequest, AuthorityBinding, AuthorizationBasis, ContentDigest, PrepareOutcome,
    TransactionAuthorityKey, TransactionSnapshot, TransactionState, TransactionStore, digest_bytes,
};

fn id<T>(value: Result<T, ValidationError>) -> Result<T, String> {
    value.map_err(|error| error.to_string())
}

fn digest(value: &str) -> ContentDigest {
    digest_bytes("linura.v04-enospc-probe.v1", value.as_bytes())
}

fn verifier() -> Result<linura_transaction::TransactionAuthorityVerifier, String> {
    TransactionAuthorityKey::new(vec![0x41; 32])
        .map(TransactionAuthorityKey::split)
        .map(|(_, verifier)| verifier)
        .map_err(|error| error.to_string())
}

fn integrity_key() -> Result<SqliteIntegrityKey, String> {
    SqliteIntegrityKey::new(vec![0x73; 32]).map_err(|error| error.to_string())
}

fn binding(namespace: &str) -> Result<AuthorityBinding, String> {
    let request = format!("request:v04-enospc:{namespace}");
    AuthorityBinding::try_new(
        id(PrincipalId::new("uid:1000"))?,
        id(RequestId::new(request.clone()))?,
        id(PlanId::new(request))?,
        digest(&format!("request:{namespace}")),
        digest(&format!("precondition:{namespace}")),
        digest(&format!("observation:{namespace}")),
        id(ProviderId::new("systemd"))?,
        id(ResourceId::new("systemd:unit:v04-enospc.service"))?,
        id(CapabilityId::new("systemd.unit.observe"))?,
        id(PolicyId::new("policy:qualification"))?,
        id(PolicyRevisionId::new("policy:qualification:v1"))?,
        RiskClass::SecuritySensitive,
        "risk-policy:v0.4:qualification",
        vec!["qualification.enospc-recovery".into()],
        digest(&format!("review:{namespace}")),
        AuthorizationBasis::PolicyAllow,
    )
    .map_err(|error| error.to_string())
}

fn prepared_snapshot(outcome: PrepareOutcome) -> TransactionSnapshot {
    match outcome {
        PrepareOutcome::Created(snapshot) | PrepareOutcome::Existing(snapshot) => snapshot,
    }
}

fn open(path: &Path) -> Result<SqliteTransactionStore, String> {
    SqliteTransactionStore::open(path, verifier()?, integrity_key()?)
        .map_err(|error| error.to_string())
}

fn prepare(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let mut store = open(path)?;
    let snapshot = prepared_snapshot(store.prepare(&binding).map_err(|error| error.to_string())?);
    if snapshot.state != TransactionState::Prepared || snapshot.binding_digest != *binding.digest()
    {
        return Err("prepare did not retain the exact authority binding".into());
    }
    store.integrity_check().map_err(|error| error.to_string())?;
    println!(
        "prepared transaction={} generation={} version={}",
        snapshot.transaction_id.as_str(),
        snapshot.current_generation,
        snapshot.state_version
    );
    Ok(())
}

fn abort(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let mut store = open(path)?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Prepared || snapshot.binding_digest != *binding.digest()
    {
        return Err(format!("abort expected exact Prepared state, found {snapshot:?}"));
    }
    let aborted = store
        .abort_prepared(&AbortRequest {
            transaction_id: snapshot.transaction_id.clone(),
            expected_generation: snapshot.current_generation,
            expected_state_version: snapshot.state_version,
            reason_digest: digest(&format!("enospc-abort:{namespace}")),
        })
        .map_err(|error| error.to_string())?;
    if aborted.state != TransactionState::Aborted {
        return Err(format!("terminal recovery did not abort: {aborted:?}"));
    }
    store.integrity_check().map_err(|error| error.to_string())?;
    println!(
        "aborted transaction={} generation={} version={}",
        aborted.transaction_id.as_str(),
        aborted.current_generation,
        aborted.state_version
    );
    Ok(())
}

fn inspect_aborted(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let store = open(path)?;
    store.integrity_check().map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Aborted
        || snapshot.current_generation != 0
        || snapshot.state_version != 2
        || snapshot.binding_digest != *binding.digest()
    {
        return Err(format!("unexpected terminal snapshot after restart: {snapshot:?}"));
    }
    println!(
        "aborted-reopen transaction={} generation={} version={}",
        snapshot.transaction_id.as_str(),
        snapshot.current_generation,
        snapshot.state_version
    );
    Ok(())
}

fn usage() -> String {
    "usage: v04_enospc_probe <prepare|abort|inspect-aborted> <db> <namespace>".into()
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let path = args.next().ok_or_else(usage)?;
    let namespace = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }
    match command.as_str() {
        "prepare" => prepare(Path::new(&path), &namespace),
        "abort" => abort(Path::new(&path), &namespace),
        "inspect-aborted" => inspect_aborted(Path::new(&path), &namespace),
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("v0.4 ENOSPC probe failed: {error}");
            ExitCode::FAILURE
        }
    }
}
