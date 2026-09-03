from __future__ import annotations

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, content: str) -> None:
    (ROOT / path).write_text(content, encoding="utf-8")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one anchor, found {count}")
    return text.replace(old, new, 1)


# 1) Non-elidable integrity-key zeroization.
path = "crates/linura-persistence-sqlite/Cargo.toml"
text = read(path)
text = replace_once(
    text,
    'sha2 = "0.10"\n',
    'sha2 = "0.10"\nzeroize = "1.8"\n',
    "sqlite zeroize dependency",
)
write(path, text)

path = "crates/linura-persistence-sqlite/src/integrity.rs"
text = read(path)
text = replace_once(
    text,
    'use sha2::{Digest, Sha256};\n',
    'use sha2::{Digest, Sha256};\nuse zeroize::Zeroize;\n',
    "zeroize import",
)
text = replace_once(
    text,
    '        self.bytes.fill(0);\n',
    '        self.bytes.zeroize();\n',
    "integrity-key drop zeroization",
)
text = replace_once(
    text,
    '            bytes.fill(0);\n            return Err(TransactionStoreError::AuthorityRejected);\n',
    '            bytes.zeroize();\n            return Err(TransactionStoreError::AuthorityRejected);\n',
    "rejected integrity-key zeroization",
)
text = replace_once(
    text,
    '        let Ok(expected) = self.tag(record_domain, canonical) else {\n            return false;\n        };\n        constant_time_eq(&expected, tag)\n',
    '        let Ok(mut expected) = self.tag(record_domain, canonical) else {\n            return false;\n        };\n        let valid = constant_time_eq(&expected, tag);\n        expected.zeroize();\n        valid\n',
    "verification temporary zeroization",
)
text = replace_once(
    text,
    '    key_block.fill(0);\n    inner_pad.fill(0);\n    outer_pad.fill(0);\n',
    '    key_block.zeroize();\n    inner_pad.zeroize();\n    outer_pad.zeroize();\n',
    "HMAC scratch zeroization",
)
text += '''\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn rejected_integrity_key_buffer_uses_zeroize_primitive() {\n        let mut bytes = vec![0xA5; INTEGRITY_KEY_BYTES - 1];\n        bytes.zeroize();\n        assert!(bytes.iter().all(|byte| *byte == 0));\n    }\n}\n'''
write(path, text)

# 2) Bound aggregate schema-fingerprint materialization as well as each field.
path = "crates/linura-persistence-sqlite/src/validation.rs"
text = read(path)
text = replace_once(
    text,
    'const MAX_SCHEMA_SQL_BYTES: usize = 64 * 1024;\n',
    'const MAX_SCHEMA_SQL_BYTES: usize = 64 * 1024;\nconst MAX_SCHEMA_OBJECTS: usize = 256;\nconst MAX_SCHEMA_FINGERPRINT_BYTES: usize = 4 * 1024 * 1024;\n',
    "schema aggregate constants",
)
anchor = '''fn schema_fingerprint(connection: &Connection) -> Result<ContentDigest, TransactionStoreError> {\n    let mut statement = connection\n'''
replacement = '''fn schema_fingerprint(connection: &Connection) -> Result<ContentDigest, TransactionStoreError> {\n    let object_count: i64 = connection\n        .query_row(\n            "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",\n            [],\n            |row| row.get(0),\n        )\n        .map_err(sqlite)?;\n    let object_count = usize::try_from(object_count).map_err(|_| {\n        TransactionStoreError::UnsupportedSchema(\n            "SQLite schema object count is negative or unrepresentable".into(),\n        )\n    })?;\n    if object_count > MAX_SCHEMA_OBJECTS {\n        return Err(TransactionStoreError::UnsupportedSchema(format!(\n            "SQLite schema object count {object_count} exceeds validation bound {MAX_SCHEMA_OBJECTS}"\n        )));\n    }\n\n    let mut statement = connection\n'''
text = replace_once(text, anchor, replacement, "schema object-count preflight")
text = replace_once(
    text,
    '    let mut encoded = Vec::new();\n',
    '    let mut encoded = Vec::with_capacity(object_count.saturating_mul(256).min(MAX_SCHEMA_FINGERPRINT_BYTES));\n',
    "bounded schema fingerprint capacity",
)
old = '''        for value in [row.1, row.3, row.5, row.7] {\n            let value = value.ok_or_else(|| {\n                TransactionStoreError::UnsupportedSchema(\n                    "SQLite schema object withheld by length preflight".into(),\n                )\n            })?;\n            canonical_field(&mut encoded, value.as_bytes());\n        }\n'''
new = '''        for value in [row.1, row.3, row.5, row.7] {\n            let value = value.ok_or_else(|| {\n                TransactionStoreError::UnsupportedSchema(\n                    "SQLite schema object withheld by length preflight".into(),\n                )\n            })?;\n            let next_len = encoded\n                .len()\n                .checked_add(8)\n                .and_then(|length| length.checked_add(value.len()))\n                .ok_or_else(|| {\n                    TransactionStoreError::UnsupportedSchema(\n                        "SQLite schema fingerprint materialization length overflow".into(),\n                    )\n                })?;\n            if next_len > MAX_SCHEMA_FINGERPRINT_BYTES {\n                return Err(TransactionStoreError::UnsupportedSchema(format!(\n                    "SQLite schema fingerprint exceeds aggregate validation bound {MAX_SCHEMA_FINGERPRINT_BYTES}"\n                )));\n            }\n            canonical_field(&mut encoded, value.as_bytes());\n        }\n'''
text = replace_once(text, old, new, "aggregate schema fingerprint bound")
write(path, text)

