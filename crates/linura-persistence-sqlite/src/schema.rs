pub(crate) const APPLICATION_ID: i64 = 0x4c4e5254; // "LNRT"
pub(crate) const SCHEMA_VERSION: i64 = 1;
pub(crate) const MIGRATION_ID: &str = "0001-v04-hardened-authority-transactions";
pub(crate) const BUSY_TIMEOUT_MS: u64 = 5_000;
pub(crate) const CONTENT_DIGEST_TEXT_BYTES: usize = 71; // "sha256:" + 64 hex
pub(crate) const MAX_AUDIT_EVENT_KIND_BYTES: usize = 64;
pub(crate) const MAX_TRANSACTION_ID_BYTES: usize = 256;
pub(crate) const MAX_PERSISTED_PRINCIPAL_BYTES: usize = 1_024;
pub(crate) const MAX_PERSISTED_REQUEST_ID_BYTES: usize = 1_024;
pub(crate) const MIN_SQLITE_PAGE_SIZE: u64 = 4_096;
pub(crate) const MAX_SQLITE_PAGE_SIZE: u64 = 65_536;

pub(crate) const MIGRATION_V1: &str = r#"
CREATE TABLE schema_migrations (
    migration_id TEXT PRIMARY KEY NOT NULL
        CHECK (length(CAST(migration_id AS BLOB)) BETWEEN 1 AND 128),
    checksum TEXT NOT NULL
        CHECK (length(CAST(checksum AS BLOB)) = 71)
) STRICT;

CREATE TABLE authority_store_identity (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
    verifier_fingerprint TEXT NOT NULL
        CHECK (length(CAST(verifier_fingerprint AS BLOB)) = 71),
    integrity_fingerprint TEXT NOT NULL
        CHECK (length(CAST(integrity_fingerprint AS BLOB)) = 71)
) STRICT;

CREATE TABLE transactions (
    transaction_id TEXT PRIMARY KEY NOT NULL
        CHECK (length(CAST(transaction_id AS BLOB)) BETWEEN 1 AND 256),
    principal TEXT NOT NULL
        CHECK (length(CAST(principal AS BLOB)) BETWEEN 1 AND 1024),
    request_id TEXT NOT NULL
        CHECK (length(CAST(request_id AS BLOB)) BETWEEN 1 AND 1024),
    current_generation INTEGER NOT NULL CHECK (current_generation >= 0),
    state_version INTEGER NOT NULL CHECK (state_version >= 1),
    integrity_tag BLOB NOT NULL CHECK (length(integrity_tag) = 32),
    UNIQUE (principal, request_id)
) STRICT;

