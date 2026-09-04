use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use linura_core::{
    CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass,
};
use linura_persistence_sqlite::{SqliteIntegrityKey, SqliteTransactionStore};
use linura_transaction::{
    AbortRequest, AuthorityBinding, AuthorizationBasis, PrepareOutcome, TransactionAuthorityKey,
    TransactionSnapshot, TransactionState, TransactionStore, digest_bytes,
};
use rusqlite::{Connection, params};

static NEXT_DB: AtomicU64 = AtomicU64::new(0);

struct TestDatabase {
    path: PathBuf,
}

impl TestDatabase {
    fn new() -> Self {
        let sequence = NEXT_DB.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "linura-v04-release-blockers-{}-{sequence}.db",
            std::process::id()
        ));
        cleanup(&path);
        Self { path }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        cleanup(&self.path);
    }
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);
    for suffix in ["-wal", "-shm", ".linura-recovery-reserve"] {
        let _ = fs::remove_file(PathBuf::from(format!("{}{}", path.display(), suffix)));
    }
}

fn integrity_key() -> SqliteIntegrityKey {
    SqliteIntegrityKey::new(vec![0x73; 32]).unwrap_or_else(|error| unreachable!("{error}"))
}

fn verifier() -> linura_transaction::TransactionAuthorityVerifier {
    TransactionAuthorityKey::new(vec![0x41; 32])
        .unwrap_or_else(|error| unreachable!("{error}"))
        .split()
        .1
}

fn binding_named(request_id: &str, plan_id: &str) -> AuthorityBinding {
    AuthorityBinding::try_new(
        PrincipalId::new("uid:1000").unwrap_or_else(|error| unreachable!("{error}")),
        RequestId::new(request_id).unwrap_or_else(|error| unreachable!("{error}")),
        PlanId::new(plan_id).unwrap_or_else(|error| unreachable!("{error}")),
        digest_bytes("test", request_id.as_bytes()),
        digest_bytes("test", plan_id.as_bytes()),
        digest_bytes("test", format!("observation:{request_id}").as_bytes()),
        ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}")),
        ResourceId::new("systemd:unit:test.service")
            .unwrap_or_else(|error| unreachable!("{error}")),
        CapabilityId::new("systemd.unit.observe").unwrap_or_else(|error| unreachable!("{error}")),
        PolicyId::new("policy:baseline").unwrap_or_else(|error| unreachable!("{error}")),
        PolicyRevisionId::new("policy:baseline:v1").unwrap_or_else(|error| unreachable!("{error}")),
        RiskClass::SecuritySensitive,
        "risk-policy:v0.4:1",
        vec!["release-blocker-regression".into()],
        digest_bytes("test", format!("review:{request_id}").as_bytes()),
        AuthorizationBasis::PolicyAllow,
    )
    .unwrap_or_else(|error| unreachable!("{error}"))
}

fn binding() -> AuthorityBinding {
    binding_named("request:release-blockers", "plan:release-blockers")
}

fn prepared(outcome: PrepareOutcome) -> TransactionSnapshot {
    match outcome {
        PrepareOutcome::Created(snapshot) | PrepareOutcome::Existing(snapshot) => snapshot,
    }
}