# 3) Same-filesystem emergency recovery reserve.
reserve_path = ROOT / "crates/linura-persistence-sqlite/src/filesystem_reserve.rs"
reserve_path.write_text(r'''use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use linura_transaction::TransactionStoreError;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

/// One emergency slot is large enough for 128 maximum-size SQLite pages.
/// The v0.4 durability workflow additionally qualifies this reserve against a
/// real ext4 ENOSPC condition with WAL + synchronous=FULL.
pub(crate) const RECOVERY_RESERVE_SLOT_BYTES: u64 = 8 * 1024 * 1024;
const RESERVE_SLOTS_PER_NONTERMINAL: u64 = 3;
const IO_CHUNK_BYTES: usize = 64 * 1024;
const RESERVE_SUFFIX: &str = ".linura-recovery-reserve";

#[derive(Debug)]
pub(crate) struct FilesystemRecoveryReserve {
    path: Option<PathBuf>,
    bootstrap_released: bool,
}

impl FilesystemRecoveryReserve {
    pub(crate) fn before_database_open(database: &Path) -> Result<Self, TransactionStoreError> {
        if database == Path::new(":memory:") {
            return Ok(Self {
                path: None,
                bootstrap_released: false,
            });
        }
        let path = reserve_path(database)?;
        validate_existing_path(&path)?;
        let slots = current_slots(&path)?;
        if slots == 0 {
            return Ok(Self {
                path: Some(path),
                bootstrap_released: false,
            });
        }
        match slots % RESERVE_SLOTS_PER_NONTERMINAL {
            0 => {
                resize_file(&path, slots - 1)?;
                Ok(Self {
                    path: Some(path),
                    bootstrap_released: true,
                })
            }
            2 => Ok(Self {
                path: Some(path),
                bootstrap_released: true,
            }),
            _ => Err(TransactionStoreError::Corruption(
                "filesystem recovery reserve has an invalid slot shape".into(),
            )),
        }
    }

    pub(crate) fn reconcile_after_database_open(
        &mut self,
        nonterminal_count: u64,
    ) -> Result<(), TransactionStoreError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        let full_slots = full_slots(nonterminal_count)?;
        if nonterminal_count == 0 {
            resize_file(path, 0)?;
            self.bootstrap_released = false;
            return Ok(());
        }
        let expected = if self.bootstrap_released {
            full_slots
                .checked_sub(1)
                .ok_or(TransactionStoreError::CapacityExceeded)?
        } else {
            full_slots
        };
        resize_file(path, expected)?;
        validate_allocated(path, expected)?;
        Ok(())
    }

    pub(crate) fn ensure_full(
        &mut self,
        nonterminal_count: u64,
    ) -> Result<(), TransactionStoreError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        let target = full_slots(nonterminal_count)?;
        resize_file(path, target)?;
        validate_allocated(path, target)?;
        self.bootstrap_released = false;
        Ok(())
    }

    pub(crate) fn ensure_terminal_headroom(
        &mut self,
        nonterminal_count: u64,
    ) -> Result<(), TransactionStoreError> {
        if nonterminal_count == 0 {
            return Err(TransactionStoreError::Corruption(
                "terminal recovery reserve requested without a nonterminal transaction".into(),
            ));
        }
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        let full = full_slots(nonterminal_count)?;
        if self.bootstrap_released {
            let expected = full
                .checked_sub(1)
                .ok_or(TransactionStoreError::CapacityExceeded)?;
            let actual = current_slots(path)?;
            if actual < expected {
                return Err(TransactionStoreError::Corruption(
                    "filesystem recovery reserve was truncated while bootstrap headroom was borrowed".into(),
                ));
            }
            if actual > expected {
                resize_file(path, expected)?;
            }
            validate_allocated(path, expected)?;
            return Ok(());
        }
        resize_file(path, full)?;
        validate_allocated(path, full)?;
        let borrowed = full
            .checked_sub(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        resize_file(path, borrowed)?;
        self.bootstrap_released = true;
        Ok(())
    }

    /// Final cleanup only releases additional space. A failure here cannot
    /// invalidate a transaction that SQLite has already committed, so callers
    /// intentionally treat it as best-effort and reopen reconciliation removes
    /// any safe over-reservation.
    pub(crate) fn finish_terminal(&mut self, nonterminal_count: u64) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if let Ok(target) = full_slots(nonterminal_count) {
            if resize_file(path, target).is_ok() {
                self.bootstrap_released = false;
            }
        }
    }
}

impl Drop for FilesystemRecoveryReserve {
    fn drop(&mut self) {
        if !self.bootstrap_released {
            return;
        }
        let Some(path) = self.path.as_ref() else {
            return;
        };
        let Ok(slots) = current_slots(path) else {
            return;
        };
        if slots % RESERVE_SLOTS_PER_NONTERMINAL == 2 {
            let _ = resize_file(path, slots.saturating_add(1));
        }
    }
}

fn full_slots(nonterminal_count: u64) -> Result<u64, TransactionStoreError> {
    nonterminal_count
        .checked_mul(RESERVE_SLOTS_PER_NONTERMINAL)
        .ok_or(TransactionStoreError::CapacityExceeded)
}

fn reserve_path(database: &Path) -> Result<PathBuf, TransactionStoreError> {
    let file_name = database.file_name().ok_or_else(|| {
        TransactionStoreError::Storage("authority database path has no file name".into())
    })?;
    let mut name = file_name.to_os_string();
    name.push(RESERVE_SUFFIX);
    Ok(database.with_file_name(name))
}

fn validate_existing_path(path: &Path) -> Result<(), TransactionStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(TransactionStoreError::Storage(
                    "filesystem recovery reserve must be a regular non-symlink file".into(),
                ));
            }
            if metadata.len() % RECOVERY_RESERVE_SLOT_BYTES != 0 {
                return Err(TransactionStoreError::Corruption(
                    "filesystem recovery reserve length is not slot-aligned".into(),
                ));
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(storage(error)),
    }
}

fn current_slots(path: &Path) -> Result<u64, TransactionStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(TransactionStoreError::Storage(
                    "filesystem recovery reserve must be a regular non-symlink file".into(),
                ));
            }
            if metadata.len() % RECOVERY_RESERVE_SLOT_BYTES != 0 {
                return Err(TransactionStoreError::Corruption(
                    "filesystem recovery reserve length is not slot-aligned".into(),
                ));
            }
            Ok(metadata.len() / RECOVERY_RESERVE_SLOT_BYTES)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(storage(error)),
    }
}

fn resize_file(path: &Path, slots: u64) -> Result<(), TransactionStoreError> {
    validate_existing_path(path)?;
    let target = slots
        .checked_mul(RECOVERY_RESERVE_SLOT_BYTES)
        .ok_or(TransactionStoreError::CapacityExceeded)?;
    let existed = path.exists();
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).map_err(storage)?;
    let current = file.metadata().map_err(storage)?.len();
    if current < target {
        file.seek(SeekFrom::Start(current)).map_err(storage)?;
        let zeros = [0_u8; IO_CHUNK_BYTES];
        let mut remaining = target - current;
        while remaining > 0 {
            let amount = usize::try_from(remaining.min(IO_CHUNK_BYTES as u64))
                .map_err(|_| TransactionStoreError::CapacityExceeded)?;
            file.write_all(&zeros[..amount]).map_err(storage)?;
            remaining -= amount as u64;
        }
    } else if current > target {
        file.set_len(target).map_err(storage)?;
    }
    file.sync_all().map_err(storage)?;
    if !existed {
        sync_parent(path)?;
    }
    Ok(())
}

fn validate_allocated(path: &Path, slots: u64) -> Result<(), TransactionStoreError> {
    let expected = slots
        .checked_mul(RECOVERY_RESERVE_SLOT_BYTES)
        .ok_or(TransactionStoreError::CapacityExceeded)?;
    if expected == 0 {
        return Ok(());
    }
    let metadata = fs::metadata(path).map_err(storage)?;
    if metadata.len() < expected {
        return Err(TransactionStoreError::Corruption(
            "filesystem recovery reserve is shorter than required".into(),
        ));
    }
    #[cfg(unix)]
    {
        let allocated = metadata
            .blocks()
            .checked_mul(512)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        if allocated < expected {
            return Err(TransactionStoreError::Corruption(
                "filesystem recovery reserve is sparse or under-allocated".into(),
            ));
        }
    }
    Ok(())
}

fn sync_parent(path: &Path) -> Result<(), TransactionStoreError> {
    #[cfg(unix)]
    {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        File::open(parent).map_err(storage)?.sync_all().map_err(storage)?;
    }
    Ok(())
}

fn storage(error: std::io::Error) -> TransactionStoreError {
    TransactionStoreError::Storage(format!("filesystem recovery reserve: {error}"))
}
''', encoding="utf-8")

