#![forbid(unsafe_code)]

//! Local SQLite/WAL persistence adapter for Linura durable authority transactions.
//!
//! SQLite stores durable authority facts; it is never treated as authoritative
//! current Linux state. All ambiguity after the pre-dispatch boundary remains
//! `Indeterminate` until Control supplies fresh authoritative recovery evidence.

use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::time::Duration;

use linura_core::{PrincipalId, RequestId};
use linura_transaction::{
    AbortRequest, AuthorityBinding, CommitRequest, ContentDigest, HandoffCommit, HandoffRequest,
    MAX_TRANSACTION_GENERATIONS, PrepareOutcome, RecoveryAnchor, RecoveryOutcome, RecoveryRequest,
    RecoveryResolution, TransactionId, TransactionSnapshot, TransactionState, TransactionStore,
    TransactionStoreError, digest_bytes, digest_parts,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

const APPLICATION_ID: i64 = 0x4c4e5254; // "LNRT"
const SCHEMA_VERSION: i64 = 1;
const MIGRATION_ID: &str = "0001-v04-authority-transactions";
const BUSY_TIMEOUT_MS: u64 = 5_000;

const MIGRATION_V1: &str = r#"
CREATE TABLE schema_migrations (
    migration_id TEXT PRIMARY KEY NOT NULL,
    checksum TEXT NOT NULL
) STRICT;

CREATE TABLE transactions (
    transaction_id TEXT PRIMARY KEY NOT NULL,
    principal TEXT NOT NULL,
    request_id TEXT NOT NULL,
    current_generation INTEGER NOT NULL CHECK (current_generation >= 0),
    state_version INTEGER NOT NULL CHECK (state_version >= 1),
    UNIQUE (principal, request_id)
) STRICT;

CREATE TABLE generations (
    transaction_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    state TEXT NOT NULL CHECK (state IN (
        'prepared', 'indeterminate', 'verified', 'committed', 'aborted', 'recovery-blocked'
    )),
    binding_digest TEXT NOT NULL,
    binding_canonical BLOB NOT NULL,
    request_digest TEXT NOT NULL,
    precondition_digest TEXT NOT NULL,
    observation_digest TEXT NOT NULL,
    desired_state_digest TEXT,
    graph_digest TEXT,
    provenance_digest TEXT,
    PRIMARY KEY (transaction_id, generation),
    FOREIGN KEY (transaction_id) REFERENCES transactions(transaction_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE audit_events (
    transaction_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK (event_sequence >= 0),
    generation INTEGER NOT NULL CHECK (generation >= 0),
    state_version INTEGER NOT NULL CHECK (state_version >= 1),
    event_kind TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    previous_digest TEXT NOT NULL,
    event_digest TEXT NOT NULL,
    PRIMARY KEY (transaction_id, event_sequence),
    FOREIGN KEY (transaction_id) REFERENCES transactions(transaction_id) ON DELETE RESTRICT
) STRICT;

CREATE TRIGGER generations_binding_immutable
BEFORE UPDATE OF binding_digest, binding_canonical, request_digest, precondition_digest, observation_digest ON generations
BEGIN
    SELECT RAISE(ABORT, 'immutable generation authority binding');
END;

CREATE TRIGGER generations_commit_provenance_guard
BEFORE UPDATE OF desired_state_digest, graph_digest, provenance_digest ON generations
WHEN NOT (
    OLD.state = 'verified' AND NEW.state = 'committed'
    AND OLD.desired_state_digest IS NULL AND OLD.graph_digest IS NULL AND OLD.provenance_digest IS NULL
    AND NEW.desired_state_digest IS NOT NULL AND NEW.graph_digest IS NOT NULL AND NEW.provenance_digest IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'immutable committed generation provenance');
END;

CREATE TRIGGER audit_events_no_update
BEFORE UPDATE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'append-only audit history');
END;

CREATE TRIGGER audit_events_no_delete
BEFORE DELETE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'append-only audit history');
END;

CREATE TRIGGER schema_migrations_no_update
BEFORE UPDATE ON schema_migrations
BEGIN
    SELECT RAISE(ABORT, 'immutable migration ledger');
END;

CREATE TRIGGER schema_migrations_no_delete
BEFORE DELETE ON schema_migrations
BEGIN
    SELECT RAISE(ABORT, 'immutable migration ledger');
END;
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreLimits {
    pub max_transactions: u64,
    pub max_generations: u64,
    pub max_audit_events: u64,
    pub max_pages: u64,
}

impl Default for StoreLimits {
    fn default() -> Self {
        Self {
            max_transactions: 4_096,
            max_generations: 16_384,
            max_audit_events: 131_072,
            max_pages: 262_144,
        }
    }
}

impl StoreLimits {
    fn validate(self) -> Result<Self, TransactionStoreError> {
        if self.max_transactions == 0
            || self.max_generations == 0
            || self.max_audit_events == 0
            || self.max_pages == 0
        {
            return Err(TransactionStoreError::CapacityExceeded);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteSettings {
    pub journal_mode: String,
    pub synchronous: i64,
    pub foreign_keys: i64,
    pub trusted_schema: i64,
    pub application_id: i64,
    pub user_version: i64,
    pub max_page_count: i64,
}

pub struct SqliteTransactionStore {
    connection: Connection,
    limits: StoreLimits,
}

impl Debug for SqliteTransactionStore {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqliteTransactionStore")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl SqliteTransactionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TransactionStoreError> {
        Self::open_with_limits(path, StoreLimits::default())
    }

    pub fn open_with_limits(
        path: impl AsRef<Path>,
        limits: StoreLimits,
    ) -> Result<Self, TransactionStoreError> {
        let limits = limits.validate()?;
        let path = path.as_ref();

        let mut connection = Connection::open(path).map_err(sqlite)?;
        connection
            .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))
            .map_err(sqlite)?;
        configure_connection(&connection, limits)?;
        initialize_or_validate_schema(&mut connection)?;

        let store = Self { connection, limits };
        store.integrity_check()?;
        Ok(store)
    }

    pub fn settings(&self) -> Result<SqliteSettings, TransactionStoreError> {
        Ok(SqliteSettings {
            journal_mode: pragma_string(&self.connection, "journal_mode")?,
            synchronous: pragma_i64(&self.connection, "synchronous")?,
            foreign_keys: pragma_i64(&self.connection, "foreign_keys")?,
            trusted_schema: pragma_i64(&self.connection, "trusted_schema")?,
            application_id: pragma_i64(&self.connection, "application_id")?,
            user_version: pragma_i64(&self.connection, "user_version")?,
            max_page_count: pragma_i64(&self.connection, "max_page_count")?,
        })
    }

    fn check_count_capacity(
        transaction: &Transaction<'_>,
        table: &str,
        maximum: u64,
        additional: u64,
    ) -> Result<(), TransactionStoreError> {
        let query = match table {
            "transactions" => "SELECT COUNT(*) FROM transactions",
            "generations" => "SELECT COUNT(*) FROM generations",
            "audit_events" => "SELECT COUNT(*) FROM audit_events",
            _ => {
                return Err(TransactionStoreError::Corruption(
                    "unknown capacity table".into(),
                ));
            }
        };
        let count: i64 = transaction
            .query_row(query, [], |row| row.get(0))
            .map_err(sqlite)?;
        let count = u64::try_from(count)
            .map_err(|_| TransactionStoreError::Corruption("negative row count".into()))?;
        if count.saturating_add(additional) > maximum {
            return Err(TransactionStoreError::CapacityExceeded);
        }
        Ok(())
    }

    fn append_audit(
        transaction: &Transaction<'_>,
        limits: StoreLimits,
        transaction_id: &TransactionId,
        generation: u64,
        state_version: u64,
        event_kind: &str,
        payload_digest: &ContentDigest,
    ) -> Result<ContentDigest, TransactionStoreError> {
        Self::check_count_capacity(transaction, "audit_events", limits.max_audit_events, 1)?;

        let previous: Option<(i64, String)> = transaction
            .query_row(
                "SELECT event_sequence, event_digest FROM audit_events \
                 WHERE transaction_id = ?1 ORDER BY event_sequence DESC LIMIT 1",
                params![transaction_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sqlite)?;
        let (sequence, previous_digest) = match previous {
            Some((sequence, digest)) => {
                let sequence = u64::try_from(sequence).map_err(|_| {
                    TransactionStoreError::Corruption("negative audit sequence".into())
                })?;
                (
                    sequence
                        .checked_add(1)
                        .ok_or(TransactionStoreError::CapacityExceeded)?,
                    ContentDigest::new(digest)
                        .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?,
                )
            }
            None => (0, ContentDigest::zero()),
        };

        let event_digest = audit_digest(
            transaction_id,
            sequence,
            generation,
            state_version,
            event_kind,
            payload_digest,
            &previous_digest,
        );
        transaction
            .execute(
                "INSERT INTO audit_events (
                    transaction_id, event_sequence, generation, state_version,
                    event_kind, payload_digest, previous_digest, event_digest
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    transaction_id.as_str(),
                    as_i64(sequence)?,
                    as_i64(generation)?,
                    as_i64(state_version)?,
                    event_kind,
                    payload_digest.as_str(),
                    previous_digest.as_str(),
                    event_digest.as_str(),
                ],
            )
            .map_err(sqlite)?;
        Ok(event_digest)
    }

    fn raw_snapshot(
        connection: &Connection,
        transaction_id: &TransactionId,
    ) -> Result<TransactionSnapshot, TransactionStoreError> {
        connection
            .query_row(
                "SELECT t.principal, t.request_id, t.current_generation, t.state_version,
                        g.state, g.binding_digest
                 FROM transactions t
                 JOIN generations g
                   ON g.transaction_id = t.transaction_id
                  AND g.generation = t.current_generation
                 WHERE t.transaction_id = ?1",
                params![transaction_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite)?
            .ok_or(TransactionStoreError::NotFound)
            .and_then(|row| snapshot_from_row(transaction_id.clone(), row))
    }

    fn assert_expected(
        snapshot: &TransactionSnapshot,
        expected_generation: u64,
        expected_state_version: u64,
        expected_binding_digest: &ContentDigest,
        expected_state: TransactionState,
    ) -> Result<(), TransactionStoreError> {
        if snapshot.current_generation != expected_generation
            || snapshot.state_version != expected_state_version
            || snapshot.binding_digest != *expected_binding_digest
            || snapshot.state != expected_state
        {
            return Err(TransactionStoreError::StateConflict);
        }
        Ok(())
    }
}

impl TransactionStore for SqliteTransactionStore {
    fn prepare(
        &mut self,
        binding: &AuthorityBinding,
    ) -> Result<PrepareOutcome, TransactionStoreError> {
        let transaction_id = binding.transaction_id();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;

        let existing = Self::raw_snapshot(&transaction, &transaction_id);
        match existing {
            Ok(snapshot) => {
                if snapshot.state == TransactionState::Prepared
                    && snapshot.binding_digest == *binding.digest()
                    && snapshot.principal == *binding.principal()
                    && snapshot.request_id == *binding.request_id()
                {
                    transaction.commit().map_err(sqlite)?;
                    return Ok(PrepareOutcome::Existing(snapshot));
                }
                return Err(TransactionStoreError::IdempotencyConflict);
            }
            Err(TransactionStoreError::NotFound) => {}
            Err(error) => return Err(error),
        }

        Self::check_count_capacity(
            &transaction,
            "transactions",
            self.limits.max_transactions,
            1,
        )?;
        Self::check_count_capacity(&transaction, "generations", self.limits.max_generations, 1)?;

        transaction
            .execute(
                "INSERT INTO transactions (
                    transaction_id, principal, request_id, current_generation, state_version
                 ) VALUES (?1, ?2, ?3, 0, 1)",
                params![
                    transaction_id.as_str(),
                    binding.principal().as_str(),
                    binding.request_id().as_str(),
                ],
            )
            .map_err(sqlite)?;
        transaction
            .execute(
                "INSERT INTO generations (
                    transaction_id, generation, state, binding_digest,
                    binding_canonical, request_digest, precondition_digest, observation_digest
                 ) VALUES (?1, 0, 'prepared', ?2, ?3, ?4, ?5, ?6)",
                params![
                    transaction_id.as_str(),
                    binding.digest().as_str(),
                    binding.canonical_bytes(),
                    binding.request_digest().as_str(),
                    binding.precondition_digest().as_str(),
                    binding.observation_digest().as_str(),
                ],
            )
            .map_err(sqlite)?;
        Self::append_audit(
            &transaction,
            self.limits,
            &transaction_id,
            0,
            1,
            "prepared",
            binding.digest(),
        )?;
        transaction.commit().map_err(sqlite)?;

        Ok(PrepareOutcome::Created(TransactionSnapshot {
            transaction_id,
            principal: binding.principal().clone(),
            request_id: binding.request_id().clone(),
            current_generation: 0,
            state_version: 1,
            state: TransactionState::Prepared,
            binding_digest: binding.digest().clone(),
        }))
    }

    fn handoff(
        &mut self,
        request: &HandoffRequest,
    ) -> Result<HandoffCommit, TransactionStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let snapshot = Self::raw_snapshot(&transaction, &request.transaction_id)?;
        Self::assert_expected(
            &snapshot,
            request.expected_generation,
            request.expected_state_version,
            &request.expected_binding_digest,
            TransactionState::Prepared,
        )?;
        if request.authority_use_digest == ContentDigest::zero() {
            return Err(TransactionStoreError::StateConflict);
        }
        let next_version = snapshot
            .state_version
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        let updated = transaction
            .execute(
                "UPDATE generations SET state = 'indeterminate'
                 WHERE transaction_id = ?1 AND generation = ?2 AND state = 'prepared'",
                params![
                    request.transaction_id.as_str(),
                    as_i64(snapshot.current_generation)?
                ],
            )
            .map_err(sqlite)?;
        if updated != 1 {
            return Err(TransactionStoreError::StateConflict);
        }
        let updated = transaction
            .execute(
                "UPDATE transactions SET state_version = ?2
                 WHERE transaction_id = ?1 AND current_generation = ?3 AND state_version = ?4",
                params![
                    request.transaction_id.as_str(),
                    as_i64(next_version)?,
                    as_i64(snapshot.current_generation)?,
                    as_i64(snapshot.state_version)?,
                ],
            )
            .map_err(sqlite)?;
        if updated != 1 {
            return Err(TransactionStoreError::StateConflict);
        }
        Self::append_audit(
            &transaction,
            self.limits,
            &request.transaction_id,
            snapshot.current_generation,
            next_version,
            "handoff-indeterminate",
            &request.authority_use_digest,
        )?;
        transaction.commit().map_err(sqlite)?;
        Ok(HandoffCommit {
            transaction_id: request.transaction_id.clone(),
            generation: snapshot.current_generation,
            state_version: next_version,
            binding_digest: snapshot.binding_digest,
            authority_use_digest: request.authority_use_digest.clone(),
        })
    }

    fn recover(
        &mut self,
        request: &RecoveryRequest,
    ) -> Result<RecoveryOutcome, TransactionStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let snapshot = Self::raw_snapshot(&transaction, &request.transaction_id)?;
        Self::assert_expected(
            &snapshot,
            request.expected_generation,
            request.expected_state_version,
            &request.expected_binding_digest,
            TransactionState::Indeterminate,
        )?;
        let next_version = snapshot
            .state_version
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;

        let outcome = match &request.resolution {
            RecoveryResolution::IntendedStateVerified { observation_digest } => {
                transition_generation_state(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    TransactionState::Indeterminate,
                    TransactionState::Verified,
                )?;
                update_transaction_pointer(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    snapshot.state_version,
                    snapshot.current_generation,
                    next_version,
                )?;
                Self::append_audit(
                    &transaction,
                    self.limits,
                    &request.transaction_id,
                    snapshot.current_generation,
                    next_version,
                    "recovery-verified",
                    observation_digest,
                )?;
                RecoveryOutcome::Verified(TransactionSnapshot {
                    state: TransactionState::Verified,
                    state_version: next_version,
                    ..snapshot.clone()
                })
            }
            RecoveryResolution::ConflictingState { observation_digest } => {
                transition_generation_state(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    TransactionState::Indeterminate,
                    TransactionState::RecoveryBlocked,
                )?;
                update_transaction_pointer(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    snapshot.state_version,
                    snapshot.current_generation,
                    next_version,
                )?;
                Self::append_audit(
                    &transaction,
                    self.limits,
                    &request.transaction_id,
                    snapshot.current_generation,
                    next_version,
                    "recovery-blocked",
                    observation_digest,
                )?;
                RecoveryOutcome::Blocked(TransactionSnapshot {
                    state: TransactionState::RecoveryBlocked,
                    state_version: next_version,
                    ..snapshot.clone()
                })
            }
            RecoveryResolution::Ambiguous { observation_digest } => {
                update_transaction_pointer(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    snapshot.state_version,
                    snapshot.current_generation,
                    next_version,
                )?;
                Self::append_audit(
                    &transaction,
                    self.limits,
                    &request.transaction_id,
                    snapshot.current_generation,
                    next_version,
                    "recovery-ambiguous",
                    observation_digest,
                )?;
                RecoveryOutcome::StillIndeterminate(TransactionSnapshot {
                    state_version: next_version,
                    ..snapshot.clone()
                })
            }
            RecoveryResolution::IntendedEffectAbsent {
                observation_digest,
                next_binding,
            } => {
                if next_binding.transaction_id() != request.transaction_id {
                    return Err(TransactionStoreError::IdempotencyConflict);
                }
                let next_generation = snapshot
                    .current_generation
                    .checked_add(1)
                    .ok_or(TransactionStoreError::CapacityExceeded)?;
                if next_generation >= MAX_TRANSACTION_GENERATIONS {
                    return Err(TransactionStoreError::CapacityExceeded);
                }
                Self::check_count_capacity(
                    &transaction,
                    "generations",
                    self.limits.max_generations,
                    1,
                )?;
                Self::check_count_capacity(
                    &transaction,
                    "audit_events",
                    self.limits.max_audit_events,
                    2,
                )?;

                transition_generation_state(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    TransactionState::Indeterminate,
                    TransactionState::Aborted,
                )?;
                transaction
                    .execute(
                        "INSERT INTO generations (
                            transaction_id, generation, state, binding_digest,
                            binding_canonical, request_digest, precondition_digest, observation_digest
                         ) VALUES (?1, ?2, 'prepared', ?3, ?4, ?5, ?6, ?7)",
                        params![
                            request.transaction_id.as_str(),
                            as_i64(next_generation)?,
                            next_binding.digest().as_str(),
                            next_binding.canonical_bytes(),
                            next_binding.request_digest().as_str(),
                            next_binding.precondition_digest().as_str(),
                            next_binding.observation_digest().as_str(),
                        ],
                    )
                    .map_err(sqlite)?;
                update_transaction_pointer(
                    &transaction,
                    &request.transaction_id,
                    snapshot.current_generation,
                    snapshot.state_version,
                    next_generation,
                    next_version,
                )?;
                Self::append_audit(
                    &transaction,
                    self.limits,
                    &request.transaction_id,
                    snapshot.current_generation,
                    next_version,
                    "recovery-retired-no-effect",
                    observation_digest,
                )?;
                Self::append_audit(
                    &transaction,
                    self.limits,
                    &request.transaction_id,
                    next_generation,
                    next_version,
                    "recovery-reprepared",
                    next_binding.digest(),
                )?;
                RecoveryOutcome::Reprepared(TransactionSnapshot {
                    transaction_id: request.transaction_id.clone(),
                    principal: next_binding.principal().clone(),
                    request_id: next_binding.request_id().clone(),
                    current_generation: next_generation,
                    state_version: next_version,
                    state: TransactionState::Prepared,
                    binding_digest: next_binding.digest().clone(),
                })
            }
        };
        transaction.commit().map_err(sqlite)?;
        Ok(outcome)
    }

    fn commit(
        &mut self,
        request: &CommitRequest,
    ) -> Result<TransactionSnapshot, TransactionStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let snapshot = Self::raw_snapshot(&transaction, &request.transaction_id)?;
        if snapshot.current_generation != request.expected_generation
            || snapshot.state_version != request.expected_state_version
            || snapshot.state != TransactionState::Verified
        {
            return Err(TransactionStoreError::StateConflict);
        }
        let next_version = snapshot
            .state_version
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        let updated = transaction
            .execute(
                "UPDATE generations
                 SET state = 'committed', desired_state_digest = ?3,
                     graph_digest = ?4, provenance_digest = ?5
                 WHERE transaction_id = ?1 AND generation = ?2 AND state = 'verified'",
                params![
                    request.transaction_id.as_str(),
                    as_i64(snapshot.current_generation)?,
                    request.desired_state_digest.as_str(),
                    request.graph_digest.as_str(),
                    request.provenance_digest.as_str(),
                ],
            )
            .map_err(sqlite)?;
        if updated != 1 {
            return Err(TransactionStoreError::StateConflict);
        }
        update_transaction_pointer(
            &transaction,
            &request.transaction_id,
            snapshot.current_generation,
            snapshot.state_version,
            snapshot.current_generation,
            next_version,
        )?;
        let payload = digest_parts(
            "linura.sqlite.commit-payload.v1",
            [
                request.desired_state_digest.as_str().as_bytes(),
                request.graph_digest.as_str().as_bytes(),
                request.provenance_digest.as_str().as_bytes(),
            ],
        );
        Self::append_audit(
            &transaction,
            self.limits,
            &request.transaction_id,
            snapshot.current_generation,
            next_version,
            "committed",
            &payload,
        )?;
        transaction.commit().map_err(sqlite)?;
        Ok(TransactionSnapshot {
            state: TransactionState::Committed,
            state_version: next_version,
            ..snapshot
        })
    }

    fn abort_prepared(
        &mut self,
        request: &AbortRequest,
    ) -> Result<TransactionSnapshot, TransactionStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let snapshot = Self::raw_snapshot(&transaction, &request.transaction_id)?;
        if snapshot.current_generation != request.expected_generation
            || snapshot.state_version != request.expected_state_version
            || snapshot.state != TransactionState::Prepared
        {
            return Err(TransactionStoreError::StateConflict);
        }
        let next_version = snapshot
            .state_version
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        transition_generation_state(
            &transaction,
            &request.transaction_id,
            snapshot.current_generation,
            TransactionState::Prepared,
            TransactionState::Aborted,
        )?;
        update_transaction_pointer(
            &transaction,
            &request.transaction_id,
            snapshot.current_generation,
            snapshot.state_version,
            snapshot.current_generation,
            next_version,
        )?;
        Self::append_audit(
            &transaction,
            self.limits,
            &request.transaction_id,
            snapshot.current_generation,
            next_version,
            "aborted-prepared",
            &request.reason_digest,
        )?;
        transaction.commit().map_err(sqlite)?;
        Ok(TransactionSnapshot {
            state: TransactionState::Aborted,
            state_version: next_version,
            ..snapshot
        })
    }

    fn snapshot(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<TransactionSnapshot, TransactionStoreError> {
        Self::raw_snapshot(&self.connection, transaction_id)
    }

    fn recovery_anchor(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<RecoveryAnchor, TransactionStoreError> {
        self.connection
            .query_row(
                "SELECT t.principal, t.request_id, t.current_generation, t.state_version,
                        g.state, g.binding_digest, g.request_digest, g.precondition_digest
                 FROM transactions t
                 JOIN generations g
                   ON g.transaction_id = t.transaction_id
                  AND g.generation = t.current_generation
                 WHERE t.transaction_id = ?1",
                params![transaction_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite)?
            .ok_or(TransactionStoreError::NotFound)
            .and_then(|row| {
                let snapshot = snapshot_from_row(
                    transaction_id.clone(),
                    (row.0, row.1, row.2, row.3, row.4, row.5),
                )?;
                let request_digest = ContentDigest::new(row.6)
                    .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
                let precondition_digest = ContentDigest::new(row.7)
                    .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
                Ok(RecoveryAnchor {
                    snapshot,
                    request_digest,
                    precondition_digest,
                })
            })
    }

    fn list_state(
        &self,
        state: TransactionState,
    ) -> Result<Vec<TransactionSnapshot>, TransactionStoreError> {
        let mut statement = self.connection.prepare(
            "SELECT t.transaction_id, t.principal, t.request_id, t.current_generation, t.state_version,
                    g.state, g.binding_digest
             FROM transactions t
             JOIN generations g
               ON g.transaction_id = t.transaction_id
              AND g.generation = t.current_generation
             WHERE g.state = ?1
             ORDER BY t.transaction_id",
        ).map_err(sqlite)?;
        let rows = statement
            .query_map(params![state.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(sqlite)?;
        let mut snapshots = Vec::new();
        for row in rows {
            let row = row.map_err(sqlite)?;
            let transaction_id = TransactionId::new(row.0)
                .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
            snapshots.push(snapshot_from_row(
                transaction_id,
                (row.1, row.2, row.3, row.4, row.5, row.6),
            )?);
        }
        Ok(snapshots)
    }

    fn integrity_check(&self) -> Result<(), TransactionStoreError> {
        validate_runtime_settings(&self.connection, self.limits)?;
        validate_schema_identity(&self.connection)?;

        let integrity: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(sqlite)?;
        if integrity != "ok" {
            return Err(TransactionStoreError::Corruption(format!(
                "SQLite integrity_check returned {integrity:?}"
            )));
        }
        let mut foreign = self
            .connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(sqlite)?;
        if foreign
            .query([])
            .map_err(sqlite)?
            .next()
            .map_err(sqlite)?
            .is_some()
        {
            return Err(TransactionStoreError::Corruption(
                "SQLite foreign_key_check reported violations".into(),
            ));
        }

        let mut transactions = self
            .connection
            .prepare(
                "SELECT transaction_id, principal, request_id, current_generation, state_version
                 FROM transactions ORDER BY transaction_id",
            )
            .map_err(sqlite)?;
        let rows = transactions
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(sqlite)?;
        for row in rows {
            let (transaction_id, principal, request_id, current_generation, state_version) =
                row.map_err(sqlite)?;
            let transaction_id = TransactionId::new(transaction_id)
                .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
            let principal = PrincipalId::new(principal)
                .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
            let request_id = RequestId::new(request_id)
                .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
            if TransactionId::for_namespace(&principal, &request_id) != transaction_id {
                return Err(TransactionStoreError::Corruption(
                    "transaction identity does not match principal/request namespace".into(),
                ));
            }
            let current_generation = u64::try_from(current_generation).map_err(|_| {
                TransactionStoreError::Corruption("negative current generation".into())
            })?;
            let state_version = u64::try_from(state_version)
                .map_err(|_| TransactionStoreError::Corruption("negative state version".into()))?;
            validate_generations(
                &self.connection,
                &transaction_id,
                current_generation,
                self.limits,
            )?;
            validate_audit_chain(
                &self.connection,
                &transaction_id,
                current_generation,
                state_version,
                self.limits,
            )?;
        }
        Ok(())
    }
}

fn configure_connection(
    connection: &Connection,
    limits: StoreLimits,
) -> Result<(), TransactionStoreError> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL;",
        )
        .map_err(sqlite)?;
    let mode: String = connection
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .map_err(sqlite)?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(TransactionStoreError::UnsupportedSchema(format!(
            "SQLite WAL mode unavailable: {mode}"
        )));
    }
    connection
        .pragma_update(None, "max_page_count", limits.max_pages)
        .map_err(sqlite)?;
    validate_runtime_settings(connection, limits)
}

fn initialize_or_validate_schema(connection: &mut Connection) -> Result<(), TransactionStoreError> {
    let application_id = pragma_i64(connection, "application_id")?;
    let user_version = pragma_i64(connection, "user_version")?;
    if application_id == 0 && user_version == 0 {
        let object_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        if object_count != 0 {
            return Err(TransactionStoreError::UnsupportedSchema(
                "unidentified authority database contains application schema objects".into(),
            ));
        }
        let migration = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        migration
            .pragma_update(None, "application_id", APPLICATION_ID)
            .map_err(sqlite)?;
        migration.execute_batch(MIGRATION_V1).map_err(sqlite)?;
        let checksum = migration_checksum();
        migration
            .execute(
                "INSERT INTO schema_migrations (migration_id, checksum) VALUES (?1, ?2)",
                params![MIGRATION_ID, checksum.as_str()],
            )
            .map_err(sqlite)?;
        migration
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(sqlite)?;
        migration.commit().map_err(sqlite)?;
    } else {
        if application_id != APPLICATION_ID {
            return Err(TransactionStoreError::UnsupportedSchema(format!(
                "application_id {application_id} is not Linura authority storage"
            )));
        }
        if user_version > SCHEMA_VERSION {
            return Err(TransactionStoreError::UnsupportedSchema(format!(
                "database schema {user_version} is newer than supported {SCHEMA_VERSION}"
            )));
        }
        if user_version != SCHEMA_VERSION {
            return Err(TransactionStoreError::UnsupportedSchema(format!(
                "database schema {user_version} requires an unavailable migration"
            )));
        }
    }
    validate_schema_identity(connection)
}

fn validate_runtime_settings(
    connection: &Connection,
    limits: StoreLimits,
) -> Result<(), TransactionStoreError> {
    let mode = pragma_string(connection, "journal_mode")?.to_ascii_lowercase();
    if mode != "wal" {
        return Err(TransactionStoreError::UnsupportedSchema(format!(
            "authority database journal mode must be WAL, found {mode}"
        )));
    }
    if pragma_i64(connection, "synchronous")? != 2 {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database synchronous mode must be FULL".into(),
        ));
    }
    if pragma_i64(connection, "foreign_keys")? != 1 {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database foreign keys must be enabled".into(),
        ));
    }
    if pragma_i64(connection, "trusted_schema")? != 0 {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database trusted_schema must be disabled".into(),
        ));
    }
    let max_pages = pragma_i64(connection, "max_page_count")?;
    if max_pages <= 0
        || u64::try_from(max_pages).map_err(|_| TransactionStoreError::CapacityExceeded)?
            > limits.max_pages
    {
        return Err(TransactionStoreError::CapacityExceeded);
    }
    Ok(())
}

