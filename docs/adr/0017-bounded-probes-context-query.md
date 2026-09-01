# ADR 0017: Bounded probes and control-plane-owned context queries

Status: Accepted

## Context

Linura must combine declarative desired state with dynamic, authoritative knowledge of a live machine. As provider breadth grows, a single semantic question may require observations from systemd, networking, hardware, storage, containers, security, remote inventory, or other subsystems with very different native APIs and cost profiles.

D-Bus, Unix file-descriptor passing, sockets, command-line tools, filesystem/kernel interfaces, HTTP/RPC, and future mechanisms are implementation details. Letting any one transport shape Linura's semantic model would couple capability, planning, policy, and agent context to an accidental Linux integration mechanism.

Likewise, letting individual providers independently own retries, unbounded polling, fan-out, cache lifetime, concurrency, or cross-provider aggregation would make resource consumption and service guarantees impossible to reason about at the control-plane level.

Linura already separates authoritative observation, planning, execution, verification, and the canonical managed-mutation lifecycle. This decision refines the observation/context side without creating another authority path or lifecycle.

## Decision

Linura adopts the following invariant:

> **Providers expose bounded mechanisms; Linura owns orchestration. Transports do not define semantics, caches do not define truth, retrieval does not define authority, and models do not define machine state.**

A **probe** is one bounded attempt to obtain provider-backed evidence for a resource/capability. A future **context query** may require one or many probes. Query planning, scheduling, budgets, deadlines, cancellation, concurrency, retry/coalescing policy, cache eligibility, aggregation, backpressure, partial-result behavior, and result-size limits are control-plane concerns.

The current `Observer`/`ObservationCoordinator` path remains the authoritative foundation. This ADR does not require an immediate generic query runtime. It defines the boundary that a later implementation must preserve.

### Provider and transport boundary

Providers adapt semantic Linura contracts to upstream mechanisms. They may internally use D-Bus, Unix sockets, native libraries/APIs, kernel/filesystem interfaces, subprocesses, HTTP/RPC, or other bounded mechanisms.

Provider/probe interfaces should stay deliberately small and stable in the same spirit as a narrow driver API: callers exchange semantic typed requests/results, while native handles and implementation mechanics remain private to the adapter. D-Bus object paths, Unix file descriptors, sockets, process handles and other transport/session handles are provider-owned, lifetime-bounded implementation details; they are not durable Linura resource identities and must not leak into portable domain contracts.

Two D-Bus roles are explicitly permitted:

1. D-Bus as a local Linura client/control transport.
2. D-Bus inside a provider/observer when an upstream Linux subsystem exposes its authoritative API through D-Bus.

Neither role may define Linura capability, resource, desired-state, observation, planning, policy, or lifecycle semantics. D-Bus paths, interfaces, signals, file descriptors, and wire-specific values remain adapter details.

### Observation, cache, uncertainty, and context boundary

`linura-observation::ObservationEnvelope` is the canonical authoritative observation envelope. It carries provider/resource/capability identity, authority, time/validity information, sequence, and typed attributes.

Authoritative state does not become more authoritative because a model, retriever or aggregator assigns it a confidence score. If an upstream subsystem natively exposes quality/uncertainty metadata, a provider may preserve that as typed evidence where the domain requires it; inferred confidence belongs to derived/retrieval context and cannot substitute for a required authoritative observation.

A cache may retain or materialize observations for bounded reuse, history, query coalescing, or context projection. Cached evidence keeps its original provenance and freshness semantics. When a consumer requires current authoritative truth, cache presence cannot waive the required freshness/authority contract.

Derived **context projections** may combine authoritative observations, system-graph state, historical evidence, diagnostics, documentation, or retrieval results for planners, UIs, agents, and future RAG. A context projection is not itself authoritative observed state.

RAG/retrieval may augment reasoning. It may never manufacture an `ObservationEnvelope`, satisfy a required authoritative observation by assertion, or grant policy/execution authority.

### Query governance and bounded service contracts

A future query runtime must make resource governance explicit. Depending on the query contract, this includes:

- deadline and timeout;
- cancellation;
- bounded concurrency and fan-out;
- priority and resource budget;
- admission control when a requested service bound cannot be honored;
- propagation of remaining deadline/resource budget into dispatched probes;
- retry and query-coalescing policy;
- cache/freshness requirements;
- backpressure and response-size bounds;
- deterministic aggregation where required;
- partial-result and provider-unavailability semantics;
- provenance for every returned fact;
- fail-closed behavior when an authoritative fact required for planning/policy is unavailable.

A query service contract should make latency/deadline, freshness, resource and partial-result expectations explicit enough that the runtime can reject, degrade, or return a typed partial result rather than silently violating them. Providers do not silently widen these budgets or turn a bounded query into an unbounded background activity.

### Relationship to the mutation lifecycle

Context queries and probes are implementation machinery inside observation/planning/intelligence concerns. They do **not** add, reorder, or bypass stages in the canonical lifecycle:

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

The authority plane remains owned by Linura Control. A query result cannot authorize an effect, and an agent or retrieval system cannot receive privileged executor authority. Query orchestration is also not a replacement transaction coordinator; mutation transaction/recovery semantics remain those defined by the canonical lifecycle.

### Terminology

`Actor` remains reserved for an authenticated human/service/agent principal. Backend observation workers are not called actors. Linura uses `Probe`, `Observer`, `Provider`, `Executor`, and `Verifier` for the relevant system roles.

## Consequences

- The existing opinionated `arch-hyprland-v1` profile can continue using Linux-native provider mechanisms without making them core dependencies.
- The planner and semantic crates remain transport/provider implementation neutral.
- The current synchronous observer API may be wrapped or evolved later to carry explicit budgets/cancellation, but no speculative distributed runtime is introduced by this ADR.
- Multi-provider batching, coalescing, persistent materialized views, semantic/vector retrieval, and fleet/cluster query federation remain future work until a bounded milestone requires and qualifies them.
- The transitional compact planning `linura_provider_sdk::Observation` must not become a second canonical observation model; planning should converge on `ObservationEnvelope` end-to-end.
- Repository validation should machine-enforce the most important dependency and terminology boundaries so transport coupling cannot silently creep inward.
