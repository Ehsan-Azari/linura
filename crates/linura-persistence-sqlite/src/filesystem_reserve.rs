use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use linura_transaction::TransactionStoreError;
use rusqlite::functions::FunctionFlags;
use rusqlite::Connection;

const MIN_RECOVERY_RESERVE_SLOT_BYTES: u64 = 256 * 1024;
const RECOVERY_RESERVE_WAL_PAGES: u64 = 32;
const RESERVE_WRITE_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug)]
struct RecoveryReserve {
    path: PathBuf,
    slot_bytes: u64,
}

impl RecoveryReserve {
    fn from_connection(
        connection: &Connection,
        page_size: u64,
    ) -> Result<Self, TransactionStoreError> {
        let database = connection.path().ok_or_else(|| {
            TransactionStoreError::UnsupportedSchema(
                "durable authority storage has no filesystem path".into(),
            )
        })?;
        if database.is_empty() {
            return Err(TransactionStoreError::UnsupportedSchema(
                "durable authority storage cannot use a temporary or in-memory database".into(),
            ));
        }
        let slot_bytes = page_size
            .checked_mul(RECOVERY_RESERVE_WAL_PAGES)
            .ok_or(TransactionStoreError::CapacityExceeded)?
            .max(MIN_RECOVERY_RESERVE_SLOT_BYTES);
        Ok(Self {
            path: reserve_path_for_database(Path::new(database)),
            slot_bytes,
        })
    }

    fn ensure_slots(&self, desired_slots: u64) -> io::Result<()> {
        let target = self.target_len(desired_slots)?;
        let mut file = self.open_locked()?;
        let original = file.metadata()?.len();
        self.require_aligned(original)?;
        if original >= target {
            verify_physical_allocation(&file, original)?;
            return Ok(());
        }

        file.seek(SeekFrom::Start(original))?;
        let mut position = original;
        let mut buffer = [0_u8; RESERVE_WRITE_CHUNK_BYTES];
        while position < target {
            fill_reserve_bytes(&mut buffer, position);
            let remaining = target - position;
            let length = usize::try_from(remaining.min(buffer.len() as u64))
                .map_err(|_| io::Error::other("recovery reserve write length overflow"))?;
            if let Err(error) = file.write_all(&buffer[..length]) {
                rollback_growth(&file, original);
                return Err(error);
            }
            position = position
                .checked_add(length as u64)
                .ok_or_else(|| io::Error::other("recovery reserve size overflow"))?;
        }
        if let Err(error) = file.sync_all() {
            rollback_growth(&file, original);
            return Err(error);
        }
        verify_physical_allocation(&file, target)
    }

    fn release_to_slots(&self, desired_slots: u64) -> io::Result<()> {
        let target = self.target_len(desired_slots)?;
        let file = self.open_locked()?;
        let current = file.metadata()?.len();
        self.require_aligned(current)?;
        if current < target {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "filesystem recovery reserve is smaller than the requested release target",
            ));
        }
        if current > target {
            file.set_len(target)?;
            file.sync_all()?;
        }
        verify_physical_allocation(&file, target)
    }

    fn validate_and_reconcile(&self, reservation_rows: u64) -> Result<(), TransactionStoreError> {
        let expected_slots = reservation_rows
            .checked_add(1)
            .ok_or(TransactionStoreError::CapacityExceeded)?;
        if !self.path.exists() {
            if reservation_rows == 0 {
                return self.ensure_slots(1).map_err(io_store);
            }
            return Err(TransactionStoreError::Corruption(
                "filesystem recovery reserve is missing for nonterminal authority state".into(),
            ));
        }

        let file = self.open_locked().map_err(io_store)?;
        let current = file.metadata().map_err(io_store)?.len();
        self.require_aligned(current).map_err(io_store)?;
        let current_slots = current / self.slot_bytes;
        drop(file);

        if reservation_rows == 0 && current_slots == 0 {
            return self.ensure_slots(1).map_err(io_store);
        }
        if current_slots > expected_slots {
            return self.release_to_slots(expected_slots).map_err(io_store);
        }
        if current_slots == expected_slots
            || (reservation_rows > 0 && current_slots.checked_add(1) == Some(expected_slots))
        {
            let file = self.open_locked().map_err(io_store)?;
            verify_physical_allocation(&file, current).map_err(io_store)?;
            return Ok(());
        }
        Err(TransactionStoreError::Corruption(
            "filesystem recovery reserve disagrees with durable audit reservations".into(),
        ))
    }

    fn open_locked(&self) -> io::Result<File> {
        let existed = self.path.exists();
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.path)?;
        file.lock()?;
        if !existed {
            sync_parent(&self.path)?;
        }
        Ok(file)
    }

    fn target_len(&self, slots: u64) -> io::Result<u64> {
        self.slot_bytes
            .checked_mul(slots)
            .ok_or_else(|| io::Error::other("recovery reserve size overflow"))
    }

    fn require_aligned(&self, length: u64) -> io::Result<()> {
        if !length.is_multiple_of(self.slot_bytes) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "filesystem recovery reserve length is not slot-aligned",
            ));
        }
        Ok(())
    }
}

