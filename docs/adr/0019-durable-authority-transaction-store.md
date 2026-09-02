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

`linura-control` owns projection from trusted v0.3 review/approval/observation material into `linura-transaction` authority-binding material. It may depend on `linura-transaction`, but not on the concrete SQLite adapter. The daemon/application composition root may wire the SQLite adapter to Control later without moving policy semantics into persistence.

Neither new crate may consume `linura-policy`, D-Bus, concrete Linux providers or privileged executors. `linura-control` remains the sole workspace consumer/orchestrator of `linura-policy`.

### 2. SQLite/WAL is the local v0.4 authority store

The default local implementation is SQLite in WAL mode. It is embedded and local-first, requires no service database, provides transactional durability and recovery semantics suitable for one-machine authority state, and keeps hosted/fleet infrastructure optional.

The adapter must configure and verify at least:

- WAL journal mode;
- `synchronous=FULL` for authority-state durability within the qualified filesystem/storage contract;
- foreign-key enforcement;
- `trusted_schema=OFF`;
- a bounded busy timeout rather than unbounded lock waiting;
- a repository-owned SQLite `application_id`;
- explicit schema/user versioning and migration checksums;
- transactional migrations;
- bounded database/audit growth with fail-closed capacity errors rather than silent authority/audit eviction.

The implementation may use the bundled SQLite build to make the repository's qualification environment deterministic and independent of a host distribution's SQLite package version.

The v0.4 durability claim is scoped to qualified local filesystems/storage that honor SQLite's required locking and sync semantics. Network filesystems, storage that lies about durability, host/hypervisor snapshot rollback and other coherent rollback of the complete database are outside the v0.4 authority guarantee unless independently qualified later.

### 3. Prepared authority is exact-bound, fresh-observation-bound and non-executable

A durable prepare generation is identified by a typed transaction ID plus a monotonically increasing generation number and binds at minimum:

- authenticated principal;
- request ID;
- canonical plan ID;
- a digest of the complete validated authoritative observation envelope, not merely its evidence ID;
- provider/resource/capability identity;
- policy ID and revision;
- trusted resulting risk and risk-policy provenance;
- material reviewed changes/findings and semantic provenance through a deterministic content digest;
- the exact authorization basis: policy allow or valid approval evidence, including approval identity/approver/validity material when approval is required.

The full observation binding covers all authority/freshness-relevant envelope material, including provider/resource/capability, authority class/source, observation timestamp, sequence/generation where present, validity/freshness window and canonical observed attributes. v0.4 must extend the canonical planning/review lineage as needed so Control retains this material or its deterministic digest rather than trying to reconstruct it from `evidence_id`.

Immediately before creating or reusing a durable prepare generation, Control must use Control-owned time to verify that the bound authoritative observation remains valid/fresh and that the current trusted review/approval still matches the exact canonical material. Expired/stale observation requires fresh authoritative observation followed by replan/review; an evidence identifier alone cannot preserve prepare eligibility.

A `PlanId`, request ID, approval ID, evidence ID, transaction ID or generation number alone is never sufficient authority evidence.

The canonical binding digest uses deterministic domain-separated, length-delimited encoding before SHA-256 hashing. Ambiguous string concatenation, map iteration order and debug formatting are not authority encodings.

A `Prepared` generation does **not** contain an executor handle, Polkit grant or capability to perform an external effect. It proves only that an exact fresh reviewed authority subject crossed the durable `prepare` boundary for that generation.

### 4. Idempotency survives restart without rebinding prior generations

The durable idempotency namespace is the authenticated principal plus request ID. One stable transaction identity owns an append-only sequence of immutable generation records. Each generation has its own immutable authority binding and binding digest; creating a later generation never rewrites, aliases or rebinds an earlier generation.

Ordinary request reuse rules are fail-closed:

- same principal + request ID + exact binding digest for the current `Prepared` generation returns the existing transaction/generation;
- same principal + request ID + different binding material through the normal prepare/request path is an idempotency conflict;
- a different principal is a different authority namespace;
- terminal or recovery state does not permit the normal request path to silently rebind the request identity to different authority material.

A new generation with changed authority material is allowed only through the explicit Control-owned recovery transition defined in section 6. That path must atomically prove all of the following against an expected transaction ID, previous generation number and previous state:

- the previous generation is still `Indeterminate`;
- fresh authoritative recovery observation proves the intended effect absent;
- current authoritative observation used for the next plan is fresh and fully validated;
- the canonical plan/review has been recomputed as required from current evidence;
- current policy is re-evaluated;
- any required approval is newly/currently valid;
- the next generation number is exactly the previous generation plus one;
- the previous generation remains immutable and linked to the new generation by an append-only recovery/audit record.

