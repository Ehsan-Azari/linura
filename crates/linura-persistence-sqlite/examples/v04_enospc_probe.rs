use std::env;
use std::path::Path;

use linura_core::{
    CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass,
};
use linura_persistence_sqlite::{SqliteIntegrityKey, SqliteTransactionStore};
use linura_transaction::{
    AbortRequest, AuthorityBinding, AuthorizationBasis, PrepareOutcome, TransactionAuthorityKey,
    TransactionSnapshot, TransactionState, TransactionStore, digest_bytes,
};

fn authority() -> Result<linura_transaction::TransactionAuthorityVerifier, String> {
    Ok(TransactionAuthorityKey::new(vec![0x41; 32])
        .map_err(|error| error.to_string())?
        .split()
        .1)
}

fn integrity_key() -> Result<SqliteIntegrityKey, String> {
    SqliteIntegrityKey::new(vec![0x73; 32]).map_err(|error| error.to_string())
}

fn binding(namespace: &str) -> Result<AuthorityBinding, String> {
    AuthorityBinding::try_new(
        PrincipalId::new("uid:1000").map_err(|error| error.to_string())?,
        RequestId::new(format!("request:v04-enospc:{namespace}"))
            .map_err(|error| error.to_string())?,
        PlanId::new(format!("plan:v04-enospc:{namespace}")).map_err(|error| error.to_string())?,
        digest_bytes("v04-enospc", format!("request:{namespace}").as_bytes()),
        digest_bytes("v04-enospc", format!("precondition:{namespace}").as_bytes()),
        digest_bytes("v04-enospc", format!("observation:{namespace}").as_bytes()),
        ProviderId::new("systemd").map_err(|error| error.to_string())?,
        ResourceId::new(format!("systemd:unit:{namespace}.service"))
            .map_err(|error| error.to_string())?,
        CapabilityId::new("systemd.unit.observe").map_err(|error| error.to_string())?,
        PolicyId::new("policy:v04-enospc").map_err(|error| error.to_string())?,
        PolicyRevisionId::new("policy:v04-enospc:v1").map_err(|error| error.to_string())?,
        RiskClass::SecuritySensitive,
        "risk-policy:v0.4:enospc",
        vec!["enospc-qualification".into()],
        digest_bytes("v04-enospc", format!("review:{namespace}").as_bytes()),
        AuthorizationBasis::PolicyAllow,
    )
    .map_err(|error| error.to_string())
}

fn prepared(outcome: PrepareOutcome) -> TransactionSnapshot {
    match outcome {
        PrepareOutcome::Created(snapshot) | PrepareOutcome::Existing(snapshot) => snapshot,
    }
}

fn prepare(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let mut store = SqliteTransactionStore::open(path, authority()?, integrity_key()?)
        .map_err(|error| error.to_string())?;
    let snapshot = prepared(store.prepare(&binding).map_err(|error| error.to_string())?);
    if snapshot.state != TransactionState::Prepared || snapshot.binding_digest != *binding.digest()
    {
        return Err(format!(
            "prepare did not persist exact Prepared state: {snapshot:?}"
        ));
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
    let mut store =
        SqliteTransactionStore::open_for_terminal_recovery(path, authority()?, integrity_key()?)
            .map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Prepared || snapshot.binding_digest != *binding.digest()
    {
        return Err(format!(
            "abort expected exact Prepared state, found {snapshot:?}"
        ));
    }
    let aborted = store
        .abort_prepared(&AbortRequest {
            transaction_id: snapshot.transaction_id.clone(),
            expected_generation: snapshot.current_generation,
            expected_state_version: snapshot.state_version,
            reason_digest: digest_bytes("v04-enospc", b"qualified-enospc-abort"),
        })
        .map_err(|error| error.to_string())?;
    if aborted.state != TransactionState::Aborted {
        return Err(format!("terminal retirement did not abort: {aborted:?}"));
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
    let store = SqliteTransactionStore::open(path, authority()?, integrity_key()?)
        .map_err(|error| error.to_string())?;
    store.integrity_check().map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Aborted
        || snapshot.current_generation != 0
        || snapshot.state_version != 2
        || snapshot.binding_digest != *binding.digest()
    {
        return Err(format!(
            "unexpected terminal snapshot after restart: {snapshot:?}"
        ));
    }
    println!(
        "aborted-reopen transaction={} generation={} version={}",
        snapshot.transaction_id.as_str(),
        snapshot.current_generation,
        snapshot.state_version
    );
    Ok(())
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    let result = match arguments.as_slice() {
        [_, command, database, namespace] if command == "prepare" => {
            prepare(Path::new(database), namespace)
        }
        [_, command, database, namespace] if command == "abort" => {
            abort(Path::new(database), namespace)
        }
        [_, command, database, namespace] if command == "inspect-aborted" => {
            inspect_aborted(Path::new(database), namespace)
        }
        _ => Err(
            "usage: v04_enospc_probe <prepare|abort|inspect-aborted> <database> <namespace>".into(),
        ),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
