# Provider model

Providers adapt Linux subsystems to Linura domain contracts. They supply authoritative observation and deterministic planning inputs; they do not own Linura's global authority sequence.

A provider declares:
- stable provider ID/version;
- supported capabilities;
- resources it can authoritatively observe;
- deterministic actions it can plan from request + observation;
- whether an effect can execute unprivileged or needs a narrow privileged executor;
- expected postconditions and verifier strategy;
- diagnostic metadata.

## Required separation

The provider SDK intentionally separates three concerns:

1. `Provider::observe` returns an authoritative observation for the requested resource.
2. `Provider::plan` consumes both the typed request and that observation to produce an immutable `ActionPlan`.
3. `EffectExecutor` and `EffectVerifier` are separate contracts. Executor success is not accepted as proof that the intended state exists; verification consumes post-execution authoritative observation.

A single implementation package may supply more than one of these roles during early development, but the interfaces remain separate so production deployments can isolate privilege and verification independently.

Providers/executors/verifiers cannot skip `authorize`, `prepare`, `commit`, `audit` or `reconcile`; those are owned by Linura Control's canonical mutation lifecycle.

## Bounded probes and orchestration

**Providers expose bounded mechanisms; Linura owns orchestration.**

A provider/observer may perform a narrow probe against its authoritative upstream subsystem. Cross-provider fan-out, global retry policy, query deadlines, cancellation, concurrency limits, admission control, query coalescing, cache policy, backpressure, partial-result policy and aggregate resource budgets belong to Linura's control-plane query orchestration rather than to individual providers.

The current `Observer` interface is intentionally small and synchronous. ADR 0017 defines the boundary for a future context-query runtime without requiring a speculative runtime today. When that runtime is introduced, it may wrap or evolve observer calls to carry explicit budgets/cancellation while preserving the existing authority and freshness semantics.

The probe boundary should remain narrow and stable: semantic request/result types cross it, while D-Bus objects, file descriptors, sockets, process/session handles and other implementation-specific handles remain provider-owned and lifetime-bounded. They are not portable Linura resource identities.

Provider implementations must not turn a bounded request into unbounded polling or silently widen a caller's deadline, resource, freshness or partial-result contract. If the runtime cannot satisfy a requested service bound, it should reject or return an explicit degraded/partial outcome rather than allow a provider to exceed the bound invisibly.

## Transport boundary

D-Bus has two legitimate adapter roles in the current architecture:

1. the local Linura client/control transport (`org.linura.Control1`);
2. a provider/observer's native upstream transport when Linux subsystems such as systemd, NetworkManager or BlueZ expose authoritative APIs over D-Bus.

Neither role defines Linura domain semantics. D-Bus object paths, interfaces, signals, Unix file descriptors and wire values must not leak into transport-neutral capability, desired-state, observation, planning, policy or lifecycle contracts.

The same rule applies to Unix sockets, subprocesses, filesystem/kernel interfaces, native libraries and HTTP/RPC: they are provider/transport mechanisms, not Linura's semantic model.

## Observation and context

`linura-observation::ObservationEnvelope` is the canonical authoritative observation envelope. It preserves provider/resource/capability identity, authority, timestamp/validity, sequence and typed attributes.

The compact `linura_provider_sdk::Observation` used by the current planning contract is transitional architecture debt. It must converge on `ObservationEnvelope` end-to-end rather than becoming a second canonical state representation.

Caching and aggregation may retain observations for bounded reuse, history, coalescing and context projection. Cached evidence keeps its provenance and freshness semantics. Cache presence cannot satisfy a consumer that requires a fresh authoritative observation unless the cached envelope still satisfies that exact authority/freshness contract.

Probabilistic confidence from a model/retriever/aggregator is context metadata, not an authority upgrade. Provider-native quality/uncertainty may be represented as typed evidence where a domain requires it, but a required authoritative fact still has to satisfy the provider/resource/capability/freshness contract.

Context projections and future RAG/retrieval may combine observations with historical evidence, documentation or indexed material for reasoning. Retrieval output cannot manufacture authoritative observed state, authorize an effect or replace required post-effect verification.

Expected first providers:

| Domain | Provider |
|---|---|
| Network | NetworkManager over D-Bus |
| Bluetooth | BlueZ over D-Bus |
| Audio/media | PipeWire/WirePlumber |
| Services | systemd D-Bus |
| Storage | UDisks2 + filesystem-specific helpers |
| Authorization | Polkit |
| Snapshots | Snapper |
| Firewall | nftables/firewalld profile, selected by platform profile |
| Packages | pacman on Arch profile |

Providers must not leak raw command output into public API types. Provider-specific diagnostics may be attached in explicitly namespaced diagnostic fields. Observation and verification evidence should be structured enough to support freshness, correlation and audit without making provider-specific text the source of truth.