path = "crates/linura-persistence-sqlite/src/lib.rs"
text = read(path)
text = replace_once(
    text,
    '#[rustfmt::skip]\nmod integrity;\n',
    '#[rustfmt::skip]\nmod filesystem_reserve;\n#[rustfmt::skip]\nmod integrity;\n',
    "filesystem reserve module",
)
write(path, text)

path = "crates/linura-persistence-sqlite/src/store.rs"
text = read(path)
text = replace_once(
    text,
    'use crate::integrity::SqliteIntegrityKey;\n',
    'use crate::filesystem_reserve::FilesystemRecoveryReserve;\nuse crate::integrity::SqliteIntegrityKey;\n',
    "reserve import",
)
text = replace_once(
    text,
    '    integrity_key: SqliteIntegrityKey,\n}',
    '    integrity_key: SqliteIntegrityKey,\n    filesystem_reserve: FilesystemRecoveryReserve,\n}',
    "reserve store field",
)
old = '''        let limits = limits.validate()?;\n        let mut connection = Connection::open(path).map_err(sqlite)?;\n        configure_connection(&connection, limits)?;\n'''
new = '''        let limits = limits.validate()?;\n        let path = path.as_ref().to_path_buf();\n        let mut filesystem_reserve = FilesystemRecoveryReserve::before_database_open(&path)?;\n        let mut connection = Connection::open(&path).map_err(sqlite)?;\n        configure_connection(&connection, limits)?;\n'''
text = replace_once(text, old, new, "reserve before SQLite open")
old = '''        let store = Self {\n            connection,\n            limits,\n            authority_verifier,\n            integrity_key,\n        };\n        store.integrity_check()?;\n        Ok(store)\n'''
new = '''        let mut store = Self {\n            connection,\n            limits,\n            authority_verifier,\n            integrity_key,\n            filesystem_reserve,\n        };\n        store.integrity_check()?;\n        let nonterminal_count = Self::current_nonterminal_count(&store.connection)?;\n        store\n            .filesystem_reserve\n            .reconcile_after_database_open(nonterminal_count)?;\n        Ok(store)\n'''
text = replace_once(text, old, new, "reserve reopen reconciliation")
anchor = '''    fn assert_expected(\n        snapshot: &TransactionSnapshot,\n'''
helper = '''    fn current_nonterminal_count(\n        connection: &Connection,\n    ) -> Result<u64, TransactionStoreError> {\n        let count: i64 = connection\n            .query_row(\n                "SELECT COUNT(*) FROM transactions t\n                 JOIN generations g ON g.transaction_id=t.transaction_id\n                  AND g.generation=t.current_generation\n                 WHERE g.state IN ('prepared','indeterminate','verified')",\n                [],\n                |row| row.get(0),\n            )\n            .map_err(sqlite)?;\n        u64::try_from(count).map_err(|_| {\n            TransactionStoreError::Corruption(\n                "negative current nonterminal transaction count".into(),\n            )\n        })\n    }\n\n    fn assert_expected(\n        snapshot: &TransactionSnapshot,\n'''
text = replace_once(text, anchor, helper, "nonterminal counter")

