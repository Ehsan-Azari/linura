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

A plan may cause external effects that cannot be atomically committed with a local database. The runtime therefore records exact reviewed authority as durable intent-to-execute state before any future effect dispatch and finalizes only after authoritative re-observation and verification. Crash recovery re-observes indeterminate effects before retry; restart or a retained database row is never sufficient retry authority.

The v0.4 transaction foundation keeps six durable semantic states: `Prepared`, `Indeterminate`, `Verified`, `Committed`, `Aborted`, and `RecoveryBlocked`. A prepared transaction is not an executor credential. `Indeterminate` may return to a new prepared attempt only when fresh authoritative observation proves the intended effect did not occur; otherwise it remains indeterminate, advances to verified when the intended state is proven, or becomes recovery-blocked on conflicting state.

Durable idempotency is scoped by authenticated principal + request identity and exact authority-binding digest. Reusing the same durable request namespace with changed plan/evidence/policy/risk/approval material is a conflict rather than a new transaction.

Setup/profile save/export operations do not create machine authority. Adoption materializes locally validated intent/setup lineage and then enters the ordinary observed-state/planning/authority path.

## Local implementation

ADR 0019 selects a persistence-neutral `linura-transaction` domain plus a concrete `linura-persistence-sqlite` adapter. SQLite/WAL is the default local authority store because Linura is local-first and must not require a service database.

The v0.4 adapter uses WAL with `synchronous=FULL`, foreign-key enforcement, `trusted_schema=OFF`, bounded lock waiting, a repository-owned application ID, explicit schema/user versioning and transactional migration checksums. Authority-state and audit growth are bounded and fail closed rather than silently evicting history.

Every transaction state change writes an append-only audit event atomically with the state update. Audit events retain deterministic sequence/generation data and chained integrity digests. Store-open/integrity validation checks SQLite integrity/foreign keys, supported schema/application identity, migration checksums, authority-binding digest shape, audit-chain continuity and transaction-state/event consistency. Corruption or an unsupported newer schema is diagnosable and fail-closed; automatic repair must not rewrite authority history.

The first Linura Library implementation should use this local durable layer or another local adapter selected by a future explicit ADR. Hosted/cloud/Git/user-owned synchronization remains optional and must not be required for local operation.

## Authority versus machine truth

Persistence is authoritative for Linura's own retained transaction/audit facts, but never for what is currently true on Linux. Current machine truth is re-derived from authoritative providers whenever planning, recovery, verification or reconciliation depends on it.

A cached or persisted statement that an effect succeeded cannot substitute for post-effect authoritative observation. Executor self-reporting likewise cannot close an indeterminate transaction.

## Backup, export and snapshots

Three operations stay distinct:

1. **Authority-state backup** — local database/evidence needed to recover Linura's own durable state.
2. **Portable setup/profile export** — self-contained declarative setups/intents/constraints suitable for reuse; excludes secret values, raw audit history and machine-specific observations.
3. **System/filesystem snapshot** — exact machine recovery point for rollback.

A portable export is not a database backup or a snapshot. A snapshot is not a trusted portable configuration source.
