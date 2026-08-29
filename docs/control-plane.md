# Linura Control and the system control plane

`linura-control` implements Linura Control, the local authority subsystem. The **system control plane** is the architectural role it fulfills and is not a separate product brand.

The control plane is the canonical mediator between intent and operating-system effects. It owns the ordering of the trustworthy mutation lifecycle; concrete providers, approval systems, persistence engines, executors, verifiers, audit sinks and reconcilers implement stage behavior behind typed boundaries.

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
- create deterministic action plans from request + observed state where possible;
- validate plans and preconditions before authority is granted;
- evaluate policy and resolve required approval evidence;
- durably prepare intent-to-execute before external effects;
- execute allowed narrow effects through providers/executors;
- re-observe and independently verify resulting state;
- commit desired-state/graph/provenance only after verification;
- append correlated audit evidence;
- reconcile persistent desired state when enabled.

## Non-responsibilities

The control plane does not:
- replace NetworkManager/BlueZ/PipeWire/systemd;
- parse arbitrary natural language itself;
- accept arbitrary shell scripts as system actions;
- provide a generic root RPC endpoint;
- trust executor success as proof of resulting machine state;
- allow providers to reorder the authority lifecycle;
- make UI-specific layout decisions.

## `0.0.0` boundary

The full lifecycle is a code-level orchestration contract in `0.0.0`, not a claim that every backend is production implemented. `MutationRuntime` deliberately leaves approval, durable prepare/commit, execution, verification, audit and reconciliation behind injectable ports while `linura-control` owns stage order and correlation checks.

The first real vertical slice should implement all eleven stages for one narrow capability before the control plane expands horizontally across Linux domains.

## Gateway transport

The first local transport should be D-Bus because Linux system/session integration and Polkit identity naturally fit it. The domain protocol remains transport-neutral. A remote gRPC/mTLS gateway may be added later as a separate process rather than exposing the session daemon directly to the network.
