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

1. **Request / intent** — establish authenticated actor, request ID, resource, capability, operation, parameters and semantic origin.
2. **Observe** — read authoritative current state and prerequisites from the responsible Linux/provider boundary. Observation is an explicit input to planning.
3. **Plan** — derive a deterministic `ActionPlan` from the request plus observed state. Plans are immutable evidence; replanning creates a new plan identity.
4. **Validate** — validate identifiers, semantic origin, preconditions, effects, verification requirements, capability assumptions and structural invariants before authority is granted.
5. **Authorize** — evaluate policy and, when required, resolve explicit user/admin/destructive approval. Authorization evidence must satisfy the exact policy decision.
6. **Prepare** — durably record intent-to-execute, idempotency/correlation data and recovery metadata before external effects are dispatched.
7. **Execute** — dispatch only the narrow typed effects in the authorized plan through the responsible executor boundary.
8. **Verify** — re-observe authoritative state after execution and evaluate expected postconditions through a verifier boundary independent from executor success reporting.
9. **Commit** — finalize Linura's desired-state, graph and semantic-provenance transaction only after successful verification.
10. **Audit** — append correlated evidence linking request, actor, plan, policy/approval, effects, observations, verification and commit outcome. Failure and compensation evidence is append-only as well.
11. **Reconcile** — compare durable desired state with authoritative observed state, surface drift and schedule/perform policy-controlled corrective work when appropriate.

## Action plan

A plan includes:
- plan + request + actor + resource identifiers;
- intent/requirement/capability origin (`SemanticReason`);
- authoritative pre-execution observation context;
- preconditions and freshness bounds;
- ordered effects;
- privilege/risk classification;
- reversibility/compensation metadata;
- expected postconditions and independent verification strategy.

## Transaction semantics

**The lifecycle does not require distributed two-phase commit across Linux subsystems.** Many operating-system effects are not atomically reversible and many upstream services do not expose prepare/commit primitives.

Linura's transactional guarantees therefore come from the canonical lifecycle itself: immutable plans, exact authorization binding, durable prepare records, idempotency/deduplication where available, explicit indeterminate states, checkpoints, independent re-observation, postcondition verification, compensation/rollback metadata where a safe inverse exists, and reconciliation when exact rollback is impossible.

A provider or executor must not pretend a non-transactional upstream mechanism is atomic. Cross-provider plans must make ordering, preconditions, failure boundaries and compensation/recovery semantics explicit rather than relying on an implicit distributed transaction.

## Idempotency and crash recovery

Every mutation has a request ID and immutable plan ID. `prepare` persists the fact that an authorized plan is about to cross the external-effect boundary. Providers/executors define whether retry is naturally idempotent, deduplicated or rejected after ambiguous execution.

If a crash occurs after external effects but before durable finalization, recovery marks the operation indeterminate and re-observes authoritative state before making another effect. It must not blindly replay the previous command.

## Verification

Executor success is execution evidence, not state proof. Verification re-observes the authoritative subsystem and evaluates expected postconditions. The provider/executor that issued a mutation is not automatically trusted as the sole verifier of the resulting state.

## Commit, audit and provenance

`commit` finalizes Linura's durable desired-state/graph/provenance representation after verification. `audit` records who requested, authorized and executed what and with which evidence. Semantic provenance remains distinct: it records *why* the managed state exists.

## `0.0.0` implementation contract

`linura-lifecycle` defines the ordered eleven-stage mutation state machine. `linura-control` owns orchestration order and exposes a `MutationRuntime` port for concrete authorization, persistence, execution, verification, audit and reconciliation implementations. `linura-provider-sdk` makes observation an explicit planning input and separates `EffectExecutor` from `EffectVerifier`.

These are foundation contracts, not a claim that every production backend already exists. The first complete vertical slice should make one narrow capability traverse all eleven stages with real persistence, approval/Polkit enforcement, executor isolation, independent verification, append-only audit and reconciliation before breadth is added.