fn validate_schema_identity(connection: &Connection) -> Result<(), TransactionStoreError> {
    if pragma_i64(connection, "application_id")? != APPLICATION_ID {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database application_id mismatch".into(),
        ));
    }
    if pragma_i64(connection, "user_version")? != SCHEMA_VERSION {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database user_version mismatch".into(),
        ));
    }
    let checksum: String = connection
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE migration_id = ?1",
            params![MIGRATION_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(sqlite)?
        .ok_or_else(|| {
            TransactionStoreError::UnsupportedSchema("migration ledger entry missing".into())
        })?;
    if checksum != migration_checksum().as_str() {
        return Err(TransactionStoreError::UnsupportedSchema(
            "migration checksum mismatch".into(),
        ));
    }
    if schema_fingerprint(connection)? != expected_schema_fingerprint()? {
        return Err(TransactionStoreError::UnsupportedSchema(
            "installed authority schema objects differ from the canonical migration".into(),
        ));
    }
    Ok(())
}

fn schema_fingerprint(connection: &Connection) -> Result<ContentDigest, TransactionStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT type, name, tbl_name, COALESCE(sql, '')
             FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%'
             ORDER BY type, name",
        )
        .map_err(sqlite)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(sqlite)?;
    let mut encoded = Vec::new();
    for row in rows {
        let (kind, name, table, sql) = row.map_err(sqlite)?;
        for value in [&kind, &name, &table, &sql] {
            let length =
                u64::try_from(value.len()).map_err(|_| TransactionStoreError::CapacityExceeded)?;
            encoded.extend_from_slice(&length.to_be_bytes());
            encoded.extend_from_slice(value.as_bytes());
        }
    }
    Ok(digest_bytes("linura.sqlite.schema-objects.v1", &encoded))
}