# Borrow the independent reserve alongside connection/integrity state in every mutating method.
for signature in [
    '        let limits = self.limits;\n        let integrity_key = &self.integrity_key;\n        let transaction = self\n            .connection\n',
]:
    # Occurs in prepare/handoff/recover/commit/abort; replace all intentionally.
    count = text.count(signature)
    if count != 5:
        raise RuntimeError(f"mutating-method borrow anchor: expected 5, found {count}")
    text = text.replace(
        signature,
        '        let limits = self.limits;\n        let integrity_key = &self.integrity_key;\n        let filesystem_reserve = &mut self.filesystem_reserve;\n        let transaction = self\n            .connection\n',
    )

# Prepare must physically reserve before creating durable Prepared state.
old = '''        check_count_capacity(&transaction, "transactions", limits.max_transactions, 1)?;\n        check_count_capacity(&transaction, "generations", limits.max_generations, 1)?;\n        check_logical_audit_reserve(&transaction, limits, 1, 1)?;\n\n        let mut transaction_record = StoredTransaction {\n'''
new = '''        check_count_capacity(&transaction, "transactions", limits.max_transactions, 1)?;\n        check_count_capacity(&transaction, "generations", limits.max_generations, 1)?;\n        check_logical_audit_reserve(&transaction, limits, 1, 1)?;\n        let nonterminal_before = Self::current_nonterminal_count(&transaction)?;\n        let nonterminal_after = nonterminal_before\n            .checked_add(1)\n            .ok_or(TransactionStoreError::CapacityExceeded)?;\n        filesystem_reserve.ensure_full(nonterminal_after)?;\n\n        let mut transaction_record = StoredTransaction {\n'''
text = replace_once(text, old, new, "prepare external reserve admission")

