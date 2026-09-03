#![forbid(unsafe_code)]

//! Hardened SQLite/WAL persistence for Linura durable authority transactions.
//!
//! The database is treated as untrusted persistence input. Control-signed
//! handoff/recovery/commit requests still authorize semantic transitions, while
//! a separate record-integrity key authenticates the durable SQLite rows
//! themselves. A process that can issue arbitrary SQL against the database
//! therefore cannot forge a trusted transaction state, generation, or audit
//! record merely by reproducing schema objects or application-defined SQLite
//! functions.

#[rustfmt::skip]
mod integrity;
#[rustfmt::skip]
mod schema;
#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
mod store;
#[rustfmt::skip]
mod validation;

pub use integrity::SqliteIntegrityKey;
pub use store::{SqliteSettings, SqliteTransactionStore, StoreLimits};

#[cfg(test)]
#[rustfmt::skip]
mod tests;
