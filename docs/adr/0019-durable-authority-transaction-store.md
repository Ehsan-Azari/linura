# ADR 0019: Durable authority transactions use a pure transaction domain with a local SQLite/WAL adapter

Status: Accepted

## Context

Linura v0.3 established exact-bound policy review and short-lived approval semantics over the canonical deterministic `ReconciliationPlan`. It deliberately stopped before the canonical `prepare` stage: policy allow, valid approval evidence and a reviewed plan are evidence, not execution authority.

v0.4 must establish the durable authority state required before any future executor can be trusted with an external effect. The roadmap requires restart-stable request idempotency, exact reviewed-plan/authorization/evidence binding, durable prepare records, explicit indeterminate-operation recovery, verified commit state, append-only audit foundations, migrations/versioning and corruption handling. It must prove those properties without introducing an executor, Polkit integration, public apply path or supported machine mutation.

A local database also cannot be treated as current Linux truth. External effects cannot be atomically committed with local database state, so recovery after an ambiguous effect boundary must re-observe authoritative machine state instead of replaying an operation merely because a local record exists.

## Decision

### 1. Separate transaction semantics from concrete persistence

v0.4 introduces two boundaries:

- `linura-transaction`: transport-neutral, persistence-neutral durable transaction domain semantics, exact authority-binding material, state transitions, recovery decisions and repository traits;
- `linura-persistence-sqlite`: the local SQLite implementation of those repository contracts, including schema migrations, transactional writes, integrity checks and append-only audit storage.

`linura-control` owns projection from trusted v0.3 review/approval material into `linura-transaction` authority-binding material. It may depend on `linura-transaction`, but not on the concrete SQLite adapter. The daemon/application composition root may wire the SQLite adapter to Control later without moving policy semantics into persistence.

Neither new crate may consume `linura-policy`, D-Bus, concrete Linux providers or privileged executors. `linura-control` remains the sole workspace consumer/orchestrator of `linura-policy`.

### 2. SQLite/WAL is the local v0.4 authority store

The default local implementation is SQLite in WAL mode. It is embedded and local-first, requires no service database, provides transactional durability and recovery semantics suitable for one-machine authority state, and keeps hosted/fleet infrastructure optional.

The adapter must configure and verify at least:

- WAL journal mode;
- `synchronous=FULL` for authority-state durability;
- foreign-key enforcement;
- `trusted_schema=OFF`;
- a bounded busy timeout rather than unbounded lock waiting;
- a repository-owned SQLite `application_id`;
- explicit schema/user versioning and migration checksums;
- transactional migrations;
- bounded database/audit growth with fail-closed capacity errors rather than silent authority/audit eviction.

The implementation may use the bundled SQLite build to make the repository's qualification environment deterministic and independent of a host distribution's SQLite package version.

### 3. Prepared authority is exact-bound and non-executable

A durable prepare record is identified by a typed transaction ID and binds at minimum:

- authenticated principal;
- request ID;
- canonical plan ID;
- authoritative observation/evidence identity;
- provider/resource/capability identity;
- policy ID and revision;
- trusted resulting risk and risk-policy provenance;
- material reviewed changes/findings and semantic provenance through a deterministic content digest;
- the exact authorization basis: policy allow or valid approval evidence, including approval identity/approver/validity material when approval is required.

Control must recompute this binding from trusted current review material immediately before creating or reusing a durable prepare record. A `PlanId`, request ID, approval ID or transaction ID alone is never sufficient.

The canonical binding digest uses deterministic domain-separated, length-delimited encoding before SHA-256 hashing. Ambiguous string concatenation, map iteration order and debug formatting are not authority encodings.

A `Prepared` record does **not** contain an executor handle, Polkit grant or capability to perform an external effect. It proves only that an exact reviewed authority subject crossed the durable `prepare` boundary.

### 4. Idempotency survives restart

The durable idempotency namespace is the authenticated principal plus request ID. Reuse rules are fail-closed:

- same principal + request ID + exact binding digest returns the existing transaction;
- same principal + request ID + different binding material is an idempotency conflict;
- a different principal is a different authority namespace;
- terminal or recovery state does not permit request identity to be silently rebound to different authority material.

SQLite uniqueness constraints backstop these semantics so process restart cannot reopen a request ID that an in-memory map forgot.

### 5. Transaction states model ambiguity explicitly

The v0.4 transaction domain includes bounded, validated transitions for:

- `Prepared` — exact durable intent-to-execute authority record exists;
- `Indeterminate` — a future effect boundary may have been crossed but the outcome is not safely known;
- `Verified` — fresh authoritative re-observation proves the intended postcondition for this exact transaction;
- `Committed` — verified durable commit metadata for desired-state/graph/provenance references is atomically recorded;
- `Aborted` — the transaction ended before any ambiguous external effect, or recovery proved a safe non-effect terminal path;
- `RecoveryBlocked` — authoritative recovery evidence conflicts with safe automatic continuation.

State transitions are monotonic except for one explicit recovery path from `Indeterminate` back to a new `Prepared` attempt **only when fresh authoritative observation proves that the intended effect did not occur**. That recovery increments an attempt/generation counter and emits an immutable audit event; it is not blind replay.

No v0.4 public or daemon path dispatches an external effect. The `Indeterminate` model is present so v0.5/v0.6 cannot later invent weaker crash semantics around an executor.

### 6. Recovery never trusts local persistence as machine truth

An indeterminate transaction requires a fresh authoritative recovery observation. Recovery classifies the observation as one of:

- intended state verified → `Verified`;
- intended effect proven absent → a new `Prepared` attempt may become eligible;
- conflicting state → `RecoveryBlocked`;
- insufficient/stale/ambiguous evidence → remain `Indeterminate`.

A restart alone, retry request, daemon PID change, SQLite row state or executor self-report must never authorize redispatch.

Observed Linux state continues to come from the authoritative observation/provider path. The transaction store retains evidence identity/digests and recovery decisions but does not fabricate current state.

### 7. Commit requires prior verification

`Committed` is reachable only from `Verified`. The commit operation atomically records the verified transaction state plus deterministic references/digests for the desired-state, graph and semantic-provenance material that later lifecycle integration will publish.

v0.4 does not claim the full persistent intent/Library model planned for v0.7. It establishes the transactionally verified commit boundary required for later managed effects.

### 8. Audit is append-only and integrity-checked

Every durable transaction transition emits an append-only audit event in the same SQLite transaction as the state change. Events include monotonic per-transaction sequence/generation information and an integrity digest chained to the previous event.

The SQLite schema prevents ordinary UPDATE/DELETE of audit rows. Store-open/integrity validation checks at minimum:

- SQLite integrity/foreign-key status;
- supported application/schema version;
- migration checksum identity;
- transaction binding digest format;
- audit event sequence and hash-chain continuity;
- consistency between a transaction's current state/generation and its terminal retained audit event.

Detected corruption, unsupported newer schema, migration mismatch or broken audit continuity fails closed. Automatic repair must not silently rewrite authority history.

### 9. Authority-state writes are bounded

Persisted authority material has explicit per-record and aggregate bounds. Oversized review/audit payloads and capacity exhaustion are rejected before unbounded cloning/serialization where practical. Authority records and audit history are never silently evicted to create room for newer authority.

Operational maintenance such as checkpointing is allowed; semantic history deletion requires a future explicit retention/archival policy and is not introduced in v0.4.

## Security assessment

This ADR is a security-boundary change. Required negative proofs include:

- request-ID reuse with changed binding is rejected across process/database reopen;
- plan/evidence/policy/risk/approval substitution changes the exact binding and cannot reuse prepare authority;
- expired/revoked/mismatched required approval cannot create a prepare record;
- deny/blocked/no-change review states cannot become prepared mutation authority;
- malformed/unsupported/corrupted database state fails closed;
- audit UPDATE/DELETE/tampering is detected or rejected;
- a crash/reopen cannot turn `Indeterminate` into retry authority;
- only fresh authoritative evidence proving no effect permits a new prepared attempt;
- commit before verification is rejected;
- no API introduced by v0.4 reaches an executor, Polkit or external Linux mutation.

## Consequences

- v0.4 makes the review-to-prepare TOCTOU boundary explicit and durable without prematurely adding execution.
- SQLite becomes an implementation detail behind a transaction repository contract rather than a dependency of policy/planning semantics.
- v0.5 can qualify a narrow executor against a pre-existing prepared-transaction/recovery model instead of inventing durability while privileged code is being introduced.
- v0.6 can integrate the executor/verifier with durable prepare, verification, commit, audit and reconciliation through the canonical lifecycle.
- Recovery correctness intentionally depends on authoritative re-observation, not local database confidence.
- The local authority path remains standalone; hosted/fleet databases or services are not prerequisites.