# Handoff remains nonterminal and cannot consume emergency headroom.
old = '''        check_logical_audit_reserve(&transaction, limits, 1, 0)?;\n        Self::ensure_audit_reservations(&transaction, request.transaction_id(), 2)?;\n'''
new = '''        check_logical_audit_reserve(&transaction, limits, 1, 0)?;\n        let nonterminal_count = Self::current_nonterminal_count(&transaction)?;\n        filesystem_reserve.ensure_full(nonterminal_count)?;\n        Self::ensure_audit_reservations(&transaction, request.transaction_id(), 2)?;\n'''
text = replace_once(text, old, new, "handoff external reserve restoration")

# Recovery: compute current count once, restore full reserve for nonterminal outcomes,
# or borrow emergency headroom for the terminal blocked outcome.
old = '''        let next_version = snapshot\n            .state_version\n            .checked_add(1)\n            .ok_or(TransactionStoreError::CapacityExceeded)?;\n\n        let outcome = match request.resolution() {\n'''
new = '''        let next_version = snapshot\n            .state_version\n            .checked_add(1)\n            .ok_or(TransactionStoreError::CapacityExceeded)?;\n        let nonterminal_before = Self::current_nonterminal_count(&transaction)?;\n\n        let outcome = match request.resolution() {\n'''
text = replace_once(text, old, new, "recovery nonterminal count")
text = replace_once(
    text,
    '''            } => {\n                check_logical_audit_reserve(&transaction, limits, 1, 0)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 2)?;\n                Self::update_generation_to_verified(\n''',
    '''            } => {\n                check_logical_audit_reserve(&transaction, limits, 1, 0)?;\n                filesystem_reserve.ensure_full(nonterminal_before)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 2)?;\n                Self::update_generation_to_verified(\n''',
    "verified recovery reserve",
)
text = replace_once(
    text,
    '''            RecoveryResolution::ConflictingState { observation_digest } => {\n                check_logical_audit_reserve(&transaction, limits, 1, -1)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 1)?;\n''',
    '''            RecoveryResolution::ConflictingState { observation_digest } => {\n                check_logical_audit_reserve(&transaction, limits, 1, -1)?;\n                filesystem_reserve.ensure_terminal_headroom(nonterminal_before)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 1)?;\n''',
    "blocked recovery terminal reserve",
)
text = replace_once(
    text,
    '''            RecoveryResolution::Ambiguous { observation_digest } => {\n                check_logical_audit_reserve(&transaction, limits, 1, 0)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 2)?;\n''',
    '''            RecoveryResolution::Ambiguous { observation_digest } => {\n                check_logical_audit_reserve(&transaction, limits, 1, 0)?;\n                filesystem_reserve.ensure_full(nonterminal_before)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 2)?;\n''',
    "ambiguous recovery reserve",
)
text = replace_once(
    text,
    '''                check_count_capacity(&transaction, "generations", limits.max_generations, 1)?;\n                check_logical_audit_reserve(&transaction, limits, 2, 0)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 3)?;\n''',
    '''                check_count_capacity(&transaction, "generations", limits.max_generations, 1)?;\n                check_logical_audit_reserve(&transaction, limits, 2, 0)?;\n                filesystem_reserve.ensure_full(nonterminal_before)?;\n                Self::ensure_audit_reservations(&transaction, request.transaction_id(), 3)?;\n''',
    "reprepare recovery reserve",
)
old = '''        transaction.commit().map_err(sqlite)?;\n        Ok(outcome)\n    }\n\n    fn commit(\n'''
new = '''        transaction.commit().map_err(sqlite)?;\n        if matches!(&outcome, RecoveryOutcome::Blocked(_)) {\n            filesystem_reserve.finish_terminal(nonterminal_before.saturating_sub(1));\n        }\n        Ok(outcome)\n    }\n\n    fn commit(\n'''
text = replace_once(text, old, new, "blocked recovery reserve cleanup")

