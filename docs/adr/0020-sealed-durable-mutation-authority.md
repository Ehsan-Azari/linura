# ADR 0020: Durable mutation authority is sealed across Control and persistence

Status: Accepted

## Context

ADR 0019 establishes the v0.4 durable authority transaction model: exact-bound prepare records, fresh authority revalidation, one-shot pre-dispatch handoff, explicit indeterminate recovery, verified commit, immutable history, and a local SQLite/WAL adapter.

Security review of the implementation exposed an additional boundary requirement. A repository trait is still an authority surface if arbitrary callers can construct the handoff or recovery request accepted by that trait. Protecting only the orchestration path is insufficient when a direct persistence caller could otherwise request an authority-sensitive state transition.

This ADR refines ADR 0019 where its earlier description of an entirely internal handoff authorization value is narrower than the implemented cross-crate contract.

## Decision

### 1. Control owns signing authority; persistence receives verification authority only

The trusted composition root provisions one 256-bit durable mutation authority key and splits it into two non-cloneable capabilities:

- `TransactionAuthoritySigner`, owned by `linura-control`, which can seal exact handoff and recovery requests;
- `TransactionAuthorityVerifier`, owned by the persistence adapter, which can validate sealed requests but cannot construct them through the public API.

The root key and signer are not persisted in SQLite. The database stores only a domain-separated verifier fingerprint and immutably pins that identity when the authority store is initialized. Reopening the same authority database with a different verifier fails closed.

All in-process root/signer/verifier secret holders are non-`Clone`, redact `Debug`, and explicitly zeroize their 256-bit key material on drop using the `zeroize` primitive rather than relying on an ordinary compiler-optimizable fill.

Production composition must therefore provision the same protected authority key across supported process restarts. Test fixtures may use deterministic keys only inside qualification code.

### 2. Authority-sensitive request construction is sealed

`HandoffRequest` and `RecoveryRequest` are public transport-neutral domain types because they cross the transaction-store boundary, but their authority-bearing fields are private. Public callers cannot construct arbitrary values with struct literals.

Control obtains these values only through signer methods that bind the complete expected mutation subject, including transaction identity, generation, state version, binding digest, and operation-specific material. Requests are authenticated with domain-separated HMAC-SHA-256 tags.

SQLite verifies the tag before opening the authority-state write transaction. A request sealed by any other signer is rejected before state, state-version, generation, or audit history can change.

The authentication tag is not a reusable executor credential. It authorizes only the exact durable compare-and-swap encoded in the request. The process-local `DispatchPermit` is still minted exclusively for the caller that wins the `Prepared -> Indeterminate` CAS and remains non-cloneable, non-persistable, and non-reconstructible from SQLite.

### 3. Handoff is bound to the current authenticated principal

Possession of a `PreparedDurableAuthority` value is not sufficient handoff authority. The handoff API requires the current transport-derived `AuthenticatedPrincipal` and compares it against the principal retained in the prepared candidate, canonical authority binding, and durable snapshot before current authority is revalidated or a sealed handoff request is created.

A prepared object transferred across sessions or identities therefore cannot be used to mint a dispatch permit for another principal.

### 4. Recovery freshness is checked at the terminal serialization boundary

Recovery observation validity is not treated as a property established once at provider-read time. For terminal recovery outcomes that would move an indeterminate generation to `Verified` or `RecoveryBlocked`, Control samples its monotonic authority clock again and requires the authoritative recovery observation to still be current immediately before sealing the recovery request and entering the durable compare-and-swap.

The no-effect/reprepare path continues to perform complete current authority re-establishment, including observation freshness, policy, risk provenance, and approval validation, before appending generation N+1.

### 5. Persistence enforces aggregate capacity on reopen as well as on writes

Store limits are recovery invariants, not only write-time admission controls. `integrity_check` validates aggregate row counts for transactions, generations, and audit events against the configured `StoreLimits` whenever the authority database is opened.

This prevents an existing database containing many individually valid transaction histories from being reopened under a stricter aggregate generation or audit limit without detection.

## Security assessment

Required negative proofs for this boundary include:

- a handoff request sealed by the wrong signer is rejected without changing the transaction snapshot or audit count;
- a recovery request sealed by the wrong signer is rejected without changing the transaction snapshot or audit count;
- an existing authority database rejects a different verifier on reopen;
- handoff rejects a current authenticated principal that does not equal the prepared candidate, canonical binding, and durable snapshot principal;
- terminal recovery cannot consume an observation that has expired by the final serialization point;
- reopening an existing database fails closed when aggregate transaction, generation, or audit counts exceed the configured limits;
- none of these mechanisms creates executor, Polkit, or external Linux mutation authority in v0.4.

## Consequences

- The transaction trait remains persistence-neutral without leaving authority-sensitive transitions freely callable.
- Control remains the policy/approval/observation authority owner while SQLite remains a mechanical durable verifier and state machine.
- Restart requires explicit protected-key provisioning rather than silently changing the authority identity of an existing database.
- Review findings about direct handoff/recovery bypasses are closed by contract, not by convention or call-site discipline.
- ADR 0019 remains the primary durable-transaction design; this ADR is the authoritative refinement for sealed mutation requests, current-principal handoff binding, terminal recovery freshness, verifier identity, and reopen-time aggregate capacity enforcement.
