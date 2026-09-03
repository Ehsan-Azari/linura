#![forbid(unsafe_code)]

//! Hardened SQLite/WAL persistence for Linura durable authority transactions.
//!
//! The database is treated as untrusted persistence input. Control-signed
//! handoff/recovery/commit requests authorize semantic transitions, while a
//! separate record-integrity key authenticates durable SQLite records. The
//! filesystem recovery reserve is an independent availability invariant: it
//! keeps physically allocated same-filesystem headroom for terminal recovery
//! when SQLite/WAL reaches real ENOSPC.

mod filesystem_reserve;
mod integrity;
mod schema;
mod store;

// Record/history decoding remains isolated from connection/schema validation so
// aggregate schema bounds and external recovery-reserve checks can fail closed
// before attacker-controlled schema material is accumulated in memory.
#[allow(dead_code)]
#[path = "validation_base.rs"]
mod validation_base;
#[path = "validation_hardened.rs"]
mod validation;

pub use integrity::SqliteIntegrityKey;
pub use store::{SqliteSettings, SqliteTransactionStore, StoreLimits};

#[cfg(test)]
mod tests;
