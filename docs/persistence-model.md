# Persistence model

Linura needs crash-safe local persistence for durable intent, reusable configuration and evidence while continuing to treat Linux providers as authoritative for actual current machine state.

## Durable entities
- intents/requirements and lifecycle lineage;
- reusable Setup revisions/composition and Setup → adopted-intent lineage;
- machine profiles and adopted profile lineage;
- local Linura Library metadata/index/history;
- desired resources;
- system-graph managed edges/metadata;
- semantic provenance;
- policy grants/approvals;
- action/request idempotency;
- audit events and verification evidence references;
- reconciliation/drift state;
- schema/version metadata.

## Transaction boundaries

A plan may cause external effects that cannot be atomically committed with a local database. The runtime therefore records intent-to-execute state before effect dispatch and finalizes only after re-observation. Crash recovery re-observes indeterminate effects before retry.

Setup/profile save/export operations do not create machine authority. Adoption materializes locally validated intent/setup lineage and then enters the ordinary observed-state/planning/authority path.

## Expected local implementation

SQLite/WAL remains the leading default because Linura is local-first and should not require a service database. Selection is finalized by ADR after event/provenance schemas stabilize.

The first Linura Library implementation should use this local durable layer or another local adapter selected by ADR. Hosted/cloud/Git/user-owned synchronization remains optional and must not be required for local operation.

## Backup, export and snapshots

Three operations stay distinct:

1. **Authority-state backup** — local database/evidence needed to recover Linura's own durable state.
2. **Portable setup/profile export** — self-contained declarative setups/intents/constraints suitable for reuse; excludes secret values, raw audit history and machine-specific observations.
3. **System/filesystem snapshot** — exact machine recovery point for rollback.

A portable export is not a database backup or a snapshot. A snapshot is not a trusted portable configuration source.