The recovery operation is therefore not an idempotency bypass: a caller cannot present a changed binding to ordinary prepare and obtain a new generation. Only the typed recovery path, guarded by fresh no-effect evidence and current authority re-establishment, can append generation N+1 under the existing stable transaction/request namespace.

SQLite uniqueness and foreign-key constraints backstop these semantics so process restart cannot reopen a request ID that an in-memory map forgot, duplicate a generation, or rewrite a prior generation binding.

These guarantees assume monotonic continuity of the qualified authority database. A coherent replacement with an older internally valid database copy can erase later idempotency/audit facts and is not detectable by an internal unkeyed hash chain alone. v0.4 therefore does not support authority-database rollback/restore or VM-snapshot rollback as a transparent operation. A future restore protocol must add an independently protected monotonic epoch/anchor or force authority invalidation and fresh re-establishment before such restore can be supported.

### 5. Transaction states model ambiguity explicitly and the pre-dispatch capability is one-shot

The v0.4 transaction domain includes bounded, validated transitions for each immutable generation:

- `Prepared` — exact durable intent-to-execute authority record exists;
- `Indeterminate` — the current generation has crossed the durable pre-dispatch handoff and a future effect may be attempted, so outcome must be treated as unknown until authoritatively observed;
- `Verified` — fresh authoritative re-observation proves the intended postcondition for this exact generation;
- `Committed` — verified durable commit metadata for desired-state/graph/provenance references is atomically recorded;
- `Aborted` — the generation ended before the durable pre-dispatch ambiguity boundary, or recovery reached an explicitly safe terminal path;
- `RecoveryBlocked` — authoritative recovery evidence conflicts with safe automatic continuation.

A future executor **must not be callable while the current generation remains `Prepared`**. Before any v0.5/v0.6 executor invocation or effect release, Control must perform an atomic compare-and-swap transition for the exact expected transaction ID + generation + binding digest from `Prepared` → `Indeterminate`, together with its audit event. Exactly one caller may win that transition.

Only the caller that successfully commits that exact transition may receive the corresponding process-local dispatch permit. The permit contract is deliberately capability-like and narrow:

- it is bound to the exact transaction ID, generation and binding digest that won the transition;
- its constructor is sealed to the trusted Control/transaction handoff path;
- it is not `Clone`, serializable, persistable or reconstructible from an `Indeterminate` database row or transaction ID;
- it is consumed by value by the future executor-dispatch API, making one successful permit usable at most once;
- a second caller that observes the already-`Indeterminate` generation receives no permit;
- a crash after durable `Indeterminate` commit but before permit consumption loses the permit permanently for that generation; restart cannot recreate it and must enter authoritative recovery instead.

The durable database stores the fact that the ambiguity boundary was crossed, not a reusable dispatch credential. Therefore two callers cannot race from the same durable `Indeterminate` row and independently reconstruct permission to dispatch the same non-idempotent effect.

v0.4 exposes and qualifies this pre-dispatch handoff and one-shot permit semantics but does not call an executor. This makes the crash/concurrency boundary part of the durable contract before privileged code is introduced.

`Indeterminate` is sticky across restart. Fresh authoritative observation proving the effect absent is necessary but **not sufficient by itself** to create a new prepared generation: Control must also revalidate/re-establish current policy, observation freshness and any required approval before atomically appending the next immutable `Prepared` generation. An expired/revoked approval cannot be revived by recovery evidence.

### 6. Recovery never trusts local persistence as machine truth

An indeterminate generation requires a fresh authoritative recovery observation. Recovery classifies the observation as one of:

- intended state verified → `Verified` for that exact generation;
- intended effect proven absent → eligible for current-authority revalidation and an explicit recovery append of generation N+1, but no retry or dispatch authority exists yet;
- conflicting state → `RecoveryBlocked`;
- insufficient/stale/ambiguous evidence → remain `Indeterminate`.

The no-effect recovery append is an explicit compare-and-swap operation over the stable transaction identity and expected previous generation. It never mutates the previous generation binding. If concurrent recovery callers race, at most one may append the next generation; losers must reload the durable transaction state and cannot create a sibling generation from the same predecessor.

A restart alone, retry request, daemon PID change, SQLite row state, lost dispatch permit or executor self-report must never authorize redispatch.

Observed Linux state continues to come from the authoritative observation/provider path. The transaction store retains full evidence binding/digests and recovery decisions but does not fabricate current state.

### 7. Commit requires prior verification

`Committed` is reachable only from `Verified`. The commit operation atomically records the verified transaction state plus deterministic references/digests for the desired-state, graph and semantic-provenance material that later lifecycle integration will publish.

v0.4 does not claim the full persistent intent/Library model planned for v0.7. It establishes the transactionally verified commit boundary required for later managed effects.

### 8. Audit is append-only and integrity-checked within one monotonic database history

Every durable transaction transition, generation append and recovery decision emits an append-only audit event in the same SQLite transaction as the state change. Events include monotonic per-transaction sequence/generation information and an integrity digest chained to the previous event.