# Commit is terminal.
old = '''        check_logical_audit_reserve(&transaction, limits, 1, -1)?;\n        Self::ensure_audit_reservations(&transaction, request.transaction_id(), 1)?;\n        let next_version = snapshot\n'''
new = '''        check_logical_audit_reserve(&transaction, limits, 1, -1)?;\n        let nonterminal_before = Self::current_nonterminal_count(&transaction)?;\n        filesystem_reserve.ensure_terminal_headroom(nonterminal_before)?;\n        Self::ensure_audit_reservations(&transaction, request.transaction_id(), 1)?;\n        let next_version = snapshot\n'''
text = replace_once(text, old, new, "commit terminal reserve")
old = '''        transaction.commit().map_err(sqlite)?;\n        Ok(TransactionSnapshot {\n            state: TransactionState::Committed,\n'''
new = '''        transaction.commit().map_err(sqlite)?;\n        filesystem_reserve.finish_terminal(nonterminal_before.saturating_sub(1));\n        Ok(TransactionSnapshot {\n            state: TransactionState::Committed,\n'''
text = replace_once(text, old, new, "commit reserve cleanup")

# Abort Prepared is terminal and is the restart-cleanup path exercised under ENOSPC.
old = '''        check_logical_audit_reserve(&transaction, limits, 1, -1)?;\n        Self::ensure_audit_reservations(&transaction, &request.transaction_id, 1)?;\n        let next_version = snapshot\n'''
new = '''        check_logical_audit_reserve(&transaction, limits, 1, -1)?;\n        let nonterminal_before = Self::current_nonterminal_count(&transaction)?;\n        filesystem_reserve.ensure_terminal_headroom(nonterminal_before)?;\n        Self::ensure_audit_reservations(&transaction, &request.transaction_id, 1)?;\n        let next_version = snapshot\n'''
text = replace_once(text, old, new, "abort terminal reserve")
old = '''        transaction.commit().map_err(sqlite)?;\n        Ok(TransactionSnapshot {\n            state: TransactionState::Aborted,\n'''
new = '''        transaction.commit().map_err(sqlite)?;\n        filesystem_reserve.finish_terminal(nonterminal_before.saturating_sub(1));\n        Ok(TransactionSnapshot {\n            state: TransactionState::Aborted,\n'''
text = replace_once(text, old, new, "abort reserve cleanup")
write(path, text)

# Fault probe: terminal abort command and postcondition inspection.
path = "crates/linura-persistence-sqlite/examples/v04_fault_probe.rs"
text = read(path)
text = replace_once(
    text,
    '    AuthorityBinding, AuthorizationBasis, ContentDigest, PrepareOutcome, TransactionAuthorityKey,\n',
    '    AbortRequest, AuthorityBinding, AuthorizationBasis, ContentDigest, PrepareOutcome, TransactionAuthorityKey,\n',
    "fault probe AbortRequest import",
)
insert_anchor = '''fn inspect_indeterminate(path: &Path, namespace: &str) -> Result<(), String> {\n'''
insert = r'''fn abort_prepared(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let mut store = SqliteTransactionStore::open(path, authority()?.1, integrity_key()?)
        .map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Prepared {
        return Err(format!("abort probe expected Prepared, got {:?}", snapshot.state));
    }
    let request = AbortRequest {
        transaction_id: snapshot.transaction_id.clone(),
        expected_generation: snapshot.current_generation,
        expected_state_version: snapshot.state_version,
        reason_digest: digest(&format!("enospc-abort:{namespace}")),
    };
    let after = store.abort_prepared(&request).map_err(|error| error.to_string())?;
    if after.state != TransactionState::Aborted {
        return Err("abort probe did not durably retire Prepared".into());
    }
    store.integrity_check().map_err(|error| error.to_string())?;
    println!(
        "aborted transaction={} generation={} version={}",
        after.transaction_id.as_str(), after.current_generation, after.state_version
    );
    Ok(())
}

fn inspect_aborted(path: &Path, namespace: &str) -> Result<(), String> {
    let binding = binding(namespace)?;
    let store = SqliteTransactionStore::open(path, authority()?.1, integrity_key()?)
        .map_err(|error| error.to_string())?;
    store.integrity_check().map_err(|error| error.to_string())?;
    let snapshot = store
        .snapshot(&binding.transaction_id())
        .map_err(|error| error.to_string())?;
    if snapshot.state != TransactionState::Aborted {
        return Err(format!("expected durable Aborted after ENOSPC recovery, got {snapshot:?}"));
    }
    println!("aborted state survived reopen transaction={}", snapshot.transaction_id.as_str());
    Ok(())
}

fn inspect_indeterminate(path: &Path, namespace: &str) -> Result<(), String> {
'''
text = replace_once(text, insert_anchor, insert, "fault probe abort functions")
text = replace_once(
    text,
    '    "usage: v04_fault_probe <prepare|handoff-wait|inspect-indeterminate|checkpoint-inspect> <db> <namespace> [marker]".into()\n',
    '    "usage: v04_fault_probe <prepare|abort-prepared|inspect-aborted|handoff-wait|inspect-indeterminate|checkpoint-inspect> <db> <namespace> [marker]".into()\n',
    "fault probe usage",
)
match_anchor = '''        "handoff-wait" => {\n'''
match_insert = '''        "abort-prepared" => {\n            if args.next().is_some() {\n                return Err(usage());\n            }\n            abort_prepared(Path::new(&path), &namespace)\n        }\n        "inspect-aborted" => {\n            if args.next().is_some() {\n                return Err(usage());\n            }\n            inspect_aborted(Path::new(&path), &namespace)\n        }\n        "handoff-wait" => {\n'''
text = replace_once(text, match_anchor, match_insert, "fault probe command dispatch")
write(path, text)