#[test]
fn filesystem_recovery_reserve_tracks_nonterminal_and_terminal_state() {
    let db = TestDatabase::new();
    let reserve = PathBuf::from(format!("{}.linura-recovery-reserve", db.path.display()));
    let mut store = SqliteTransactionStore::open(&db.path, verifier(), integrity_key())
        .unwrap_or_else(|error| unreachable!("{error}"));

    // With no durable nonterminal authority, no reserve allocation is needed.
    assert!(
        !reserve.exists()
            || fs::metadata(&reserve)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len()
                == 0
    );
    let snapshot = prepared(
        store
            .prepare(&binding())
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let prepared_reserve = fs::metadata(&reserve)
        .unwrap_or_else(|error| unreachable!("{error}"))
        .len();
    // Prepare admits one durable reservation only after the sidecar contains
    // that reservation plus a dedicated 1 MiB pre-open recovery slot.
    assert!(prepared_reserve >= 2 * 1024 * 1024);

    let aborted = store
        .abort_prepared(&AbortRequest {
            transaction_id: snapshot.transaction_id.clone(),
            expected_generation: snapshot.current_generation,
            expected_state_version: snapshot.state_version,
            reason_digest: digest_bytes("test", b"abort"),
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(aborted.state, TransactionState::Aborted);
    // Once no durable nonterminal authority remains, no emergency sidecar
    // allocation is required. The next prepare must re-establish both its
    // transaction reservation and dedicated opener headroom before admission.
    assert!(
        !reserve.exists()
            || fs::metadata(&reserve)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len()
                == 0
    );
    store
        .integrity_check()
        .unwrap_or_else(|error| unreachable!("{error}"));
}

#[cfg(target_os = "linux")]
#[test]
fn multi_reservation_terminal_recovery_never_shrinks_before_sqlite_commit() {
    let db = TestDatabase::new();
    let reserve = PathBuf::from(format!("{}.linura-recovery-reserve", db.path.display()));
    let first_binding = binding_named("request:multi-first", "plan:multi-first");
    let second_binding = binding_named("request:multi-second", "plan:multi-second");

    let mut store = SqliteTransactionStore::open(&db.path, verifier(), integrity_key())
        .unwrap_or_else(|error| unreachable!("{error}"));
    let first = prepared(
        store
            .prepare(&first_binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let second = prepared(
        store
            .prepare(&second_binding)
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    drop(store);

    let three_slot_len = fs::metadata(&reserve)
        .unwrap_or_else(|error| unreachable!("{error}"))
        .len();
    assert!(three_slot_len >= 3 * 1024 * 1024);

    // Punch only the dedicated opener slot, then emulate a process death before
    // terminal mutation. The subsequent raw SQLite DELETE/ROLLBACK exercises
    // the exact multi-reservation failure window from the review: V2 must not
    // truncate the sidecar while SQLite can still restore the deleted row.
    let recovery =
        SqliteTransactionStore::open_for_terminal_recovery(&db.path, verifier(), integrity_key())
            .unwrap_or_else(|error| unreachable!("{error}"));
    drop(recovery);

    let mut raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    let transaction = raw
        .transaction()
        .unwrap_or_else(|error| unreachable!("{error}"));
    let deleted = transaction
        .execute(
            "DELETE FROM audit_reservations WHERE transaction_id = ?1",
            params![first.transaction_id.as_str()],
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(deleted, 1);
    assert_eq!(
        fs::metadata(&reserve)
            .unwrap_or_else(|error| unreachable!("{error}"))
            .len(),
        three_slot_len,
    );
    transaction
        .rollback()
        .unwrap_or_else(|error| unreachable!("{error}"));
    drop(raw);
    assert_eq!(
        fs::metadata(&reserve)
            .unwrap_or_else(|error| unreachable!("{error}"))
            .len(),
        three_slot_len,
    );

    // Both Prepared transactions remain recoverable. Each successful terminal
    // SQLite commit is followed by authenticated reserve reconciliation, so the
    // physical shrink occurs only after the corresponding durable row removal.
    let mut first_recovery =
        SqliteTransactionStore::open_for_terminal_recovery(&db.path, verifier(), integrity_key())
            .unwrap_or_else(|error| unreachable!("{error}"));
    let first_aborted = first_recovery
        .abort_prepared(&AbortRequest {
            transaction_id: first.transaction_id,
            expected_generation: first.current_generation,
            expected_state_version: first.state_version,
            reason_digest: digest_bytes("test", b"multi-first-abort"),
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(first_aborted.state, TransactionState::Aborted);
    let two_slot_len = fs::metadata(&reserve)
        .unwrap_or_else(|error| unreachable!("{error}"))
        .len();
    assert_eq!(two_slot_len, 2 * 1024 * 1024);
    drop(first_recovery);

    let mut second_recovery =
        SqliteTransactionStore::open_for_terminal_recovery(&db.path, verifier(), integrity_key())
            .unwrap_or_else(|error| unreachable!("{error}"));
    let second_aborted = second_recovery
        .abort_prepared(&AbortRequest {
            transaction_id: second.transaction_id,
            expected_generation: second.current_generation,
            expected_state_version: second.state_version,
            reason_digest: digest_bytes("test", b"multi-second-abort"),
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(second_aborted.state, TransactionState::Aborted);
    assert!(
        !reserve.exists()
            || fs::metadata(&reserve)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len()
                == 0
    );
}

#[test]
fn aggregate_schema_fingerprint_input_is_bounded_before_materialization() {
    let db = TestDatabase::new();
    drop(
        SqliteTransactionStore::open(&db.path, verifier(), integrity_key())
            .unwrap_or_else(|error| unreachable!("{error}")),
    );

    let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
    for index in 0..300_u64 {
        raw.execute_batch(&format!(
            "CREATE TABLE extra_{index} (value INTEGER) STRICT;"
        ))
        .unwrap_or_else(|error| unreachable!("{error}"));
    }
    drop(raw);

    assert!(matches!(
        SqliteTransactionStore::open(&db.path, verifier(), integrity_key()),
        Err(linura_transaction::TransactionStoreError::UnsupportedSchema(reason))
            if reason.contains("aggregate fingerprint validation bounds")
    ));
}