The SQLite schema prevents ordinary UPDATE/DELETE of audit rows. Store-open/integrity validation checks at minimum:

- SQLite integrity/foreign-key status;
- supported application/schema version;
- migration checksum identity;
- transaction/generation binding digest format;
- generation continuity and immutability;
- audit event sequence and hash-chain continuity;
- consistency between a transaction's current generation/state and its terminal retained audit event.

These checks detect malformed state and non-coherent mutation/deletion/reordering inside the retained history, but an unkeyed internal chain is **not** an external rollback anchor and does not claim to detect replacement by an older complete, internally consistent database. Detected corruption, unsupported newer schema, migration mismatch or broken retained audit continuity fails closed. Automatic repair must not silently rewrite authority history.

### 9. Authority-state writes are bounded

Persisted authority material has explicit per-record and aggregate bounds. Oversized review/audit payloads and capacity exhaustion are rejected before unbounded cloning/serialization where practical. Authority records and audit history are never silently evicted to create room for newer authority.

Operational maintenance such as checkpointing is allowed; semantic history deletion requires a future explicit retention/archival policy and is not introduced in v0.4.

### 10. Durability qualification covers the actual local crash boundary

The release may claim process/crash recovery only for the explicitly qualified local filesystem/storage profile. Qualification must exercise transaction-boundary faults, not just graceful reopen:

- process `SIGKILL` before and after durable transition commit points;
- abrupt disposable-VM power interruption after a transition has acknowledged durable commit, followed by filesystem/database reopen;
- crash during WAL/checkpoint-sensitive activity where the harness can deterministically inject it;
- representative local I/O/write failure paths that prove failed SQLite commits do not advance transaction/audit state;
- integrity validation after each recovered case.

The qualification report must name filesystem/storage/hypervisor assumptions and distinguish process restart, abrupt guest power loss and unsupported coherent host/snapshot rollback. `synchronous=FULL` is a database configuration, not a universal hardware durability proof.

## Security assessment

This ADR is a security-boundary change. Required negative proofs include:

- request-ID reuse with changed binding through the ordinary prepare path is rejected across process/database reopen;
- recovery can append generation N+1 only from the expected `Indeterminate` generation after fresh no-effect observation plus complete current authority re-establishment;
- prior generation bindings remain immutable and concurrent recovery cannot create sibling next generations;
- plan/full-observation/policy/risk/approval substitution changes the exact generation binding and cannot reuse prepare authority;
- stale/expired authoritative observation cannot create or reuse prepare authority;
- expired/revoked/mismatched required approval cannot create or re-prepare a transaction;
- deny/blocked/no-change review states cannot become prepared mutation authority;
- malformed/unsupported/corrupted database state fails closed;
- ordinary audit UPDATE/DELETE/tampering is rejected or detected within the retained database history;
- coherent whole-database rollback is documented as unsupported rather than claimed as internally detectable;
- a crash/reopen cannot turn `Indeterminate` into retry authority or recreate a lost dispatch permit;
- a future executor cannot be called until the exact current generation is durably `Indeterminate`;
- exactly one successful `Prepared` → `Indeterminate` compare-and-swap can mint the non-cloneable/non-reconstructible permit for a generation, and the future dispatch API consumes it at most once;
- fresh authoritative evidence proving no effect permits only recovery reauthorization/re-prepare eligibility, not automatic redispatch;
- commit before verification is rejected;
- process-kill, abrupt guest-power and write-failure tests demonstrate the qualified SQLite/WAL boundary;
- no API introduced by v0.4 reaches an executor, Polkit or external Linux mutation.

## Consequences

- v0.4 makes the review-to-prepare and pre-dispatch crash/concurrency boundaries explicit and durable without prematurely adding execution.
- SQLite becomes an implementation detail behind a transaction repository contract rather than a dependency of policy/planning semantics.
- The canonical plan/review lineage gains complete authoritative-observation binding/freshness material sufficient for safe prepare-time revalidation.
- Stable request idempotency and immutable per-generation bindings coexist: ordinary changed-binding reuse stays rejected while authoritative no-effect recovery can append a new generation without rewriting history.
- v0.5 can qualify a narrow executor only after the generation is durably marked indeterminate and only with the one-shot permit returned to the winning handoff caller, eliminating both the unsafe `Prepared`-while-dispatched window and duplicate-dispatch races by construction.
- v0.6 can integrate the executor/verifier with durable prepare, pre-dispatch handoff, verification, commit, audit and reconciliation through the canonical lifecycle.
- Recovery correctness intentionally depends on authoritative re-observation plus current authorization, not local database confidence.
- Internal hash chaining strengthens retained-history integrity but is not misrepresented as an external anti-rollback mechanism.
- The local authority path remains standalone; hosted/fleet databases or services are not prerequisites.