# Permanent VM qualification: true ext4 ENOSPC, then Prepared -> Aborted via
# the sidecar headroom while the filler still occupies the rest of the fs.
path = ".github/workflows/v04-durability-vm.yml"
text = read(path)
text = replace_once(
    text,
    '            cloud-image-utils \\\n            openssh-client \\\n',
    '            cloud-image-utils \\\n            e2fsprogs \\\n            openssh-client \\\n',
    "e2fsprogs qualification dependency",
)
anchor = '''      - name: Qualify process SIGKILL after acknowledged durable handoff\n'''
step = r'''      - name: Qualify real ext4 ENOSPC terminal recovery reserve
        run: |
          set -euo pipefail
          ssh_common=(
            -i "$SSH_KEY"
            -o BatchMode=yes
            -o StrictHostKeyChecking=no
            -o UserKnownHostsFile=/dev/null
            -o ConnectTimeout=3
          )
          ssh "${ssh_common[@]}" -p 2222 linura@127.0.0.1 'bash -s' <<'GUEST' | tee "$ARTIFACT_DIR/ext4-enospc.log"
          set -euo pipefail
          image=/var/tmp/linura-v04-enospc.ext4
          mountpoint=/mnt/linura-v04-enospc
          sudo -n umount "$mountpoint" 2>/dev/null || true
          sudo -n rm -f "$image"
          sudo -n truncate -s 160M "$image"
          sudo -n mkfs.ext4 -q -F -m 0 "$image"
          sudo -n mkdir -p "$mountpoint"
          sudo -n mount -o loop "$image" "$mountpoint"
          sudo -n chown linura:linura "$mountpoint"

          db="$mountpoint/authority.db"
          linura-v04-fault-probe prepare "$db" real-enospc
          reserve="$db.linura-recovery-reserve"
          test -f "$reserve"
          reserve_bytes="$(stat -c %s "$reserve")"
          allocated_bytes="$(( $(stat -c %b "$reserve") * 512 ))"
          test "$reserve_bytes" -ge $((24 * 1024 * 1024))
          test "$allocated_bytes" -ge "$reserve_bytes"

          # Consume every ordinary user-allocatable block while leaving the
          # physically allocated Linura reserve intact.
          set +e
          dd if=/dev/zero of="$mountpoint/filler" bs=1M conv=fsync status=none
          dd_status=$?
          set -e
          test "$dd_status" -ne 0
          if dd if=/dev/zero of="$mountpoint/must-fail" bs=4096 count=1 conv=fsync status=none 2>/dev/null; then
            echo "filesystem did not reach a real ENOSPC condition" >&2
            exit 1
          fi
          df -B1 "$mountpoint"

          # Reopening SQLite may need WAL/SHM filesystem blocks. The reserve
          # implementation releases bootstrap headroom before Connection::open,
          # and abort then retires Prepared while the filler remains in place.
          linura-v04-fault-probe abort-prepared "$db" real-enospc
          linura-v04-fault-probe inspect-aborted "$db" real-enospc
          test "$(stat -c %s "$reserve")" -eq 0
          sync
          sudo -n umount "$mountpoint"
          sudo -n rm -f "$image"
          GUEST

      - name: Qualify process SIGKILL after acknowledged durable handoff
'''
text = replace_once(text, anchor, step, "ext4 ENOSPC qualification step")
text = replace_once(
    text,
    '                  "deterministic SQLite write-denial test",\n                  "WAL FULL checkpoint after restart",\n',
    '                  "deterministic SQLite write-denial test",\n                  "real ext4 ENOSPC Prepared-to-Aborted recovery using same-filesystem reserved WAL headroom",\n                  "WAL FULL checkpoint after restart",\n',
    "ENOSPC evidence record",
)
write(path, text)

