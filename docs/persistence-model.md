# Persistence model

Linura needs crash-safe local persistence for durable intent and evidence while continuing to treat Linux providers as authoritative for actual current machine state.

## Durable entities
- intents/requirements and lifecycle lineage;
- machine profiles and adopted profile lineage;
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

## Expected local implementation

SQLite/WAL remains the leading default because Linura is local-first and should not require a service database. Selection is finalized by ADR after event/provenance schemas stabilize.

## Backup/export

Portable profile export contains intent/constraints rather than secrets or raw audit history. Backup/recovery of local authority metadata is distinct from portable profile replay.
