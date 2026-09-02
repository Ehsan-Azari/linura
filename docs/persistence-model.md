# Persistence model

Linura needs durable local persistence for retained authority/transaction facts while continuing to treat authoritative Linux observation as the source of actual current machine state.

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
- durable prepare/pre-dispatch/recovery transaction state;
- audit events and verification evidence references;
- reconciliation/drift state;
- schema/version metadata.

## Transaction boundaries

A plan may eventually cause external effects that cannot be atomically committed with a local database. The runtime therefore records exact reviewed authority as durable intent-to-execute state before any future effect dispatch and finalizes only after authoritative re-observation and verification.

The v0.4 transaction foundation keeps six durable semantic states: `Prepared`, `Indeterminate`, `Verified`, `Committed`, `Aborted`, and `RecoveryBlocked`. A prepared transaction is not an executor credential.

A future executor must not be called while the current generation is merely `Prepared`. Before dispatch, Control must atomically persist the generation-bound `Prepared` → `Indeterminate` transition plus its audit event; only after that commit may a future dispatch capability exist. This prevents a crash from leaving an actually attempted effect represented as an unattempted prepared row.

After `Indeterminate`, restart never means retry. Fresh authoritative observation must resolve the ambiguity. Proof of intended state may advance to `Verified`. Proof that the effect did not occur permits only current-authority revalidation/re-prepare eligibility; it does not itself authorize redispatch. Conflicting state becomes `RecoveryBlocked`; stale/ambiguous evidence leaves the transaction `Indeterminate`.

Durable idempotency is scoped by authenticated principal + request identity and exact authority-binding digest. Reusing the same durable request namespace with changed plan/complete-observation/policy/risk/approval material is a conflict rather than a new transaction.

Setup/profile save/export operations do not create machine authority. Adoption materializes locally validated intent/setup lineage and then enters the ordinary observed-state/planning/authority path.

## Complete observation binding

`evidence_id` is a reference, not a complete authority/freshness binding. v0.4 must retain or deterministically digest the complete validated authoritative observation envelope used by planning/review, including provider/resource/capability, authority, observation time/sequence, validity/freshness material and canonical observed attributes.

Control uses Control-owned time immediately before prepare to verify that this exact observation remains fresh and matches the trusted canonical plan/review. Changed or expired observation requires fresh authoritative observation followed by replan/review; it cannot reuse old prepare authority merely because an evidence ID is unchanged.

Recovery likewise consumes fresh authoritative observation rather than database/executor assertions.

## Local implementation

ADR 0019 selects a persistence-neutral `linura-transaction` domain plus a concrete `linura-persistence-sqlite` adapter. SQLite/WAL is the default local authority store because Linura is local-first and must not require a service database.

The v0.4 adapter uses WAL with `synchronous=FULL`, foreign-key enforcement, `trusted_schema=OFF`, bounded lock waiting, a repository-owned application ID, explicit schema/user versioning and transactional migration checksums. Authority-state and audit growth are bounded and fail closed rather than silently evicting history.

Every transaction state change writes an append-only audit event atomically with the state update. Audit events retain deterministic sequence/generation data and chained integrity digests. Store-open/integrity validation checks SQLite integrity/foreign keys, supported schema/application identity, migration checksums, authority-binding digest shape, retained audit-chain continuity and transaction-state/event consistency. Detected corruption or an unsupported newer schema is diagnosable and fail-closed; automatic repair must not rewrite authority history.

An internal unkeyed hash chain strengthens consistency of the retained history but is not an external anti-rollback anchor. A complete older database copy can itself be internally consistent. v0.4 therefore does not support transparent authority-database rollback/restore or host/VM snapshot rollback. A future supported restore protocol requires an independently protected monotonic epoch/anchor or an explicit authority invalidation/re-establishment design.

The first Linura Library implementation should use this local durable layer or another local adapter selected by a future explicit ADR. Hosted/cloud/Git/user-owned synchronization remains optional and must not be required for local operation.

## Durability qualification boundary

SQLite durability depends on the filesystem/storage honoring required locking and synchronization semantics. The v0.4 release claim is therefore scoped to its explicitly qualified disposable-VM local filesystem/storage profile and must record those assumptions.

Qualification covers more than graceful process reopen:
- `SIGKILL` around durable transaction commit points;
- abrupt disposable guest power interruption after acknowledged durable transitions;
- WAL/checkpoint-sensitive recovery where deterministically injectable;
- representative write/I/O failure behavior;
- integrity/state/audit validation after reopen.

A recovered database must expose either the complete prior state or the complete committed state for a semantic transition, never a half state/audit pair. Failed SQLite commits cannot advance authority state. `synchronous=FULL` is required for the qualified profile but is not represented as a universal hardware durability guarantee.

Coherent host/hypervisor/filesystem snapshot rollback is a separate unsupported v0.4 condition, not part of the power-loss recovery claim.

## Authority versus machine truth

Persistence is authoritative for Linura's own retained transaction/audit facts within one monotonic database history, but never for what is currently true on Linux. Current machine truth is re-derived from authoritative providers whenever planning, prepare freshness validation, recovery, verification or reconciliation depends on it.

A cached or persisted statement that an effect succeeded cannot substitute for post-effect authoritative observation. Executor self-reporting likewise cannot close an indeterminate transaction.

## Backup, export and snapshots

Three operations stay distinct:

1. **Authority-state backup** — local database/evidence needed to recover Linura's own durable state. Transparent restore is not yet a supported v0.4 authority operation because coherent rollback needs an external epoch/anchor or explicit invalidation protocol.
2. **Portable setup/profile export** — self-contained declarative setups/intents/constraints suitable for reuse; excludes secret values, raw audit history and machine-specific observations.
3. **System/filesystem snapshot** — exact machine recovery point for rollback. Restoring one may also roll authority state backward and is therefore not transparent/supported by the v0.4 authority model.

A portable export is not a database backup or a snapshot. A snapshot is not a trusted portable configuration source.