# A focused integration regression for aggregate schema-object count and reserve
# reconciliation. The real ENOSPC guarantee itself is qualified in the VM gate.
test_path = ROOT / "crates/linura-persistence-sqlite/tests/release_blockers.rs"
test_path.write_text(r'''#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use linura_core::{
    CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
    ResourceId, RiskClass,
};
use linura_persistence_sqlite::{SqliteIntegrityKey, SqliteTransactionStore};
use linura_transaction::{
    AuthorityBinding, AuthorizationBasis, TransactionAuthorityKey, TransactionStore, digest_bytes,
};
use rusqlite::Connection;

fn temp_db(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("linura-{label}-{}-{nonce}.db", std::process::id()))
}

fn keys() -> (linura_transaction::TransactionAuthorityVerifier, SqliteIntegrityKey) {
    let verifier = TransactionAuthorityKey::new(vec![0x41; 32])
        .map(TransactionAuthorityKey::split)
        .map(|(_, verifier)| verifier)
        .unwrap_or_else(|error| unreachable!("{error}"));
    let integrity = SqliteIntegrityKey::new(vec![0x73; 32])
        .unwrap_or_else(|error| unreachable!("{error}"));
    (verifier, integrity)
}

fn binding() -> AuthorityBinding {
    AuthorityBinding::try_new(
        PrincipalId::new("uid:1000").unwrap_or_else(|error| unreachable!("{error}")),
        RequestId::new("request:release-blocker").unwrap_or_else(|error| unreachable!("{error}")),
        PlanId::new("plan:release-blocker").unwrap_or_else(|error| unreachable!("{error}")),
        digest_bytes("test", b"request"),
        digest_bytes("test", b"precondition"),
        digest_bytes("test", b"observation"),
        ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}")),
        ResourceId::new("systemd:unit:test.service").unwrap_or_else(|error| unreachable!("{error}")),
        CapabilityId::new("systemd.unit.observe").unwrap_or_else(|error| unreachable!("{error}")),
        PolicyId::new("policy:test").unwrap_or_else(|error| unreachable!("{error}")),
        PolicyRevisionId::new("policy:test:v1").unwrap_or_else(|error| unreachable!("{error}")),
        RiskClass::SecuritySensitive,
        "risk:v1",
        vec!["rule:1".into()],
        digest_bytes("test", b"review"),
        AuthorizationBasis::PolicyAllow,
    )
    .unwrap_or_else(|error| unreachable!("{error}"))
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);
    let mut reserve = path.as_os_str().to_os_string();
    reserve.push(".linura-recovery-reserve");
    let _ = fs::remove_file(PathBuf::from(reserve));
    let _ = fs::remove_file(path.with_extension("db-wal"));
    let _ = fs::remove_file(path.with_extension("db-shm"));
}

#[test]
fn reserve_is_physically_allocated_and_repaired_from_authenticated_state() {
    let path = temp_db("filesystem-reserve");
    let (verifier, integrity) = keys();
    let mut store = SqliteTransactionStore::open(&path, verifier, integrity)
        .unwrap_or_else(|error| unreachable!("{error}"));
    store.prepare(&binding()).unwrap_or_else(|error| unreachable!("{error}"));
    drop(store);

    let reserve = PathBuf::from(format!("{}.linura-recovery-reserve", path.display()));
    let metadata = fs::metadata(&reserve).unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(metadata.len(), 24 * 1024 * 1024);
    fs::OpenOptions::new()
        .write(true)
        .open(&reserve)
        .and_then(|file| file.set_len(0))
        .unwrap_or_else(|error| unreachable!("{error}"));

    let (verifier, integrity) = keys();
    let store = SqliteTransactionStore::open(&path, verifier, integrity)
        .unwrap_or_else(|error| unreachable!("{error}"));
    drop(store);
    let repaired = fs::metadata(&reserve).unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(repaired.len(), 24 * 1024 * 1024);
    cleanup(&path);
}

#[test]
fn schema_object_count_is_bounded_before_aggregate_materialization() {
    let path = temp_db("schema-count");
    let (verifier, integrity) = keys();
    let store = SqliteTransactionStore::open(&path, verifier, integrity)
        .unwrap_or_else(|error| unreachable!("{error}"));
    drop(store);

    let connection = Connection::open(&path).unwrap_or_else(|error| unreachable!("{error}"));
    for index in 0..300_u16 {
        connection
            .execute_batch(&format!("CREATE TABLE attacker_{index} (id INTEGER);"))
            .unwrap_or_else(|error| unreachable!("{error}"));
    }
    drop(connection);

    let (verifier, integrity) = keys();
    let error = SqliteTransactionStore::open(&path, verifier, integrity)
        .err()
        .unwrap_or_else(|| unreachable!("tampered oversized schema must be rejected"));
    assert!(error.to_string().contains("schema object count"));
    cleanup(&path);
}
''', encoding="utf-8")

# Remove this one-shot patcher from the resulting source tree.
Path(__file__).unlink()
workflow = ROOT / ".github/workflows/tmp-v04-release-blocker-patch.yml"
if workflow.exists():
    workflow.unlink()
