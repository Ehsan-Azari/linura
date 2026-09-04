# ADR 0020: Durable mutation authority is sealed across Control and persistence

Status: Accepted

## Context

ADR 0019 establishes the v0.4 durable authority transaction model: exact-bound prepare records, fresh authority revalidation, one-shot pre-dispatch handoff, explicit indeterminate recovery, verified commit, immutable history, and a local SQLite/WAL adapter.

Security review of the implementation exposed an additional boundary requirement. A repository trait is still an authority surface if arbitrary callers can construct the handoff, recovery, or commit request accepted by that trait. Protecting only the orchestration path is insufficient when a direct persistence caller—or an independently opened SQLite connection—could otherwise request or synthesize an authority-sensitive state transition.

This ADR refines ADR 0019 where its earlier description of an entirely internal handoff authorization value is narrower than the implemented cross-crate and storage contract.

## Decision

### 1. Control owns signing authority; persistence receives verification authority only

The trusted composition root provisions one 256-bit durable mutation authority key and splits it into two non-cloneable capabilities:

- `TransactionAuthoritySigner`, owned by `linura-control`, which can seal exact handoff, recovery, and commit requests;
- `TransactionAuthorityVerifier`, owned by the persistence adapter, which can validate sealed requests but cannot construct them through the public API.

The root key and signer are not persisted in SQLite. The database stores only a domain-separated verifier fingerprint and immutably pins that identity when the authority store is initialized. Reopening the same authority database with a different verifier fails closed.

All in-process root/signer/verifier secret holders are non-`Clone`, redact `Debug`, and explicitly zeroize their 256-bit key material on drop using non-elidable secret-memory writes. Rejected owned key buffers are scrubbed before validation errors return.

Production composition must therefore provision the same protected authority key across supported process restarts. Test fixtures may use deterministic keys only inside qualification code.

### 2. Authority-sensitive request construction is sealed

`HandoffRequest`, `RecoveryRequest`, and `CommitRequest` are public transport-neutral domain types because they cross the transaction-store boundary, but their authority-bearing fields are private. Public callers cannot construct arbitrary values with struct literals.

Control obtains these values only through signer methods that bind the complete expected mutation subject, including transaction identity, generation, state version, binding digest, authority-validity window where applicable, and operation-specific material. Requests are authenticated with domain-separated HMAC-SHA-256 tags.

SQLite verifies the tag before opening the authority-state write transaction. A request sealed by any other signer is rejected before state, state-version, generation, provenance, or audit history can change.

The authentication tag is not a reusable executor credential. It authorizes only the exact durable compare-and-swap encoded in the request. The process-local `DispatchPermit` is still minted exclusively for the caller that wins the `Prepared -> Indeterminate` CAS and remains non-cloneable, non-persistable, and non-reconstructible from SQLite.

### 3. Handoff is bound to the current authenticated principal

Possession of a `PreparedDurableAuthority` value is not sufficient handoff authority. The handoff API requires the current transport-derived `AuthenticatedPrincipal` and compares it against the principal retained in the prepared candidate, canonical authority binding, and durable snapshot before current authority is revalidated or a sealed handoff request is created.

A prepared object transferred across sessions or identities therefore cannot be used to mint a dispatch permit for another principal.

### 4. Recovery freshness is checked at the terminal serialization boundary

Recovery observation validity is not treated as a property established once at provider-read time. For terminal recovery outcomes that would move an indeterminate generation to `Verified` or `RecoveryBlocked`, Control samples its authority clock again immediately before sealing the recovery request.

Handoff and recovery envelopes carry signer-authenticated `authorized_at_unix_ms` / `expires_at_unix_ms` values. SQLite verifies the HMAC, obtains `BEGIN IMMEDIATE`, and only then samples wall time and enforces the sealed interval before any authority mutation. SQLite busy waits therefore cannot extend observation or approval authority past its deadline. Approval expiry retains its exclusive seconds boundary when converted to milliseconds.