fn expected_schema_fingerprint() -> Result<ContentDigest, TransactionStoreError> {
    let reference = Connection::open_in_memory().map_err(sqlite)?;
    reference.execute_batch(MIGRATION_V1).map_err(sqlite)?;
    schema_fingerprint(&reference)
}

fn validate_generations(
    connection: &Connection,
    transaction_id: &TransactionId,
    current_generation: u64,
    limits: StoreLimits,
) -> Result<(), TransactionStoreError> {
    if current_generation >= MAX_TRANSACTION_GENERATIONS {
        return Err(TransactionStoreError::Corruption(
            "current generation exceeds domain bound".into(),
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT generation, state, binding_digest, binding_canonical,
                    request_digest, precondition_digest, observation_digest,
                    desired_state_digest, graph_digest, provenance_digest
             FROM generations WHERE transaction_id = ?1 ORDER BY generation",
        )
        .map_err(sqlite)?;
    let rows = statement
        .query_map(params![transaction_id.as_str()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(sqlite)?;
    let mut expected = 0_u64;
    let mut current_state = None;
    for row in rows {
        let (
            generation,
            state,
            binding_digest,
            canonical,
            request_digest,
            precondition_digest,
            observation_digest,
            desired_state_digest,
            graph_digest,
            provenance_digest,
        ) = row.map_err(sqlite)?;
        let generation = u64::try_from(generation)
            .map_err(|_| TransactionStoreError::Corruption("negative generation".into()))?;
        if generation != expected {
            return Err(TransactionStoreError::Corruption(
                "generation history is not contiguous".into(),
            ));
        }
        let state = TransactionState::parse(&state)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        let binding_digest = ContentDigest::new(binding_digest)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        ContentDigest::new(request_digest)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        ContentDigest::new(precondition_digest)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        ContentDigest::new(observation_digest)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        if digest_bytes("linura.authority-binding.digest.v1", &canonical) != binding_digest {
            return Err(TransactionStoreError::Corruption(
                "stored authority binding digest mismatch".into(),
            ));
        }
        let commit_digests = [
            desired_state_digest.as_deref(),
            graph_digest.as_deref(),
            provenance_digest.as_deref(),
        ];
        if state == TransactionState::Committed {
            if commit_digests.iter().any(|value| value.is_none()) {
                return Err(TransactionStoreError::Corruption(
                    "committed generation is missing commit provenance digests".into(),
                ));
            }
            for value in commit_digests.into_iter().flatten() {
                ContentDigest::new(value.to_owned())
                    .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
            }
        } else if commit_digests.iter().any(|value| value.is_some()) {
            return Err(TransactionStoreError::Corruption(
                "non-committed generation contains commit provenance digests".into(),
            ));
        }
        if generation < current_generation && state != TransactionState::Aborted {
            return Err(TransactionStoreError::Corruption(
                "historical generation is not safely retired".into(),
            ));
        }
        if generation == current_generation {
            current_state = Some(state);
        }
        expected = expected
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        if expected > limits.max_generations {
            return Err(TransactionStoreError::CapacityExceeded);
        }
    }
    if expected != current_generation.saturating_add(1) || current_state.is_none() {
        return Err(TransactionStoreError::Corruption(
            "current generation pointer is inconsistent".into(),
        ));
    }
    Ok(())
}

fn validate_audit_chain(
    connection: &Connection,
    transaction_id: &TransactionId,
    current_generation: u64,
    state_version: u64,
    limits: StoreLimits,
) -> Result<(), TransactionStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT event_sequence, generation, state_version, event_kind,
                    payload_digest, previous_digest, event_digest
             FROM audit_events WHERE transaction_id = ?1 ORDER BY event_sequence",
        )
        .map_err(sqlite)?;
    let rows = statement
        .query_map(params![transaction_id.as_str()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(sqlite)?;
    let mut expected_sequence = 0_u64;
    let mut previous = ContentDigest::zero();
    let mut last = None;
    for row in rows {
        let (sequence, generation, version, kind, payload, stored_previous, stored_digest) =
            row.map_err(sqlite)?;
        let sequence = u64::try_from(sequence)
            .map_err(|_| TransactionStoreError::Corruption("negative audit sequence".into()))?;
        let generation = u64::try_from(generation)
            .map_err(|_| TransactionStoreError::Corruption("negative audit generation".into()))?;
        let version = u64::try_from(version).map_err(|_| {
            TransactionStoreError::Corruption("negative audit state version".into())
        })?;
        if sequence != expected_sequence {
            return Err(TransactionStoreError::Corruption(
                "audit sequence is not contiguous".into(),
            ));
        }
        let payload = ContentDigest::new(payload)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        let stored_previous = ContentDigest::new(stored_previous)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        let stored_digest = ContentDigest::new(stored_digest)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        if stored_previous != previous {
            return Err(TransactionStoreError::Corruption(
                "audit previous-digest chain is broken".into(),
            ));
        }
        let expected_digest = audit_digest(
            transaction_id,
            sequence,
            generation,
            version,
            &kind,
            &payload,
            &previous,
        );
        if expected_digest != stored_digest {
            return Err(TransactionStoreError::Corruption(
                "audit event digest mismatch".into(),
            ));
        }
        previous = stored_digest;
        last = Some((generation, version, kind, payload));
        expected_sequence = expected_sequence
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        if expected_sequence > limits.max_audit_events {
            return Err(TransactionStoreError::CapacityExceeded);
        }
    }
    let (last_generation, last_version, last_kind, last_payload) = last.ok_or_else(|| {
        TransactionStoreError::Corruption("transaction has no audit history".into())
    })?;
    if last_generation != current_generation || last_version != state_version {
        return Err(TransactionStoreError::Corruption(
            "transaction pointer/version disagrees with terminal audit event".into(),
        ));
    }
    let snapshot = SqliteTransactionStore::raw_snapshot(connection, transaction_id)?;
    if state_for_event_kind(&last_kind)? != snapshot.state {
        return Err(TransactionStoreError::Corruption(
            "current transaction state disagrees with terminal audit event".into(),
        ));
    }
    if snapshot.state == TransactionState::Committed {
        let (desired, graph, provenance): (String, String, String) = connection
            .query_row(
                "SELECT desired_state_digest, graph_digest, provenance_digest
                 FROM generations WHERE transaction_id = ?1 AND generation = ?2",
                params![transaction_id.as_str(), as_i64(current_generation)?],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(sqlite)?;
        let desired = ContentDigest::new(desired)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        let graph = ContentDigest::new(graph)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        let provenance = ContentDigest::new(provenance)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?;
        let expected_payload = digest_parts(
            "linura.sqlite.commit-payload.v1",
            [
                desired.as_str().as_bytes(),
                graph.as_str().as_bytes(),
                provenance.as_str().as_bytes(),
            ],
        );
        if last_payload != expected_payload {
            return Err(TransactionStoreError::Corruption(
                "committed provenance digests disagree with terminal audit payload".into(),
            ));
        }
    }
    Ok(())
}

fn state_for_event_kind(kind: &str) -> Result<TransactionState, TransactionStoreError> {
    match kind {
        "prepared" | "recovery-reprepared" => Ok(TransactionState::Prepared),
        "handoff-indeterminate" | "recovery-ambiguous" => Ok(TransactionState::Indeterminate),
        "recovery-verified" => Ok(TransactionState::Verified),
        "committed" => Ok(TransactionState::Committed),
        "aborted-prepared" | "recovery-retired-no-effect" => Ok(TransactionState::Aborted),
        "recovery-blocked" => Ok(TransactionState::RecoveryBlocked),
        _ => Err(TransactionStoreError::Corruption(format!(
            "unknown audit event kind {kind:?}"
        ))),
    }
}

fn transition_generation_state(
    transaction: &Transaction<'_>,
    transaction_id: &TransactionId,
    generation: u64,
    from: TransactionState,
    to: TransactionState,
) -> Result<(), TransactionStoreError> {
    let updated = transaction
        .execute(
            "UPDATE generations SET state = ?4
             WHERE transaction_id = ?1 AND generation = ?2 AND state = ?3",
            params![
                transaction_id.as_str(),
                as_i64(generation)?,
                from.as_str(),
                to.as_str(),
            ],
        )
        .map_err(sqlite)?;
    if updated != 1 {
        return Err(TransactionStoreError::StateConflict);
    }
    Ok(())
}

fn update_transaction_pointer(
    transaction: &Transaction<'_>,
    transaction_id: &TransactionId,
    expected_generation: u64,
    expected_version: u64,
    next_generation: u64,
    next_version: u64,
) -> Result<(), TransactionStoreError> {
    let updated = transaction
        .execute(
            "UPDATE transactions
             SET current_generation = ?4, state_version = ?5
             WHERE transaction_id = ?1 AND current_generation = ?2 AND state_version = ?3",
            params![
                transaction_id.as_str(),
                as_i64(expected_generation)?,
                as_i64(expected_version)?,
                as_i64(next_generation)?,
                as_i64(next_version)?,
            ],
        )
        .map_err(sqlite)?;
    if updated != 1 {
        return Err(TransactionStoreError::StateConflict);
    }
    Ok(())
}

fn snapshot_from_row(
    transaction_id: TransactionId,
    row: (String, String, i64, i64, String, String),
) -> Result<TransactionSnapshot, TransactionStoreError> {
    let (principal, request_id, generation, state_version, state, binding_digest) = row;
    Ok(TransactionSnapshot {
        transaction_id,
        principal: PrincipalId::new(principal)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?,
        request_id: RequestId::new(request_id)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?,
        current_generation: u64::try_from(generation)
            .map_err(|_| TransactionStoreError::Corruption("negative generation".into()))?,
        state_version: u64::try_from(state_version)
            .map_err(|_| TransactionStoreError::Corruption("negative state version".into()))?,
        state: TransactionState::parse(&state)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?,
        binding_digest: ContentDigest::new(binding_digest)
            .map_err(|error| TransactionStoreError::Corruption(error.to_string()))?,
    })
}

fn audit_digest(
    transaction_id: &TransactionId,
    sequence: u64,
    generation: u64,
    state_version: u64,
    event_kind: &str,
    payload_digest: &ContentDigest,
    previous_digest: &ContentDigest,
) -> ContentDigest {
    let sequence = sequence.to_string();
    let generation = generation.to_string();
    let state_version = state_version.to_string();
    digest_parts(
        "linura.sqlite.audit-event.v1",
        [
            transaction_id.as_str().as_bytes(),
            sequence.as_bytes(),
            generation.as_bytes(),
            state_version.as_bytes(),
            event_kind.as_bytes(),
            payload_digest.as_str().as_bytes(),
            previous_digest.as_str().as_bytes(),
        ],
    )
}

fn migration_checksum() -> ContentDigest {
    digest_bytes("linura.sqlite.migration.v1", MIGRATION_V1.as_bytes())
}

fn pragma_i64(connection: &Connection, name: &str) -> Result<i64, TransactionStoreError> {
    connection
        .pragma_query_value(None, name, |row| row.get(0))
        .map_err(sqlite)
}

fn pragma_string(connection: &Connection, name: &str) -> Result<String, TransactionStoreError> {
    connection
        .pragma_query_value(None, name, |row| row.get(0))
        .map_err(sqlite)
}

fn as_i64(value: u64) -> Result<i64, TransactionStoreError> {
    i64::try_from(value).map_err(|_| TransactionStoreError::CapacityExceeded)
}

fn sqlite(error: rusqlite::Error) -> TransactionStoreError {
    TransactionStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use linura_core::{
        CapabilityId, PlanId, PolicyId, PolicyRevisionId, ProviderId, ResourceId, RiskClass,
        ValidationError,
    };
    use linura_transaction::{AuthorizationBasis, TransactionValidationError};

    use super::*;

    static NEXT_DB: AtomicU64 = AtomicU64::new(0);

    struct TestDatabase {
        path: PathBuf,
    }

    impl TestDatabase {
        fn new() -> Self {
            let sequence = NEXT_DB.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "linura-v04-sqlite-{}-{sequence}.db",
                std::process::id()
            ));
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(path.with_extension("db-wal"));
            let _ = fs::remove_file(path.with_extension("db-shm"));
            Self { path }
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
            let wal = PathBuf::from(format!("{}-wal", self.path.display()));
            let shm = PathBuf::from(format!("{}-shm", self.path.display()));
            let _ = fs::remove_file(wal);
            let _ = fs::remove_file(shm);
        }
    }

    fn id<T>(value: Result<T, ValidationError>) -> T {
        value.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn digest(value: &str) -> ContentDigest {
        digest_bytes("test", value.as_bytes())
    }

    fn binding_with_observation(observation: &str) -> AuthorityBinding {
        AuthorityBinding::try_new(
            id(PrincipalId::new("uid:1000")),
            id(RequestId::new("request:sqlite")),
            id(PlanId::new("request:sqlite")),
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

    fn prepared_snapshot(outcome: PrepareOutcome) -> TransactionSnapshot {
        match outcome {
            PrepareOutcome::Created(snapshot) | PrepareOutcome::Existing(snapshot) => snapshot,
        }
    }

    #[test]
    fn qualified_sqlite_settings_are_enforced() {
        let db = TestDatabase::new();
        let store =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        let settings = store
            .settings()
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(settings.journal_mode.to_ascii_lowercase(), "wal");
        assert_eq!(settings.synchronous, 2);
        assert_eq!(settings.foreign_keys, 1);
        assert_eq!(settings.trusted_schema, 0);
        assert_eq!(settings.application_id, APPLICATION_ID);
        assert_eq!(settings.user_version, SCHEMA_VERSION);
        assert!(settings.max_page_count > 0);
        assert!(
            u64::try_from(settings.max_page_count).unwrap_or_default()
                <= StoreLimits::default().max_pages
        );
    }

    #[test]
    fn exact_prepare_replay_survives_reopen_and_changed_binding_conflicts() {
        let db = TestDatabase::new();
        let binding = binding_with_observation("observation-a");
        let first = {
            let mut store = SqliteTransactionStore::open(&db.path)
                .unwrap_or_else(|error| unreachable!("{error}"));
            prepared_snapshot(
                store
                    .prepare(&binding)
                    .unwrap_or_else(|error| unreachable!("{error}")),
            )
        };
        let mut reopened =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
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
    fn handoff_is_single_winner_and_indeterminate_survives_reopen() {
        let db = TestDatabase::new();
        let binding = binding_with_observation("observation-a");
        let mut store =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        let prepared = prepared_snapshot(
            store
                .prepare(&binding)
                .unwrap_or_else(|error| unreachable!("{error}")),
        );
        let request = HandoffRequest {
            transaction_id: prepared.transaction_id.clone(),
            expected_generation: prepared.current_generation,
            expected_state_version: prepared.state_version,
            expected_binding_digest: prepared.binding_digest.clone(),
            authority_use_digest: digest("authority-use"),
        };
        let commit = store
            .handoff(&request)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(commit.generation, 0);
        assert!(matches!(
            store.handoff(&request),
            Err(TransactionStoreError::StateConflict)
        ));
        drop(store);

        let reopened =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        let snapshot = reopened
            .snapshot(&prepared.transaction_id)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(snapshot.state, TransactionState::Indeterminate);
        assert_eq!(snapshot.state_version, 2);
    }

    #[test]
    fn no_effect_recovery_retires_predecessor_and_appends_one_next_generation() {
        let db = TestDatabase::new();
        let first_binding = binding_with_observation("observation-a");
        let next_binding = binding_with_observation("observation-b");
        let mut store =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        let prepared = prepared_snapshot(
            store
                .prepare(&first_binding)
                .unwrap_or_else(|error| unreachable!("{error}")),
        );
        let handoff = store
            .handoff(&HandoffRequest {
                transaction_id: prepared.transaction_id.clone(),
                expected_generation: 0,
                expected_state_version: 1,
                expected_binding_digest: prepared.binding_digest.clone(),
                authority_use_digest: digest("authority-use"),
            })
            .unwrap_or_else(|error| unreachable!("{error}"));
        let outcome = store
            .recover(&RecoveryRequest {
                transaction_id: prepared.transaction_id.clone(),
                expected_generation: 0,
                expected_state_version: handoff.state_version,
                expected_binding_digest: prepared.binding_digest.clone(),
                resolution: RecoveryResolution::IntendedEffectAbsent {
                    observation_digest: digest("recovery-no-effect"),
                    next_binding: Box::new(next_binding.clone()),
                },
            })
            .unwrap_or_else(|error| unreachable!("{error}"));
        let RecoveryOutcome::Reprepared(reprepared) = outcome else {
            unreachable!("expected reprepare")
        };
        assert_eq!(reprepared.current_generation, 1);
        assert_eq!(reprepared.state, TransactionState::Prepared);
        let predecessor_state: String = store
            .connection
            .query_row(
                "SELECT state FROM generations WHERE transaction_id=?1 AND generation=0",
                params![prepared.transaction_id.as_str()],
                |row| row.get(0),
            )
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(predecessor_state, "aborted");
        assert!(matches!(
            store.recover(&RecoveryRequest {
                transaction_id: prepared.transaction_id.clone(),
                expected_generation: 0,
                expected_state_version: handoff.state_version,
                expected_binding_digest: prepared.binding_digest.clone(),
                resolution: RecoveryResolution::ConflictingState {
                    observation_digest: digest("conflict")
                },
            }),
            Err(TransactionStoreError::StateConflict)
        ));
        assert!(matches!(
            store.prepare(&first_binding),
            Err(TransactionStoreError::IdempotencyConflict)
        ));
        assert!(matches!(
            store.prepare(&next_binding),
            Ok(PrepareOutcome::Existing(_))
        ));
    }

    #[test]
    fn verification_is_required_before_commit() {
        let db = TestDatabase::new();
        let binding = binding_with_observation("observation-a");
        let mut store =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        let prepared = prepared_snapshot(
            store
                .prepare(&binding)
                .unwrap_or_else(|error| unreachable!("{error}")),
        );
        assert!(matches!(
            store.commit(&CommitRequest {
                transaction_id: prepared.transaction_id.clone(),
                expected_generation: 0,
                expected_state_version: 1,
                desired_state_digest: digest("desired"),
                graph_digest: digest("graph"),
                provenance_digest: digest("provenance"),
            }),
            Err(TransactionStoreError::StateConflict)
        ));
        let handoff = store
            .handoff(&HandoffRequest {
                transaction_id: prepared.transaction_id.clone(),
                expected_generation: 0,
                expected_state_version: 1,
                expected_binding_digest: prepared.binding_digest.clone(),
                authority_use_digest: digest("authority-use"),
            })
            .unwrap_or_else(|error| unreachable!("{error}"));
        let verified = store
            .recover(&RecoveryRequest {
                transaction_id: prepared.transaction_id.clone(),
                expected_generation: 0,
                expected_state_version: handoff.state_version,
                expected_binding_digest: prepared.binding_digest.clone(),
                resolution: RecoveryResolution::IntendedStateVerified {
                    observation_digest: digest("verified-observation"),
                },
            })
            .unwrap_or_else(|error| unreachable!("{error}"));
        let RecoveryOutcome::Verified(verified) = verified else {
            unreachable!("expected verified")
        };
        let committed = store
            .commit(&CommitRequest {
                transaction_id: prepared.transaction_id,
                expected_generation: verified.current_generation,
                expected_state_version: verified.state_version,
                desired_state_digest: digest("desired"),
                graph_digest: digest("graph"),
                provenance_digest: digest("provenance"),
            })
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(committed.state, TransactionState::Committed);
        assert!(
            store
                .connection
                .execute(
                    "UPDATE generations SET desired_state_digest=?1, graph_digest=?2, provenance_digest=?3 WHERE transaction_id=?4 AND generation=?5",
                    params![
                        digest("tampered-desired").as_str(),
                        digest("tampered-graph").as_str(),
                        digest("tampered-provenance").as_str(),
                        committed.transaction_id.as_str(),
                        committed.current_generation,
                    ],
                )
                .is_err()
        );
        store
            .integrity_check()
            .unwrap_or_else(|error| unreachable!("{error}"));
    }

    #[test]
    fn audit_history_is_append_only() {
        let db = TestDatabase::new();
        let mut store =
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        let binding = binding_with_observation("observation-a");
        let prepared = prepared_snapshot(
            store
                .prepare(&binding)
                .unwrap_or_else(|error| unreachable!("{error}")),
        );
        assert!(
            store
                .connection
                .execute(
                    "UPDATE audit_events SET event_kind='tampered' WHERE transaction_id=?1",
                    params![prepared.transaction_id.as_str()],
                )
                .is_err()
        );
        assert!(
            store
                .connection
                .execute(
                    "DELETE FROM audit_events WHERE transaction_id=?1",
                    params![prepared.transaction_id.as_str()],
                )
                .is_err()
        );
        store
            .integrity_check()
            .unwrap_or_else(|error| unreachable!("{error}"));
    }

    #[test]
    fn installed_schema_tampering_is_detected_on_reopen() {
        let db = TestDatabase::new();
        drop(
            SqliteTransactionStore::open(&db.path).unwrap_or_else(|error| unreachable!("{error}")),
        );
        let raw = Connection::open(&db.path).unwrap_or_else(|error| unreachable!("{error}"));
        raw.execute_batch("DROP TRIGGER generations_binding_immutable;")
            .unwrap_or_else(|error| unreachable!("{error}"));
        drop(raw);
        assert!(matches!(
            SqliteTransactionStore::open(&db.path),
            Err(TransactionStoreError::UnsupportedSchema(_))
        ));
    }

    #[test]
    fn future_schema_and_migration_mismatch_fail_closed() {
        let future = TestDatabase::new();
        drop(
            SqliteTransactionStore::open(&future.path)
                .unwrap_or_else(|error| unreachable!("{error}")),
        );
        let raw = Connection::open(&future.path).unwrap_or_else(|error| unreachable!("{error}"));
        raw.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap_or_else(|error| unreachable!("{error}"));
        drop(raw);
        assert!(matches!(
            SqliteTransactionStore::open(&future.path),
            Err(TransactionStoreError::UnsupportedSchema(_))
        ));

        let mismatch = TestDatabase::new();
        drop(
            SqliteTransactionStore::open(&mismatch.path)
                .unwrap_or_else(|error| unreachable!("{error}")),
        );
        let raw = Connection::open(&mismatch.path).unwrap_or_else(|error| unreachable!("{error}"));
        raw.execute_batch("DROP TRIGGER schema_migrations_no_update;")
            .unwrap_or_else(|error| unreachable!("{error}"));
        raw.execute(
            "UPDATE schema_migrations SET checksum=?1 WHERE migration_id=?2",
            params![digest("other-migration").as_str(), MIGRATION_ID],
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
        drop(raw);
        assert!(matches!(
            SqliteTransactionStore::open(&mismatch.path),
            Err(TransactionStoreError::UnsupportedSchema(_))
        ));
    }
}
