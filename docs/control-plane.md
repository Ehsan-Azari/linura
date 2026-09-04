# Linura Control and the system control plane

`linura-control` implements Linura Control, the local authority subsystem. The **system control plane** is the architectural role it fulfills and is not a separate product brand.

The control plane is the canonical mediator between intent and operating-system effects. It owns the ordering of the trustworthy mutation lifecycle; concrete observation adapters, policy engines, approval systems, persistence engines, executors, verifiers, audit sinks and reconcilers implement bounded stage behavior behind typed boundaries as their milestones mature.

## Canonical managed-mutation lifecycle

```text
request / intent
→ observe
→ plan
→ validate
→ authorize
→ prepare
→ execute
→ verify
→ commit
→ audit
→ reconcile
```

A successful managed mutation must not skip or reorder these stages. See [Action lifecycle](action-lifecycle.md) and [ADR 0012](adr/0012-canonical-mutation-lifecycle.md).

## Responsibilities

- expose versioned local API;
- authenticate/identify callers and retain request identity;
- discover platform/provider capabilities;
- observe authoritative current state before planning;
- accept desired changes and semantic origin;
- derive deterministic reconciliation plans through the canonical planner;
- validate plans and evidence before authority is granted;
- evaluate policy and resolve required approval evidence over the exact reviewed subject;
- later, durably prepare intent-to-execute before external effects;
- later, execute allowed narrow effects through executors;
- later, re-observe and independently verify resulting state;
- later, commit desired-state/graph/provenance only after verification;
- later, append correlated audit evidence and reconcile persistent desired state.

## Current authority maturity

The released v0.2 control path is `PlanPreviewControl`: authenticated principal → authoritative observation → deterministic `ReconciliationPlan` → non-executable `PlanPreview` with bounded process-local retention.

v0.3 extends that exact lineage with deterministic policy and approval review. It does not introduce a second provider-owned plan type and it stops before durable `prepare` or any executor authority. An `allow` decision or valid approval is review evidence, not permission to invoke a privileged effector.

The early bootstrap `ControlPlane::apply`/`MutationRuntime` scaffold was removed instead of retained as a legacy compatibility path because it was unused by live applications and depended on the superseded provider-generated `ActionPlan` model. The canonical eleven-stage state machine remains in `linura-lifecycle`, and narrow executor package scaffolds remain for their later milestones.

## Non-responsibilities

The control plane does not:
- replace NetworkManager/BlueZ/PipeWire/systemd;
- parse arbitrary natural language itself;
- accept arbitrary shell scripts as system actions;
- provide a generic root RPC endpoint;
- trust executor success as proof of resulting machine state;
- allow providers to create a parallel planning/policy/authority path;
- let approval itself become an executor credential;
- make UI-specific layout decisions.

## Future stage integration

v0.4 adds durable review/prepare/recovery binding. v0.5 qualifies a first narrow privileged executor and independent verifier without product mutation support. v0.6 is the first milestone allowed to integrate all eleven stages for a supported bounded Experimental external effect.

That sequencing preserves future scaffolding without precommitting current authority APIs to an obsolete generic execution model.

## Gateway transport

The first local transport uses D-Bus because Linux system/session integration and caller identity naturally fit it. The domain protocol remains transport-neutral. A remote gRPC/mTLS gateway may be added later as a separate process rather than exposing the session daemon directly to the network.

## v0.5 executor isolation

The v0.5 systemd executor is deliberately **not** wired into Linura Control. Control retains the v0.4 process-local, non-serializable one-shot dispatch-permit boundary; v0.5 only qualifies the isolated executor/verifier components on disposable fixtures. No durable row, digest, Polkit decision, executor receipt, or public Control1 call substitutes for the future authenticated one-shot handoff. That integration belongs to v0.6 and must preserve the canonical eleven-stage lifecycle.