The no-effect/reprepare path continues to perform complete current authority re-establishment, including observation freshness, policy, risk provenance, and approval validation, before appending generation N+1.

### 5. Capacity is a recoverability invariant, including real filesystem capacity

Store limits are recovery invariants. `integrity_check` validates aggregate row counts for transactions, generations, and audit events against configured `StoreLimits` whenever the authority database is opened.

The store also reserves at least one future logical audit slot and one preallocated SQLite page-reservation record for every current nonterminal transaction (`Prepared`, `Indeterminate`, or `Verified`). Admission accounts for events consumed by the transition and the resulting set of nonterminal transactions.

SQLite page availability is not treated as equivalent to backing-filesystem availability. A separate same-directory recovery-reserve sidecar is physically written and `fsync`ed on the same filesystem as the authority database. The sidecar retains emergency WAL/filesystem headroom in addition to the in-database `audit_reservations`. Store-owned reservation changes reconcile that sidecar conservatively: failed database writes may leave excess reserved bytes, but must not leave less than the recoverability invariant. Terminal retirement consumes already reserved headroom before the corresponding SQLite/WAL write can require new filesystem blocks; nonterminal advancement must first retain the required future reserve.

A permanent disposable-VM qualification mounts the authority database on a dedicated ext4 filesystem, drives that filesystem to genuine `ENOSPC`, starts a new process, retires a durable `Prepared` transaction to `Aborted`, then unmounts/remounts the same filesystem and verifies the terminal record. `PRAGMA max_page_count` tests remain useful but are not represented as evidence for real filesystem exhaustion.

### 6. Retained history is immutable independently of caller PRAGMAs

`recursive_triggers=ON` remains a validated runtime setting for the adapter connection, but retained-history security does not depend on an independently opened connection inheriting that PRAGMA.

The canonical schema includes `BEFORE INSERT` conflict guards for transaction identity (including `(principal, request_id)`), generation identity, audit-event identity, migration identity, and verifier identity. Conflicting `INSERT OR REPLACE` operations are therefore rejected before SQLite can perform an implicit replacement delete, even from a connection whose `recursive_triggers` setting is off.

The schema also keeps fail-closed update/delete guards for immutable authority, migration, audit, and committed provenance material. Live `sqlite_schema` fingerprint validation covers the complete guard set.

### 7. Raw SQLite mutation is detected cryptographically and fails closed

The SQLite adapter does **not** rely on a connection-local SQL function name as an unforgeable mutation capability. Independently opened writers can physically alter SQLite bytes if operating-system permissions allow them to write the database.

Instead, a separately provisioned `SqliteIntegrityKey` authenticates every trusted transaction, generation, and audit record with domain-separated keyed tags. SQLite stores only the integrity-key fingerprint. The integrity key is distinct from Control's mutation signer: it authenticates durable storage facts and cannot mint `HandoffRequest`, `RecoveryRequest`, `CommitRequest`, or a `DispatchPermit`.

Raw SQLite writes can physically alter a row, but a writer that does not possess the record-integrity key cannot produce a trusted replacement tag. Current-row loads, audit extension, reopen validation, and `integrity_check` authenticate retained records and fail closed on mismatches. Structural schema guards remain defense in depth for accidental or unsophisticated mutation; they are not represented as the cryptographic trust boundary.

This keyed-record design does not solve coherent rollback of the entire database/filesystem/VM to an older previously valid snapshot. Detecting that class of rollback requires a future independently protected monotonic anchor or restore protocol and remains outside the supported v0.4 threat boundary.

### 8. Verified commit material is durable and restart-resumable

A successful `Indeterminate -> Verified` transition durably records the exact desired-state, graph, and provenance digests that Control derived from the authoritative recovery evidence. Those digests are included in the signer-authenticated recovery request and are persisted atomically with the transition to `Verified`.

A subsequent `CommitRequest` must match the same verified snapshot and those exact persisted digests. Commit changes only `Verified -> Committed`; it cannot replace the durable verified commit material.

