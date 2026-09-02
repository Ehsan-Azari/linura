#![forbid(unsafe_code)]

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use linura_core::{
    CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass, ValidationError,
};
use linura_persistence_sqlite::SqliteTransactionStore;
use linura_transaction::{
    AuthorityBinding, AuthorizationBasis, ContentDigest, HandoffRequest, PrepareOutcome,
    TransactionSnapshot, TransactionState, TransactionStore, digest_bytes,
};
use rusqlite::Connection;

fn id<T>(value: Result<T, ValidationError>) -> Result<T, String> {
    value.map_err(|error| error.to_string())
}

fn digest(value: &str) -> ContentDigest {
    digest_bytes("linura.v04-fault-probe.v1", value.as_bytes())
}

fn binding(namespace: &str) -> Result<AuthorityBinding, String> {
    let request = format!("request:v04-fault:{namespace}");
    AuthorityBinding::try_new(
        id(PrincipalId::new("uid:1000"))?,
        id(RequestId::new(request.clone()))?,
        id(PlanId::new(request))?,
        digest(&format!("request:{namespace}")),
        digest(&format!("precondition:{namespace}")),
        digest(&format!("observation:{namespace}")),
        id(ProviderId::new("systemd"))?,
        id(ResourceId::new("systemd:unit:v04-fault.service"))?,
        id(CapabilityId::new("systemd.unit.observe"))?,
        id(PolicyId::new("policy:qualification"))?,
        id(PolicyRevisionId::new("policy:qualification:v1"))?,
        RiskClass::SecuritySensitive,
        "risk-policy:v0.4:qualification",
        vec!["qualification.no-external-effect".into()],
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

fn prepare(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let mut store = SqliteTransactionStore::open(path).map_err(|error| error.to_string())?;
    let snapshot = prepared_snapshot(store.prepare(&binding).map_err(|error| error.to_string())?);
    if snapshot.state != TransactionState::Prepared || snapshot.binding_digest != *binding.digest()
    {
        return Err("prepare did not retain the exact prepared binding".into());
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

fn handoff_and_wait(path: &Path, namespace: &str, marker: &Path) -> Result<(), String> {
    let binding = binding(namespace)?;
    let store = SqliteTransactionStore::open(path).map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Prepared || snapshot.binding_digest != *binding.digest()
    {
        return Err("handoff probe expected the exact prepared generation".into());
    }

    let mut store = store;
    let commit = store
        .handoff(&HandoffRequest {
            transaction_id: snapshot.transaction_id.clone(),
            expected_generation: snapshot.current_generation,
            expected_state_version: snapshot.state_version,
            expected_binding_digest: snapshot.binding_digest.clone(),
            authority_use_digest: digest(&format!("handoff-authority:{namespace}")),
        })
        .map_err(|error| error.to_string())?;
    if commit.generation != snapshot.current_generation {
        return Err("handoff commit generation mismatch".into());
    }

    let after = store
        .snapshot(&snapshot.transaction_id)
        .map_err(|error| error.to_string())?;
    if after.state != TransactionState::Indeterminate
        || after.state_version <= snapshot.state_version
    {
        return Err("handoff was not durably indeterminate before acknowledgement".into());
    }
    store.integrity_check().map_err(|error| error.to_string())?;

    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(marker)
        .map_err(|error| error.to_string())?;
    writeln!(
        file,
        "transaction={} generation={} version={}",
        after.transaction_id.as_str(),
        after.current_generation,
        after.state_version
    )
    .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    println!("handoff acknowledged; waiting for injected termination");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn inspect_indeterminate(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let store = SqliteTransactionStore::open(path).map_err(|error| error.to_string())?;
    store.integrity_check().map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Indeterminate
        || snapshot.current_generation != 0
        || snapshot.state_version != 2
        || snapshot.binding_digest != *binding.digest()
    {
        return Err(format!("unexpected reopened snapshot: {snapshot:?}"));
    }
    println!(
        "indeterminate transaction={} generation={} version={} binding={}",
        snapshot.transaction_id.as_str(),
        snapshot.current_generation,
        snapshot.state_version,
        snapshot.binding_digest.as_str()
    );
    Ok(())
}

fn checkpoint_and_inspect(path: &Path, namespace: &str) -> Result<(), String> {
    inspect_indeterminate(path, namespace)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let result: (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(FULL)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| error.to_string())?;
    if result.0 != 0 {
        return Err(format!("WAL checkpoint remained busy: {result:?}"));
    }
    drop(connection);
    inspect_indeterminate(path, namespace)
}

fn usage() -> String {
    "usage: v04_fault_probe <prepare|handoff-wait|inspect-indeterminate|checkpoint-inspect> <db> <namespace> [marker]".into()
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let path = args.next().ok_or_else(usage)?;
    let namespace = args.next().ok_or_else(usage)?;
    match command.as_str() {
        "prepare" => {
            if args.next().is_some() {
                return Err(usage());
            }
            prepare(Path::new(&path), &namespace)
        }
        "handoff-wait" => {
            let marker = args.next().ok_or_else(usage)?;
            if args.next().is_some() {
                return Err(usage());
            }
            handoff_and_wait(Path::new(&path), &namespace, Path::new(&marker))
        }
        "inspect-indeterminate" => {
            if args.next().is_some() {
                return Err(usage());
            }
            inspect_indeterminate(Path::new(&path), &namespace)
        }
        "checkpoint-inspect" => {
            if args.next().is_some() {
                return Err(usage());
            }
            checkpoint_and_inspect(Path::new(&path), &namespace)
        }
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("v0.4 fault probe failed: {error}");
            ExitCode::FAILURE
        }
    }
}
