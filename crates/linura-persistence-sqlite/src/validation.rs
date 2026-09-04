use std::time::Duration;

use linura_transaction::{ContentDigest, TransactionState, TransactionStoreError, digest_bytes};
use rusqlite::config::DbConfig;
use rusqlite::{Connection, OptionalExtension, params};

use crate::filesystem_reserve::{
    register_filesystem_reserve_functions, validate_filesystem_reserve,
};
use crate::integrity::SqliteIntegrityKey;
use crate::schema::{
    APPLICATION_ID, BUSY_TIMEOUT_MS, CONTENT_DIGEST_TEXT_BYTES, MAX_SQLITE_PAGE_SIZE, MIGRATION_ID,
    MIGRATION_V1, MIN_SQLITE_PAGE_SIZE, SCHEMA_VERSION,
};
use crate::store::{StoreLimits, SqliteSettings};

pub(crate) use crate::validation_base::{
    StoredAuditEvent, StoredGeneration, StoredTransaction, as_i64, audit_digest, audit_tag,
    check_count_capacity, check_logical_audit_reserve, generation_tag, list_transaction_ids,
    load_current, load_last_audit_event, pragma_i64, pragma_string, reservation_bytes, sqlite,
    transaction_tag, validate_aggregate_capacity, validate_audit_chain, validate_generation_history,
    validate_logical_audit_reserve,
};

const MAX_SCHEMA_TYPE_BYTES: u64 = 32;
const MAX_SCHEMA_NAME_BYTES: u64 = 256;
const MAX_SCHEMA_SQL_BYTES: u64 = 64 * 1024;
const MAX_SCHEMA_OBJECTS: u64 = 256;
const MAX_SCHEMA_FINGERPRINT_BYTES: u64 = 4 * 1024 * 1024;
const CANONICAL_FIELDS_PER_SCHEMA_OBJECT: u64 = 4;
const CANONICAL_LENGTH_PREFIX_BYTES: u64 = 8;

pub(crate) fn configure_connection(
    connection: &Connection,
    limits: StoreLimits,
) -> Result<(), TransactionStoreError> {
    connection
        .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))
        .map_err(sqlite)?;
    connection
        .set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)
        .map_err(sqlite)?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA recursive_triggers=ON; PRAGMA synchronous=FULL;",
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
    let page_size = page_size(connection)?;
    register_filesystem_reserve_functions(connection, page_size)?;
    connection
        .pragma_update(None, "max_page_count", limits.max_pages)
        .map_err(sqlite)?;
    validate_runtime_settings(connection, limits)
}

pub(crate) fn initialize_or_validate_schema(
    connection: &mut Connection,
    authority_fingerprint: &ContentDigest,
    integrity_fingerprint: &ContentDigest,
) -> Result<(), TransactionStoreError> {
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
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        migration
            .pragma_update(None, "application_id", APPLICATION_ID)
            .map_err(sqlite)?;
        migration.execute_batch(MIGRATION_V1).map_err(sqlite)?;
        migration
            .execute(
                "INSERT INTO authority_store_identity (
                    singleton, verifier_fingerprint, integrity_fingerprint
                 ) VALUES (1, ?1, ?2)",
                params![authority_fingerprint.as_str(), integrity_fingerprint.as_str()],
            )
            .map_err(sqlite)?;
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
        if user_version != SCHEMA_VERSION {
            return Err(TransactionStoreError::UnsupportedSchema(format!(
                "database schema {user_version} is not supported schema {SCHEMA_VERSION}"
            )));
        }
    }
    validate_schema_identity(connection, authority_fingerprint, integrity_fingerprint)
}