CREATE TABLE generations (
    transaction_id TEXT NOT NULL
        CHECK (length(CAST(transaction_id AS BLOB)) BETWEEN 1 AND 256),
    generation INTEGER NOT NULL CHECK (generation >= 0),
    state TEXT NOT NULL CHECK (state IN (
        'prepared', 'indeterminate', 'verified', 'committed', 'aborted', 'recovery-blocked'
    )),
    binding_digest TEXT NOT NULL
        CHECK (length(CAST(binding_digest AS BLOB)) = 71),
    binding_canonical BLOB NOT NULL CHECK (length(binding_canonical) <= 262144),
    request_digest TEXT NOT NULL
        CHECK (length(CAST(request_digest AS BLOB)) = 71),
    precondition_digest TEXT NOT NULL
        CHECK (length(CAST(precondition_digest AS BLOB)) = 71),
    observation_digest TEXT NOT NULL
        CHECK (length(CAST(observation_digest AS BLOB)) = 71),
    desired_state_digest TEXT
        CHECK (desired_state_digest IS NULL OR length(CAST(desired_state_digest AS BLOB)) = 71),
    graph_digest TEXT
        CHECK (graph_digest IS NULL OR length(CAST(graph_digest AS BLOB)) = 71),
    provenance_digest TEXT
        CHECK (provenance_digest IS NULL OR length(CAST(provenance_digest AS BLOB)) = 71),
    integrity_tag BLOB NOT NULL CHECK (length(integrity_tag) = 32),
    PRIMARY KEY (transaction_id, generation),
    FOREIGN KEY (transaction_id) REFERENCES transactions(transaction_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE audit_events (
    transaction_id TEXT NOT NULL
        CHECK (length(CAST(transaction_id AS BLOB)) BETWEEN 1 AND 256),
    event_sequence INTEGER NOT NULL CHECK (event_sequence >= 0),
    generation INTEGER NOT NULL CHECK (generation >= 0),
    state_version INTEGER NOT NULL CHECK (state_version >= 1),
    event_kind TEXT NOT NULL
        CHECK (length(CAST(event_kind AS BLOB)) BETWEEN 1 AND 64),
    payload_digest TEXT NOT NULL
        CHECK (length(CAST(payload_digest AS BLOB)) = 71),
    previous_digest TEXT NOT NULL
        CHECK (length(CAST(previous_digest AS BLOB)) = 71),
    event_digest TEXT NOT NULL
        CHECK (length(CAST(event_digest AS BLOB)) = 71),
    integrity_tag BLOB NOT NULL CHECK (length(integrity_tag) = 32),
    PRIMARY KEY (transaction_id, event_sequence),
    FOREIGN KEY (transaction_id) REFERENCES transactions(transaction_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE audit_reservations (
    transaction_id TEXT NOT NULL
        CHECK (length(CAST(transaction_id AS BLOB)) BETWEEN 1 AND 256),
    slot INTEGER NOT NULL CHECK (slot >= 0),
    reserved BLOB NOT NULL CHECK (length(reserved) BETWEEN 8192 AND 131072),
    PRIMARY KEY (transaction_id, slot),
    FOREIGN KEY (transaction_id) REFERENCES transactions(transaction_id) ON DELETE RESTRICT
) STRICT;

CREATE TRIGGER audit_reservations_filesystem_reserve
BEFORE INSERT ON audit_reservations
BEGIN
    SELECT CASE linura_fs_reserve_slots((SELECT COUNT(*) FROM audit_reservations) + 2)
        WHEN 1 THEN NULL
        WHEN 0 THEN RAISE(ABORT, 'filesystem recovery reserve capacity exceeded')
        ELSE RAISE(ABORT, 'filesystem recovery reserve failure')
    END;
END;

CREATE TRIGGER audit_reservations_filesystem_release
BEFORE DELETE ON audit_reservations
BEGIN
    SELECT CASE linura_fs_release_slots((SELECT COUNT(*) FROM audit_reservations))
        WHEN 1 THEN NULL
        WHEN 0 THEN RAISE(ABORT, 'filesystem recovery reserve capacity exceeded')
        ELSE RAISE(ABORT, 'filesystem recovery reserve failure')
    END;
END;

CREATE TRIGGER transactions_no_conflicting_insert
BEFORE INSERT ON transactions
WHEN EXISTS (
    SELECT 1 FROM transactions
    WHERE transaction_id = NEW.transaction_id
       OR (principal = NEW.principal AND request_id = NEW.request_id)
)
BEGIN
    SELECT RAISE(ABORT, 'immutable transaction identity conflict');
END;

CREATE TRIGGER generations_no_conflicting_insert
BEFORE INSERT ON generations
WHEN EXISTS (
    SELECT 1 FROM generations
    WHERE transaction_id = NEW.transaction_id AND generation = NEW.generation
)
BEGIN
    SELECT RAISE(ABORT, 'immutable generation history conflict');
END;

CREATE TRIGGER audit_events_no_conflicting_insert
BEFORE INSERT ON audit_events
WHEN EXISTS (
    SELECT 1 FROM audit_events
    WHERE transaction_id = NEW.transaction_id AND event_sequence = NEW.event_sequence
)
BEGIN
    SELECT RAISE(ABORT, 'append-only audit history conflict');
END;

CREATE TRIGGER schema_migrations_no_conflicting_insert
BEFORE INSERT ON schema_migrations
WHEN EXISTS (
    SELECT 1 FROM schema_migrations WHERE migration_id = NEW.migration_id
)
BEGIN
    SELECT RAISE(ABORT, 'immutable migration ledger conflict');
END;

CREATE TRIGGER authority_store_identity_no_conflicting_insert
BEFORE INSERT ON authority_store_identity
WHEN EXISTS (
    SELECT 1 FROM authority_store_identity WHERE singleton = NEW.singleton
)
BEGIN
    SELECT RAISE(ABORT, 'immutable authority verifier identity conflict');
END;

CREATE TRIGGER transactions_identity_immutable
BEFORE UPDATE OF transaction_id, principal, request_id ON transactions
BEGIN
    SELECT RAISE(ABORT, 'immutable transaction identity');
END;

CREATE TRIGGER transactions_no_delete
BEFORE DELETE ON transactions
BEGIN
    SELECT RAISE(ABORT, 'immutable transaction identity');
END;

CREATE TRIGGER generations_no_delete
BEFORE DELETE ON generations
BEGIN
    SELECT RAISE(ABORT, 'immutable generation history');
END;

CREATE TRIGGER generations_binding_immutable
BEFORE UPDATE OF binding_digest, binding_canonical, request_digest, precondition_digest, observation_digest ON generations
BEGIN
    SELECT RAISE(ABORT, 'immutable generation authority binding');
END;

CREATE TRIGGER generations_commit_provenance_guard
BEFORE UPDATE OF desired_state_digest, graph_digest, provenance_digest ON generations
WHEN NOT (
    OLD.state = 'indeterminate' AND NEW.state = 'verified'
    AND OLD.desired_state_digest IS NULL AND OLD.graph_digest IS NULL AND OLD.provenance_digest IS NULL
    AND NEW.desired_state_digest IS NOT NULL AND NEW.graph_digest IS NOT NULL AND NEW.provenance_digest IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'immutable verified commit material');
END;

CREATE TRIGGER authority_store_identity_no_update
BEFORE UPDATE ON authority_store_identity
BEGIN
    SELECT RAISE(ABORT, 'immutable authority verifier identity');
END;

CREATE TRIGGER authority_store_identity_no_delete
BEFORE DELETE ON authority_store_identity
BEGIN
    SELECT RAISE(ABORT, 'immutable authority verifier identity');
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
