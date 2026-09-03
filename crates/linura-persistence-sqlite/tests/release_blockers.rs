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
use rusqlite::Connection;

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

fn binding() -> AuthorityBinding {
    AuthorityBinding::try_new(
        PrincipalId::new("uid:1000").unwrap_or_else(|error| unreachable!("{error}")),
        RequestId::new("request:release-blockers").unwrap_or_else(|error| unreachable!("{error}")),
        PlanId::new("plan:release-blockers").unwrap_or_else(|error| unreachable!("{error}")),
        digest_bytes("test", b"request"),
        digest_bytes("test", b"precondition"),
        digest_bytes("test", b"observation"),
        ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}")),
        ResourceId::new("systemd:unit:test.service")
            .unwrap_or_else(|error| unreachable!("{error}")),
        CapabilityId::new("systemd.unit.observe").unwrap_or_else(|error| unreachable!("{error}")),
        PolicyId::new("policy:baseline").unwrap_or_else(|error| unreachable!("{error}")),
        PolicyRevisionId::new("policy:baseline:v1")
            .unwrap_or_else(|error| unreachable!("{error}")),
        RiskClass::SecuritySensitive,
        "risk-policy:v0.4:1",
        vec!["release-blocker-regression".into()],
        digest_bytes("test", b"review"),
        AuthorizationBasis::PolicyAllow,
    )
    .unwrap_or_else(|error| unreachable!("{error}"))
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

    let baseline = fs::metadata(&reserve)
        .unwrap_or_else(|error| unreachable!("{error}"))
        .len();
    assert!(baseline >= 256 * 1024);
    let snapshot = prepared(
        store
            .prepare(&binding())
            .unwrap_or_else(|error| unreachable!("{error}")),
    );
    let prepared_reserve = fs::metadata(&reserve)
        .unwrap_or_else(|error| unreachable!("{error}"))
        .len();
    assert_eq!(prepared_reserve, baseline * 2);

    let aborted = store
        .abort_prepared(&AbortRequest {
            transaction_id: snapshot.transaction_id.clone(),
            expected_generation: snapshot.current_generation,
            expected_state_version: snapshot.state_version,
            reason_digest: digest_bytes("test", b"abort"),
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(aborted.state, TransactionState::Aborted);
    assert_eq!(
        fs::metadata(&reserve)
            .unwrap_or_else(|error| unreachable!("{error}"))
            .len(),
        baseline
    );
    store
        .integrity_check()
        .unwrap_or_else(|error| unreachable!("{error}"));
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