pub(crate) fn register_filesystem_reserve_functions(
    connection: &Connection,
    page_size: u64,
) -> Result<(), TransactionStoreError> {
    let reserve = RecoveryReserve::from_connection(connection, page_size)?;
    let grow = reserve.clone();
    connection
        .create_scalar_function(
            "linura_fs_reserve_slots",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS,
            move |context| {
                let Ok(desired) = u64::try_from(context.get::<i64>(0)?) else {
                    return Ok(-1_i64);
                };
                Ok(reserve_result(grow.ensure_slots(desired)))
            },
        )
        .map_err(sqlite_store)?;

    let shrink = reserve;
    connection
        .create_scalar_function(
            "linura_fs_release_slots",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS,
            move |context| {
                let Ok(desired) = u64::try_from(context.get::<i64>(0)?) else {
                    return Ok(-1_i64);
                };
                Ok(reserve_result(shrink.release_to_slots(desired)))
            },
        )
        .map_err(sqlite_store)
}

pub(crate) fn validate_filesystem_reserve(
    connection: &Connection,
    page_size: u64,
    reservation_rows: u64,
) -> Result<(), TransactionStoreError> {
    // The complete durable reservation scan and this reconciliation must share
    // one SQLite write-serialization point. Reconciliation may shrink the
    // same-filesystem sidecar, so accepting an autocommit caller would re-open
    // the mixed-snapshot race this invariant is intended to prevent.
    if connection.is_autocommit() {
        return Err(TransactionStoreError::StateConflict);
    }
    let locked_rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM audit_reservations", [], |row| row.get(0))
        .map_err(sqlite_store)?;
    let locked_rows = u64::try_from(locked_rows).map_err(|_| {
        TransactionStoreError::Corruption(
            "negative aggregate physical reservation count under reconciliation lock".into(),
        )
    })?;
    if locked_rows != reservation_rows {
        return Err(TransactionStoreError::Corruption(
            "physical reservation count changed inside serialized validation".into(),
        ));
    }
    RecoveryReserve::from_connection(connection, page_size)?
        .validate_and_reconcile(locked_rows)
}

pub(crate) fn reserve_path_for_database(database: &Path) -> PathBuf {
    let mut path = database.as_os_str().to_os_string();
    path.push(".linura-recovery-reserve");
    PathBuf::from(path)
}

fn fill_reserve_bytes(buffer: &mut [u8], offset: u64) {
    let mut state = offset ^ 0x9e37_79b9_7f4a_7c15;
    for byte in buffer {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state >> 24) as u8;
    }
}

fn sync_parent(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()
}

fn verify_physical_allocation(file: &File, expected_len: u64) -> io::Result<()> {
    let metadata = file.metadata()?;
    if metadata.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "filesystem recovery reserve length changed unexpectedly",
        ));
    }
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::MetadataExt;
        let allocated = metadata.blocks().saturating_mul(512);
        if allocated < expected_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "filesystem recovery reserve is sparse or not physically allocated",
            ));
        }
    }
    Ok(())
}

fn rollback_growth(file: &File, original: u64) {
    let _ = file.set_len(original);
    let _ = file.sync_all();
}

