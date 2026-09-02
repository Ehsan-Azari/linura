# Action lifecycle

A managed mutation is downstream of approved intent or an explicit deterministic request. Persistent managed state always carries semantic provenance and passes through one canonical authority lifecycle.

## Canonical successful path

```text
request / intent
      ↓
observe
      ↓
plan
      ↓
validate
      ↓
authorize
      ↓
prepare
      ↓
execute
      ↓
verify
      ↓
commit
      ↓
audit
      ↓
reconcile
```

The order is normative. Concrete implementations may stop early on unsupported capability, invalid input, denial, failed approval, execution failure, failed verification or recovery conditions, but a successful managed mutation must not reorder or skip stages.

## Stage contracts

1. **Request / intent** — establish authenticated principal/actor provenance, request ID, resource/capability target and semantic origin.
2. **Observe** — read authoritative current state and prerequisites from the responsible Linux/provider boundary. Observation is an explicit input to planning.
3. **Plan** — derive a deterministic `ReconciliationPlan` from normalized desired state plus authoritative observed state. The plan is immutable review evidence; replanning against different material input creates a different review subject even if a pre-1.0 identifier happens to be reused.
4. **Validate** — validate identifiers, semantic origin, evidence identity/freshness, planned changes/findings, capability assumptions and structural invariants before authority is granted.
5. **Authorize** — evaluate policy over the exact validated plan and authenticated principal and, when required, resolve explicit user/admin/destructive approval. Authorization evidence must satisfy the exact plan/evidence/policy/principal binding.
6. **Prepare** — durably record the exact reviewed authority binding, intent-to-execute, idempotency/correlation data and recovery metadata before external effects are dispatched.
7. **Execute** — materialize/dispatch only the narrow typed effects permitted by the prepared and authorized plan through the responsible executor boundary.
8. **Verify** — re-observe authoritative state after execution and evaluate expected postconditions through a verifier boundary independent from executor success reporting.
9. **Commit** — finalize Linura's desired-state, graph and semantic-provenance transaction only after successful verification.
10. **Audit** — append correlated evidence linking request, principal/actor, plan, policy/approval, prepared authority, effects, observations, verification and commit outcome. Failure and compensation evidence is append-only as well.
11. **Reconcile** — compare durable desired state with authoritative observed state, surface drift and schedule/perform policy-controlled corrective work when appropriate.

## Plan and effect boundary

Linura deliberately separates a **reviewable reconciliation plan** from later executable effect materialization.

The current canonical `ReconciliationPlan` includes:
- plan/request/actor/resource/provider/capability identity;
- intent/requirement/capability origin (`SemanticReason`);
- exact authoritative evidence identity;
- prospective risk;
- deterministic current→desired changes;
- findings/blockers;
- an execution-authority invariant that remains disabled through v0.3.

A reviewed/approved plan is still not a prepared mutation. Later milestones add the durable binding and narrowly typed effect/executor contract only after the plan has passed policy/approval review. This prevents provider-owned executable plans or approval artifacts from becoming a parallel execution path.

## Transaction semantics

**The lifecycle does not require distributed two-phase commit across Linux subsystems.** Many operating-system effects are not atomically reversible and many upstream services do not expose prepare/commit primitives.

Linura's transactional guarantees therefore come from the canonical lifecycle itself: immutable review subjects, exact authorization binding, durable prepare records, idempotency/deduplication where available, explicit indeterminate states, checkpoints, independent re-observation, postcondition verification, compensation/rollback metadata where a safe inverse exists, and reconciliation when exact rollback is impossible.

A provider or executor must not pretend a non-transactional upstream mechanism is atomic. Cross-provider plans must make ordering, preconditions, failure boundaries and compensation/recovery semantics explicit rather than relying on an implicit distributed transaction.

## Idempotency and crash recovery

Every mutation has a request ID and immutable reviewed-plan identity/binding. `prepare` persists the fact that an authorized plan is about to cross the external-effect boundary. Providers/executors define whether retry is naturally idempotent, deduplicated or rejected after ambiguous execution.

If a crash occurs after external effects but before durable finalization, recovery marks the operation indeterminate and re-observes authoritative state before making another effect. It must not blindly replay the previous command.

## Verification

Executor success is execution evidence, not state proof. Verification re-observes the authoritative subsystem and evaluates expected postconditions. The provider/executor that issued a mutation is not automatically trusted as the sole verifier of the resulting state.

## Commit, audit and provenance

`commit` finalizes Linura's durable desired-state/graph/provenance representation after verification. `audit` records who requested, authorized and executed what and with which evidence. Semantic provenance remains distinct: it records *why* the managed state exists.

## Implementation maturity

`linura-lifecycle` preserves the ordered eleven-stage state machine as a future-proof architectural invariant. The earlier bootstrap `MutationRuntime`/generic executable `ActionPlan` scaffold was removed once v0.2 established the canonical reconciliation-plan path; preserving a competing execution model would be more dangerous than useful.

This removal does not collapse future stages. v0.3 implements review-only authority and stops before `prepare`; v0.4 establishes durable transaction/recovery binding; v0.5 qualifies narrow executor/verifier contracts; v0.6 is the first milestone allowed to integrate the complete lifecycle and publish a bounded supported Experimental managed effect.