After restart or a retryable persistence failure, Control may reconstruct a process-local `VerifiedDurableAuthority` only by loading `VerifiedCommitMaterial` from the current durable `Verified` generation and matching the current authenticated principal. This resumes the already-verified commit capability without reconstructing approval, policy, handoff, dispatch, or executor authority and without recomputing a different provenance chain.

### 9. Persisted authority data is bounded before materialization

The canonical generation schema enforces the domain maximum for `binding_canonical`. Reopen-time integrity validation queries `length(binding_canonical)` first and conditionally materializes the BLOB only when it is within `MAX_AUTHORITY_BINDING_BYTES`. Audit text and digest fields receive the same schema/query preflight treatment.

Live-schema validation additionally bounds both individual schema fields and the aggregate number/encoded byte size of schema objects before accumulating their canonical fingerprint input. A legacy, tampered, or otherwise malformed database therefore fails closed with a typed corruption/schema error rather than allocating attacker-controlled persistence input up to the database size.

### 10. The authority-store schema is a repository-visible migration

The durable SQLite format is not defined only by an embedded Rust string. Migration `0001-v04-hardened-authority-transactions` has a versioned descriptor under `migrations/system/`.

Its explicit preconditions require a fresh SQLite identity (`application_id == 0`, `user_version == 0`), no non-SQLite application schema objects, and provisioned verifier/integrity identities. Application remains idempotent through the SQLite identity checks. Before the migration is considered installed, the adapter verifies the live canonical schema and the `schema_migrations` ledger entry whose checksum is domain-separated as `linura.sqlite.migration.v1`. A precondition or verification mismatch fails closed; released migration IDs are never reused.

## Security assessment

Required negative proofs for this boundary include:

- a handoff request sealed by the wrong signer is rejected without changing the transaction snapshot or audit count;
- a recovery request sealed by the wrong signer is rejected without changing the transaction snapshot or audit count;
- a commit request must match signer-bound durable verified commit material;
- an existing authority database rejects a different verifier or record-integrity key on reopen;
- handoff rejects a current authenticated principal that does not equal the prepared candidate, canonical binding, and durable snapshot principal;
- terminal recovery cannot consume an observation or approval that has expired by the final serialized mutation point;
- reopening an existing database fails closed when aggregate transaction, generation, audit, schema-fingerprint, or nonterminal reserve limits are violated;
- a raw SQLite connection with default PRAGMAs cannot replace retained authority history without subsequent keyed validation detecting the mutation;
- a writer without `SqliteIntegrityKey` cannot create a trusted current generation or audit tail merely by making rows self-consistent;
- oversized persisted binding/audit/schema inputs are rejected before unbounded Rust materialization;
- a durable `Verified` generation can be resumed after restart using exactly its persisted signer-bound commit material;
- a real ext4 `ENOSPC` condition still permits the qualified terminal retirement path using physically reserved same-filesystem headroom;
- migration discovery can identify the durable authority-store format and independently verify its ID/preconditions/ledger contract;
- none of these mechanisms creates executor, Polkit, or external Linux mutation authority in v0.4.

## Consequences

- The transaction trait remains persistence-neutral without leaving authority-sensitive transitions freely callable.
- Control remains the policy/approval/observation authority owner while SQLite remains a mechanical durable verifier and state machine.
- Restart requires explicit protected-key provisioning rather than silently changing the authority identity of an existing database.
- Restart after durable verification does not lose the exact commit evidence chain or require recomputation.
- Raw database write permission is not confused with trusted authority-state authorship; keyed record authentication detects unauthorized durable-state fabrication.
- Logical audit capacity, SQLite page capacity, and real filesystem/WAL headroom are distinct invariants and are qualified independently.
- The authority schema is registered in the repository migration catalog instead of existing only inside adapter source.
- ADR 0019 remains the primary durable-transaction design; this ADR is the authoritative refinement for sealed mutations, current-principal binding, serialized freshness, verifier/integrity identity, immutable authenticated storage, durable verified resume, bounded persistence input, migration registration, and recoverability capacity.