fn reserve_result(result: io::Result<()>) -> i64 {
    match result {
        Ok(()) => 1,
        Err(error) if error.raw_os_error() == Some(28) => 0,
        Err(_) => -1,
    }
}

fn io_store(error: io::Error) -> TransactionStoreError {
    if error.raw_os_error() == Some(28) {
        TransactionStoreError::CapacityExceeded
    } else if error.kind() == io::ErrorKind::InvalidData {
        TransactionStoreError::Corruption(error.to_string())
    } else {
        TransactionStoreError::Storage(error.to_string())
    }
}

fn sqlite_store(error: rusqlite::Error) -> TransactionStoreError {
    if let rusqlite::Error::SqliteFailure(code, _) = &error
        && code.code == rusqlite::ErrorCode::DiskFull
    {
        return TransactionStoreError::CapacityExceeded;
    }
    TransactionStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_RESERVE: AtomicU64 = AtomicU64::new(0);

    fn temporary_database_path() -> PathBuf {
        let sequence = NEXT_RESERVE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "linura-v04-filesystem-reserve-{}-{sequence}.db",
            std::process::id()
        ))
    }

    #[test]
    fn reserve_growth_is_physical_and_release_is_slot_aligned() {
        let database = temporary_database_path();
        let path = reserve_path_for_database(&database);
        let reserve = RecoveryReserve {
            path: path.clone(),
            slot_bytes: 64 * 1024,
        };
        let _ = fs::remove_file(&path);

        reserve
            .ensure_slots(3)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            fs::metadata(&path)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len(),
            3 * 64 * 1024
        );
        reserve
            .release_to_slots(2)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            fs::metadata(&path)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len(),
            2 * 64 * 1024
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn one_slot_deficit_is_only_a_live_recovery_state() {
        let database = temporary_database_path();
        let path = reserve_path_for_database(&database);
        let reserve = RecoveryReserve {
            path: path.clone(),
            slot_bytes: 64 * 1024,
        };
        let _ = fs::remove_file(&path);
        reserve
            .ensure_slots(2)
            .unwrap_or_else(|error| unreachable!("{error}"));
        reserve
            .release_to_slots(1)
            .unwrap_or_else(|error| unreachable!("{error}"));
        reserve
            .validate_and_reconcile(1)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn filesystem_reconciliation_requires_database_write_serialization() {
        let database = temporary_database_path();
        let path = reserve_path_for_database(&database);
        let _ = fs::remove_file(&database);
        let _ = fs::remove_file(&path);

        let connection = Connection::open(&database)
            .unwrap_or_else(|error| unreachable!("{error}"));
        connection
            .execute_batch("CREATE TABLE audit_reservations (slot INTEGER NOT NULL);")
            .unwrap_or_else(|error| unreachable!("{error}"));
        connection
            .execute("INSERT INTO audit_reservations(slot) VALUES (0), (1)", [])
            .unwrap_or_else(|error| unreachable!("{error}"));
        let page_size: i64 = connection
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .unwrap_or_else(|error| unreachable!("{error}"));
        let page_size = u64::try_from(page_size)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let reserve = RecoveryReserve::from_connection(&connection, page_size)
            .unwrap_or_else(|error| unreachable!("{error}"));
        reserve
            .ensure_slots(3)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let before = fs::metadata(&path)
            .unwrap_or_else(|error| unreachable!("{error}"))
            .len();

        assert!(matches!(
            validate_filesystem_reserve(&connection, page_size, 2),
            Err(TransactionStoreError::StateConflict)
        ));
        assert_eq!(
            fs::metadata(&path)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len(),
            before
        );

        connection
            .execute_batch("BEGIN IMMEDIATE")
            .unwrap_or_else(|error| unreachable!("{error}"));
        validate_filesystem_reserve(&connection, page_size, 2)
            .unwrap_or_else(|error| unreachable!("{error}"));
        connection
            .execute_batch("COMMIT")
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            fs::metadata(&path)
                .unwrap_or_else(|error| unreachable!("{error}"))
                .len(),
            before
        );

        drop(connection);
        let _ = fs::remove_file(database);
        let _ = fs::remove_file(path);
    }
}