pub(crate) fn validate_runtime_settings(
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
    if pragma_i64(connection, "recursive_triggers")? != 1 {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database recursive triggers must be enabled".into(),
        ));
    }
    if !connection
        .db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)
        .map_err(sqlite)?
    {
        return Err(TransactionStoreError::UnsupportedSchema(
            "authority database must disable automatic WAL checkpoint-on-close".into(),
        ));
    }
    let page_size = page_size(connection)?;
    if !(MIN_SQLITE_PAGE_SIZE..=MAX_SQLITE_PAGE_SIZE).contains(&page_size)
        || !page_size.is_power_of_two()
    {
        return Err(TransactionStoreError::UnsupportedSchema(format!(
            "unsupported SQLite page_size {page_size}"
        )));
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

pub(crate) fn validate_schema_identity(
    connection: &Connection,
    authority_fingerprint: &ContentDigest,
    integrity_fingerprint: &ContentDigest,
) -> Result<(), TransactionStoreError> {
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

    let identity = connection
        .query_row(
            "SELECT
                length(CAST(verifier_fingerprint AS BLOB)),
                CASE WHEN length(CAST(verifier_fingerprint AS BLOB)) = ?1 THEN verifier_fingerprint ELSE NULL END,
                length(CAST(integrity_fingerprint AS BLOB)),
                CASE WHEN length(CAST(integrity_fingerprint AS BLOB)) = ?1 THEN integrity_fingerprint ELSE NULL END
             FROM authority_store_identity WHERE singleton = 1",
            params![as_i64(CONTENT_DIGEST_TEXT_BYTES as u64)?],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(sqlite)?
        .ok_or(TransactionStoreError::AuthorityRejected)?;
    if identity.0 != CONTENT_DIGEST_TEXT_BYTES as i64
        || identity.2 != CONTENT_DIGEST_TEXT_BYTES as i64
    {
        return Err(TransactionStoreError::Corruption(
            "authority-store identity fingerprint length is invalid".into(),
        ));
    }
    let stored_authority = ContentDigest::new(identity.1.ok_or_else(|| {
        TransactionStoreError::Corruption("authority fingerprint withheld by length preflight".into())
    })?)
    .map_err(|_| TransactionStoreError::AuthorityRejected)?;
    let stored_integrity = ContentDigest::new(identity.3.ok_or_else(|| {
        TransactionStoreError::Corruption("integrity fingerprint withheld by length preflight".into())
    })?)
    .map_err(|_| TransactionStoreError::AuthorityRejected)?;
    if stored_authority != *authority_fingerprint || stored_integrity != *integrity_fingerprint {
        return Err(TransactionStoreError::AuthorityRejected);
    }

    let checksum = connection
        .query_row(
            "SELECT length(CAST(checksum AS BLOB)),
                    CASE WHEN length(CAST(checksum AS BLOB)) = ?2 THEN checksum ELSE NULL END
             FROM schema_migrations WHERE migration_id = ?1",
            params![MIGRATION_ID, as_i64(CONTENT_DIGEST_TEXT_BYTES as u64)?],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(sqlite)?
        .ok_or_else(|| {
            TransactionStoreError::UnsupportedSchema("migration ledger entry missing".into())
        })?;
    if checksum.0 != CONTENT_DIGEST_TEXT_BYTES as i64
        || checksum.1.as_deref() != Some(migration_checksum().as_str())
    {
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
    let (object_count, aggregate_bytes): (i64, i64) = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(
                 ?1
                 + length(CAST(type AS BLOB))
                 + length(CAST(name AS BLOB))
                 + length(CAST(tbl_name AS BLOB))
                 + length(CAST(COALESCE(sql, '') AS BLOB))
             ), 0)
             FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%'",
            params![
                as_i64(CANONICAL_FIELDS_PER_SCHEMA_OBJECT * CANONICAL_LENGTH_PREFIX_BYTES)?
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(sqlite)?;
    let object_count = u64::try_from(object_count).map_err(|_| {
        TransactionStoreError::UnsupportedSchema("negative SQLite schema object count".into())
    })?;
    let aggregate_bytes = u64::try_from(aggregate_bytes).map_err(|_| {
        TransactionStoreError::UnsupportedSchema("negative SQLite schema byte count".into())
    })?;
    if object_count > MAX_SCHEMA_OBJECTS || aggregate_bytes > MAX_SCHEMA_FINGERPRINT_BYTES {
        return Err(TransactionStoreError::UnsupportedSchema(
            "SQLite schema exceeds aggregate fingerprint validation bounds".into(),
        ));
    }

    let mut statement = connection
        .prepare(
            "SELECT
                length(CAST(type AS BLOB)), CASE WHEN length(CAST(type AS BLOB)) <= ?1 THEN type ELSE NULL END,
                length(CAST(name AS BLOB)), CASE WHEN length(CAST(name AS BLOB)) <= ?2 THEN name ELSE NULL END,
                length(CAST(tbl_name AS BLOB)), CASE WHEN length(CAST(tbl_name AS BLOB)) <= ?2 THEN tbl_name ELSE NULL END,
                length(CAST(COALESCE(sql, '') AS BLOB)), CASE WHEN length(CAST(COALESCE(sql, '') AS BLOB)) <= ?3 THEN COALESCE(sql, '') ELSE NULL END
             FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%'
             ORDER BY type, name",
        )
        .map_err(sqlite)?;
    let rows = statement
        .query_map(
            params![
                as_i64(MAX_SCHEMA_TYPE_BYTES)?,
                as_i64(MAX_SCHEMA_NAME_BYTES)?,
                as_i64(MAX_SCHEMA_SQL_BYTES)?,
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .map_err(sqlite)?;
    let capacity = usize::try_from(aggregate_bytes).map_err(|_| {
        TransactionStoreError::UnsupportedSchema(
            "SQLite schema aggregate size cannot be represented in memory".into(),
        )
    })?;
    let mut encoded = Vec::with_capacity(capacity);
    let mut seen = 0_u64;
    for row in rows {
        let row = row.map_err(sqlite)?;
        seen = seen
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        if seen > MAX_SCHEMA_OBJECTS
            || row.0 < 0
            || row.0 > MAX_SCHEMA_TYPE_BYTES as i64
            || row.2 < 0
            || row.2 > MAX_SCHEMA_NAME_BYTES as i64
            || row.4 < 0
            || row.4 > MAX_SCHEMA_NAME_BYTES as i64
            || row.6 < 0
            || row.6 > MAX_SCHEMA_SQL_BYTES as i64
        {
            return Err(TransactionStoreError::UnsupportedSchema(
                "SQLite schema object exceeds validation byte bound".into(),
            ));
        }
        for value in [row.1, row.3, row.5, row.7] {
            let value = value.ok_or_else(|| {
                TransactionStoreError::UnsupportedSchema(
                    "SQLite schema object withheld by length preflight".into(),
                )
            })?;
            append_canonical_field(&mut encoded, value.as_bytes())?;
        }
        if encoded.len() > capacity || encoded.len() as u64 > MAX_SCHEMA_FINGERPRINT_BYTES {
            return Err(TransactionStoreError::UnsupportedSchema(
                "SQLite schema changed during bounded fingerprint validation".into(),
            ));
        }
    }
    if seen != object_count || encoded.len() as u64 != aggregate_bytes {
        return Err(TransactionStoreError::UnsupportedSchema(
            "SQLite schema changed during fingerprint validation".into(),
        ));
    }
    Ok(digest_bytes("linura.sqlite.schema-objects.v1", &encoded))
}

fn expected_schema_fingerprint() -> Result<ContentDigest, TransactionStoreError> {
    let reference = Connection::open_in_memory().map_err(sqlite)?;
    reference.execute_batch(MIGRATION_V1).map_err(sqlite)?;
    schema_fingerprint(&reference)
}

pub(crate) fn with_immediate_validation<T>(
    connection: &Connection,
    validate: impl FnOnce() -> Result<T, TransactionStoreError>,
) -> Result<T, TransactionStoreError> {
    connection.execute_batch("BEGIN IMMEDIATE").map_err(sqlite)?;
    let result = validate();
    match result {
        Ok(value) => {
            if let Err(error) = connection.execute_batch("COMMIT") {
                let _ = connection.execute_batch("ROLLBACK");
                return Err(sqlite(error));
            }
            Ok(value)
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

pub(crate) fn validate_physical_reservations_locked(
    connection: &Connection,
    key: &SqliteIntegrityKey,
) -> Result<(), TransactionStoreError> {
    if connection.is_autocommit() {
        return Err(TransactionStoreError::StateConflict);
    }
    let expected_bytes = reservation_bytes(connection)?;
    let ids = list_transaction_ids(connection)?;
    let mut expected_total = 0_u64;
    for id in ids {
        let (_, _, snapshot) = load_current(connection, key, &id)?;
        let expected = u64::from(matches!(
            snapshot.state,
            TransactionState::Prepared
                | TransactionState::Indeterminate
                | TransactionState::Verified
        ));
        expected_total = expected_total
            .checked_add(expected)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        let mut statement = connection
            .prepare(
                "SELECT slot, length(reserved) FROM audit_reservations
                 WHERE transaction_id = ?1 ORDER BY slot",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map(params![id.as_str()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(sqlite)?;
        let mut count = 0_u64;
        for row in rows {
            let (slot, length) = row.map_err(sqlite)?;
            if u64::try_from(slot).ok() != Some(count)
                || u64::try_from(length).ok() != Some(expected_bytes)
            {
                return Err(TransactionStoreError::Corruption(
                    "physical audit reservation is malformed".into(),
                ));
            }
            count = count
                .checked_add(1)
                .ok_or(TransactionStoreError::CapacityExceeded)?;
        }
        if count != expected {
            return Err(TransactionStoreError::Corruption(
                "physical audit reservation count disagrees with current state".into(),
            ));
        }
    }
    let actual_total: i64 = connection
        .query_row("SELECT COUNT(*) FROM audit_reservations", [], |row| row.get(0))
        .map_err(sqlite)?;
    let actual_total = u64::try_from(actual_total).map_err(|_| {
        TransactionStoreError::Corruption("negative aggregate physical reservation count".into())
    })?;
    if actual_total != expected_total {
        return Err(TransactionStoreError::Corruption(
            "aggregate physical audit reservation count mismatch".into(),
        ));
    }
    validate_filesystem_reserve(connection, page_size(connection)?, actual_total)
}

fn page_size(connection: &Connection) -> Result<u64, TransactionStoreError> {
    let value = pragma_i64(connection, "page_size")?;
    u64::try_from(value)
        .map_err(|_| TransactionStoreError::UnsupportedSchema("negative page_size".into()))
}

fn migration_checksum() -> ContentDigest {
    digest_bytes("linura.sqlite.migration.v1", MIGRATION_V1.as_bytes())
}

fn append_canonical_field(
    buffer: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), TransactionStoreError> {
    let length = u64::try_from(bytes.len()).map_err(|_| TransactionStoreError::CapacityExceeded)?;
    let next = buffer
        .len()
        .checked_add(CANONICAL_LENGTH_PREFIX_BYTES as usize)
        .and_then(|length| length.checked_add(bytes.len()))
        .ok_or(TransactionStoreError::CapacityExceeded)?;
    if next as u64 > MAX_SCHEMA_FINGERPRINT_BYTES {
        return Err(TransactionStoreError::UnsupportedSchema(
            "SQLite schema exceeds aggregate fingerprint validation bounds".into(),
        ));
    }
    buffer.extend_from_slice(&length.to_be_bytes());
    buffer.extend_from_slice(bytes);
    Ok(())
}

#[allow(dead_code)]
fn _settings_type_remains_part_of_the_public_contract(_: Option<SqliteSettings>) {}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;

    use super::*;

    #[test]
    fn immediate_validation_acquires_write_lock_before_scanning() {
        let database = std::env::temp_dir().join(format!(
            "linura-v04-physical-validation-lock-{}.db",
            std::process::id()
        ));
        let _ = fs::remove_file(&database);
        let _ = fs::remove_file(format!("{}-wal", database.display()));
        let _ = fs::remove_file(format!("{}-shm", database.display()));

        let writer = Connection::open(&database)
            .unwrap_or_else(|error| unreachable!("{error}"));
        writer
            .execute_batch("CREATE TABLE validation_lock_probe (id INTEGER PRIMARY KEY); BEGIN IMMEDIATE;")
            .unwrap_or_else(|error| unreachable!("{error}"));
        let reader = Connection::open(&database)
            .unwrap_or_else(|error| unreachable!("{error}"));
        reader
            .busy_timeout(Duration::ZERO)
            .unwrap_or_else(|error| unreachable!("{error}"));

        let entered = Cell::new(false);
        let blocked = with_immediate_validation(&reader, || {
            entered.set(true);
            Ok(())
        });
        assert!(blocked.is_err());
        assert!(!entered.get());

        writer
            .execute_batch("ROLLBACK")
            .unwrap_or_else(|error| unreachable!("{error}"));
        with_immediate_validation(&reader, || {
            entered.set(true);
            Ok(())
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
        assert!(entered.get());

        drop(reader);
        drop(writer);
        let _ = fs::remove_file(&database);
        let _ = fs::remove_file(format!("{}-wal", database.display()));
        let _ = fs::remove_file(format!("{}-shm", database.display()));
    }
}
