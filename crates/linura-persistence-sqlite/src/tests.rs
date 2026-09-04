use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use linura_core::{
    CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass, ValidationError,
};
use linura_transaction::{
    AuthorizationBasis, RecoveryResolution, TransactionAuthorityKey, TransactionAuthoritySigner,
    TransactionAuthorityVerifier, TransactionSnapshot, TransactionState, TransactionStore,
    TransactionStoreError, TransactionValidationError, digest_bytes,
};
use rusqlite::functions::FunctionFlags;
use rusqlite::{Connection, params};

use super::*;
use crate::validation::{pragma_i64, reservation_bytes};

static NEXT_DB: AtomicU64 = AtomicU64::new(0);

struct TestDatabase {
    path: PathBuf,
}

impl TestDatabase {
    fn new() -> Self {
        let sequence = NEXT_DB.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "linura-v04-hardened-sqlite-{}-{sequence}.db",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(PathBuf::from(format!("{}-wal", path.display())));
        let _ = fs::remove_file(PathBuf::from(format!("{}-shm", path.display())));
        let _ = fs::remove_file(PathBuf::from(format!(
            "{}.linura-recovery-reserve",
            path.display()
        )));
        Self { path }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(PathBuf::from(format!("{}-wal", self.path.display())));
        let _ = fs::remove_file(PathBuf::from(format!("{}-shm", self.path.display())));
        let _ = fs::remove_file(PathBuf::from(format!(
            "{}.linura-recovery-reserve",
            self.path.display()
        )));
    }
}

fn id<T>(value: Result<T, ValidationError>) -> T {
    value.unwrap_or_else(|error| unreachable!("{error}"))
}

fn digest(value: &str) -> linura_transaction::ContentDigest {
    digest_bytes("test", value.as_bytes())
}

fn authority() -> (TransactionAuthoritySigner, TransactionAuthorityVerifier) {
    TransactionAuthorityKey::new(vec![0x41; 32])
        .unwrap_or_else(|error| unreachable!("{error}"))
        .split()
}

fn integrity_key() -> SqliteIntegrityKey {
    SqliteIntegrityKey::new(vec![0x73; 32])
        .unwrap_or_else(|error| unreachable!("{error}"))
}

fn alternate_integrity_key() -> SqliteIntegrityKey {
    SqliteIntegrityKey::new(vec![0x74; 32])
        .unwrap_or_else(|error| unreachable!("{error}"))
}

fn open_store(path: impl AsRef<Path>) -> (TransactionAuthoritySigner, SqliteTransactionStore) {
    let (signer, verifier) = authority();
    let store = SqliteTransactionStore::open(path, verifier, integrity_key())
        .unwrap_or_else(|error| unreachable!("{error}"));
    (signer, store)
}

fn binding_with_request(request: &str, observation: &str) -> linura_transaction::AuthorityBinding {
    linura_transaction::AuthorityBinding::try_new(
        id(PrincipalId::new("uid:1000")),
        id(RequestId::new(request)),
        id(PlanId::new(request)),
        digest("request"),
        digest("precondition"),
        digest(observation),
        id(ProviderId::new("systemd")),
        id(ResourceId::new("systemd:unit:test.service")),
        id(CapabilityId::new("systemd.unit.observe")),
        id(PolicyId::new("policy:baseline")),
        id(PolicyRevisionId::new("policy:baseline:v1")),
        RiskClass::SecuritySensitive,
        "risk-policy:v0.4:1",
        vec!["systemd.active-state.security".into()],
        digest("review"),
        AuthorizationBasis::PolicyAllow,
    )
    .unwrap_or_else(|error: TransactionValidationError| unreachable!("{error}"))
}

fn binding_with_observation(observation: &str) -> linura_transaction::AuthorityBinding {
    binding_with_request("request:sqlite", observation)
}

fn prepared_snapshot(outcome: linura_transaction::PrepareOutcome) -> TransactionSnapshot {
    match outcome {
        linura_transaction::PrepareOutcome::Created(snapshot)
        | linura_transaction::PrepareOutcome::Existing(snapshot) => snapshot,
    }
}

#[test]
fn qualified_sqlite_settings_are_enforced() {
    let db = TestDatabase::new();
    let store = open_store(&db.path).1;
    let settings = store
        .settings()
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(settings.journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(settings.synchronous, 2);
    assert_eq!(settings.foreign_keys, 1);
    assert_eq!(settings.trusted_schema, 0);
    assert!(settings.max_page_count > 0);
}

#[test]
fn exact_prepare_replay_survives_reopen_and_changed_binding_conflicts() {
    let db = TestDatabase::new();
    let binding = binding_with_observation("observation-a");
    let first = {
        let mut store = open_store(&db.path).1;
        prepared_snapshot(
            store
                .prepare(&binding)
                .unwrap_or_else(|error| unreachable!("{error}")),
        )
    };
    let mut reopened = open_store(&db.path).1;
    let replay = prepared_snapshot(
        reopened
            .prepare(&binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    assert_eq!(first, replay);
    assert!(matches!(
        reopened.prepare(&binding_with_observation("observation-b")),
        Err(TransactionStoreError::IdempotencyConflict)
    ));
}

#[test]
fn wrong_verifier_or_record_integrity_key_is_rejected_on_reopen() {
    let db = TestDatabase::new();
    drop(open_store(&db.path).1);

    let wrong_verifier = TransactionAuthorityKey::new(vec![0x52; 32])
        .unwrap_or_else(|error| unreachable!("{error}"))
        .split()
        .1;
    assert!(matches!(
        SqliteTransactionStore::open(&db.path, wrong_verifier, integrity_key()),
        Err(TransactionStoreError::AuthorityRejected)
    ));
    assert!(matches!(
        SqliteTransactionStore::open(&db.path, authority().1, alternate_integrity_key()),
        Err(TransactionStoreError::AuthorityRejected)
    ));
}

#[test]
fn handoff_recovery_verified_commit_and_restart_resume_are_authenticated() {
    let db = TestDatabase::new();
    let binding = binding_with_observation("authenticated-flow");
    let (signer, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let handoff = store
        .handoff(
            &signer
                .authorize_handoff(&prepared, digest("authority-use"), 1, u64::MAX)
                .unwrap_or_else(|error| unreachable!("{error}")),
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    let indeterminate = TransactionSnapshot {
        state: TransactionState::Indeterminate,
        state_version: handoff.state_version,
        ..prepared.clone()
    };
    let desired = digest("desired");
    let graph = digest("graph");
    let provenance = digest("provenance");
    let recovery = signer
        .authorize_recovery(
            &indeterminate,
            RecoveryResolution::IntendedStateVerified {
                observation_digest: digest("verified-observation"),
                desired_state_digest: desired.clone(),
                graph_digest: graph.clone(),
                provenance_digest: provenance.clone(),
            },
            1,
            u64::MAX,
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    let linura_transaction::RecoveryOutcome::Verified(verified) = store
        .recover(&recovery)
        .unwrap_or_else(|error| unreachable!("{error}"))
    else {
        unreachable!("expected verified")
    };
    drop(store);

    let (signer, mut reopened) = open_store(&db.path);
    let material = reopened
        .verified_commit_material(&verified.transaction_id)
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(material.snapshot, verified);
    assert_eq!(material.desired_state_digest, desired);
    assert_eq!(material.graph_digest, graph);
    assert_eq!(material.provenance_digest, provenance);
    let commit = signer
        .authorize_commit(
            &material.snapshot,
            material.desired_state_digest,
            material.graph_digest,
            material.provenance_digest,
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    let committed = reopened
        .commit(&commit)
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(committed.state, TransactionState::Committed);
    reopened
        .integrity_check()
        .unwrap_or_else(|error| unreachable!("{error}"));
}

#[test]
fn a_spoofed_sql_function_cannot_forge_authenticated_state() {
    let db = TestDatabase::new();
    let binding = binding_with_observation("spoofed-gate");
    let (_, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );

    let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    raw.create_scalar_function(
        "linura_internal_mutation_gate",
        0,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS,
        |_| Ok(1_i64),
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    raw.execute(
        "UPDATE generations SET state='verified' WHERE transaction_id=?1 AND generation=0",
        params![prepared.transaction_id.as_str()],
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    assert!(matches!(
        store.snapshot(&prepared.transaction_id),
        Err(TransactionStoreError::Corruption(reason))
            if reason.contains("integrity tag mismatch")
    ));
}

#[test]
fn a_forged_audit_tail_cannot_be_legitimized_by_the_next_store_write() {
    let db = TestDatabase::new();
    let binding = binding_with_observation("forged-audit");
    let (_, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    let zero = linura_transaction::ContentDigest::zero();
    raw.execute(
        "INSERT INTO audit_events (
            transaction_id,event_sequence,generation,state_version,event_kind,
            payload_digest,previous_digest,event_digest,integrity_tag
         ) VALUES (?1,1,0,2,'handoff-indeterminate',?2,?2,?2,zeroblob(32))",
        params![prepared.transaction_id.as_str(), zero.as_str()],
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    drop(raw);
    assert!(matches!(
        store.abort_prepared(&linura_transaction::AbortRequest {
            transaction_id: prepared.transaction_id.clone(),
            expected_generation: prepared.current_generation,
            expected_state_version: prepared.state_version,
            reason_digest: digest("restart-abort"),
        }),
        Err(TransactionStoreError::Corruption(reason))
            if reason.contains("audit record integrity tag mismatch")
    ));
}

#[test]
fn generation_rows_beyond_authenticated_current_pointer_fail_closed() {
    let db = TestDatabase::new();
    let (_, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding_with_observation("forged-future-generation"))
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    raw.execute(
        "INSERT INTO generations (
            transaction_id,generation,state,binding_digest,binding_canonical,
            request_digest,precondition_digest,observation_digest,
            desired_state_digest,graph_digest,provenance_digest,integrity_tag
         ) SELECT transaction_id,1,state,binding_digest,binding_canonical,
                  request_digest,precondition_digest,observation_digest,
                  desired_state_digest,graph_digest,provenance_digest,zeroblob(32)
           FROM generations WHERE transaction_id=?1 AND generation=0",
        params![prepared.transaction_id.as_str()],
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    drop(raw);

    assert!(matches!(
        store.integrity_check(),
        Err(TransactionStoreError::Corruption(reason))
            if reason.contains("retained generation row count disagrees")
    ));
    drop(store);
    assert!(matches!(
        SqliteTransactionStore::open(&db.path, authority().1, integrity_key()),
        Err(TransactionStoreError::Corruption(reason))
            if reason.contains("retained generation row count disagrees")
    ));
}

#[test]
fn complete_integrity_validation_acquires_write_serialization_before_scanning() {
    let db = TestDatabase::new();
    let (_, store) = open_store(&db.path);
    let writer = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    writer
        .execute_batch("BEGIN IMMEDIATE")
        .unwrap_or_else(|error| unreachable!("{error}"));
    store
        .connection
        .busy_timeout(Duration::ZERO)
        .unwrap_or_else(|error| unreachable!("{error}"));

    assert!(matches!(
        store.integrity_check(),
        Err(TransactionStoreError::Storage(reason)) if reason.contains("locked")
    ));

    writer
        .execute_batch("ROLLBACK")
        .unwrap_or_else(|error| unreachable!("{error}"));
    store
        .connection
        .busy_timeout(Duration::from_secs(5))
        .unwrap_or_else(|error| unreachable!("{error}"));
    store
        .integrity_check()
        .unwrap_or_else(|error| unreachable!("{error}"));
}

#[test]
fn physical_page_reserve_prevents_nonterminal_stranding_and_preserves_abort() {
    let db = TestDatabase::new();
    let binding = binding_with_observation("physical-reserve");
    let (signer, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    store
        .connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
        .unwrap_or_else(|error| unreachable!("{error}"));
    let freelist = pragma_i64(&store.connection, "freelist_count").unwrap_or(-1);
    assert_eq!(freelist, 0);
    let page_count = pragma_i64(&store.connection, "page_count")
        .unwrap_or_else(|error| unreachable!("{error}"));
    store
        .connection
        .pragma_update(None, "max_page_count", page_count)
        .unwrap_or_else(|error| unreachable!("{error}"));

    let request = signer
        .authorize_handoff(&prepared, digest("physical-reserve-use"), 1, u64::MAX)
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert!(matches!(
        store.handoff(&request),
        Err(TransactionStoreError::CapacityExceeded)
    ));
    assert_eq!(
        store
            .snapshot(&prepared.transaction_id)
            .unwrap_or_else(|error| unreachable!("{error}")),
        prepared
    );

    let aborted = store
        .abort_prepared(&linura_transaction::AbortRequest {
            transaction_id: prepared.transaction_id.clone(),
            expected_generation: prepared.current_generation,
            expected_state_version: prepared.state_version,
            reason_digest: digest("physical-capacity-abort"),
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(aborted.state, TransactionState::Aborted);
}

#[test]
fn every_nonterminal_transaction_has_exactly_one_durable_reservation_after_commit() {
    let db = TestDatabase::new();
    let (_, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding_with_observation("reservation-shape"))
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let (count, length): (i64, i64) = store
        .connection
        .query_row(
            "SELECT COUNT(*), MIN(length(reserved)) FROM audit_reservations WHERE transaction_id=?1",
            params![prepared.transaction_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(count, 1);
    assert_eq!(
        u64::try_from(length).unwrap_or_default(),
        reservation_bytes(&store.connection).unwrap_or_default()
    );
}

#[test]
fn audit_text_is_length_preflighted_before_rust_materialization() {
    let db = TestDatabase::new();
    let (_, mut store) = open_store(&db.path);
    let prepared = prepared_snapshot(
        store
            .prepare(&binding_with_observation("audit-bound"))
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    raw.execute_batch("PRAGMA ignore_check_constraints=ON;")
        .unwrap_or_else(|error| unreachable!("{error}"));
    let zero = linura_transaction::ContentDigest::zero();
    raw.execute(
        "INSERT INTO audit_events (
            transaction_id,event_sequence,generation,state_version,event_kind,
            payload_digest,previous_digest,event_digest,integrity_tag
         ) VALUES (?1,1,0,2,?2,?3,?3,?3,zeroblob(32))",
        params![prepared.transaction_id.as_str(), "x".repeat(65), zero.as_str()],
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    drop(raw);
    assert!(matches!(
        store.snapshot(&prepared.transaction_id),
        Err(TransactionStoreError::Corruption(reason))
            if reason.contains("audit record exceeds persisted byte bounds")
    ));
}

#[test]
fn logical_audit_capacity_still_reserves_a_safe_exit() {
    let db = TestDatabase::new();
    let limits = StoreLimits {
        max_audit_events: 1,
        ..StoreLimits::default()
    };
    let (_, verifier) = authority();
    let mut store = SqliteTransactionStore::open_with_limits(
        &db.path,
        limits,
        verifier,
        integrity_key(),
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    assert!(matches!(
        store.prepare(&binding_with_observation("logical-reserve")),
        Err(TransactionStoreError::CapacityExceeded)
    ));
}

#[test]
fn installed_schema_tampering_is_detected_on_reopen() {
    let db = TestDatabase::new();
    drop(open_store(&db.path).1);
    let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    raw.execute_batch("DROP TRIGGER generations_binding_immutable;")
        .unwrap_or_else(|error| unreachable!("{error}"));
    drop(raw);
    assert!(matches!(
        SqliteTransactionStore::open(&db.path, authority().1, integrity_key()),
        Err(TransactionStoreError::UnsupportedSchema(_))
    ));
}

#[test]
fn invalid_integrity_key_material_is_rejected() {
    assert!(matches!(
        SqliteIntegrityKey::new(vec![0_u8; 32]),
        Err(TransactionStoreError::AuthorityRejected)
    ));
    assert!(matches!(
        SqliteIntegrityKey::new(vec![0x11; 31]),
        Err(TransactionStoreError::AuthorityRejected)
    ));
}
