# Control1 non-executable plan preview

This document defines the implementation boundary for the `v0.2.0` Control1/CLI integration. It is a development contract, not a supported-release claim.

## Purpose

Expose Linura's deterministic desired-state planner to authenticated local clients without creating a mutation API.

The public flow is:

```text
authenticated transport caller
→ transport credential verification + wire decoding
→ typed desired-state request + transport-neutral authenticated principal
→ linura-control retained-request replay check
→ fresh authoritative observation for first-seen requests
→ planner observation projection
→ deterministic reconciliation preview
→ bounded linura-control process-local retention
→ transport wire encoding / lookup / explanation
```

`linura-control::PlanPreviewControl` owns the replay → observation → planning → retention sequence. A transport such as `linura-dbus` authenticates the caller, derives the stable principal and concrete actor provenance, decodes/encodes its wire representation, and delegates. No transport is allowed to reimplement those authority semantics.

The flow ends at a preview. It does not enter authorization, prepare, execution, verification, commit, audit, or reconciliation.

## Authority boundary

The transport's authenticated credentials are the actor source of truth. For the D-Bus implementation, Control1 resolves the sender through D-Bus credentials before accepting a planning request; a caller cannot supply or override its `Actor` identity. The first accepted request retains that transport-derived actor in the preview for provenance.

Replay and retained-preview ownership use a separate transport-neutral `AuthenticatedPrincipal`. D-Bus derives that principal from the authenticated Unix UID, so the same local principal can safely retry after reconnecting with a different D-Bus unique name. Future transports must derive an equivalently stable authenticated principal from their own credential boundary and then call the same `linura-control` API rather than defining independent replay or retention rules.

Desired-state values, semantic summaries, and semantic-origin IDs are untrusted request data. They must pass the same typed identifier, desired-state, semantic-provenance, observation-identity, and freshness validation used by the deterministic planner. Resource-bound validation that determines whether observation/planning may proceed is owned by `linura-control`; transports may additionally reject malformed or lossy wire representations before typed conversion.

A preview may classify its prospective risk as `system-mutation`, but that classification is descriptive only. `ReconciliationPlan::execution_authorized()` remains false by construction. Control1 must not expose an executor handle, policy-approval shortcut, Polkit request, arbitrary command, `apply`, or any method that converts the preview into an external effect.

## Experimental Control1 operations

The implementation uses explicit preview names rather than reviving the removed pre-stable generic `Plan` method:

- `PlanDesiredState` — authenticate and decode the request, then delegate to `linura-control`; the control plane returns an exact retained result for an identical replay or, for a first-seen request, observes the target through the requested authoritative route, builds a deterministic non-executable preview, and retains it in the bounded daemon-lifetime plan store.
- `GetPlanPreview` — authenticate the principal and delegate to `linura-control` to retrieve the exact retained preview by typed plan ID without re-observing or mutating the machine.
- `ExplainPlanPreview` — authenticate the principal and delegate to `linura-control` to return the retained preview's semantic origin, authoritative evidence identity, ordered differences, blockers, and status without recomputing or executing it.

The checked-in D-Bus XML, runtime interface, control-plane API, SDK client, CLI, contract tests, and documentation must change atomically when these methods change.

## Planning request

A request carries only data needed to describe desired state:

- caller-supplied `request_id`;
- provider, resource, and observation-capability route;
- semantic summary;
- intent, requirement, and capability origin IDs;
- one or more desired state key/value pairs.

The actor and authenticated principal are deliberately absent from the public request payload because they are derived from authenticated transport credentials.

At least one semantic origin must survive validation. Duplicate origins, malformed IDs, empty state, duplicate/conflicting state keys, control characters, unsupported providers/capabilities, stale/future evidence, and observation identity mismatches fail closed.

The public request boundary is explicitly resource-bounded by `linura-control` before planning or retention:

- semantic summary: at most 4 KiB of UTF-8 bytes;
- intent origins: at most 64 entries;
- requirement origins: at most 64 entries;
- capability origins: at most 64 entries;
- total semantic origins: at most 128 entries;
- desired-state attributes: 1 through 128 entries;
- attribute keys: existing typed/planner limit of 256 bytes each;
- attribute values: existing planner limit of 4 KiB each;
- normalized request payload: at most 64 KiB of counted UTF-8 input bytes across variable-length fields.

The control plane rejects over-limit requests before authoritative observation so an authenticated local caller cannot use observation work or retained previews as an unbounded memory/CPU amplifier. A transport may reject the same bounds earlier, but that is defense in depth and is not the source of authority policy.

## Observation binding and retry semantics

For a **first-seen** request ID, `PlanPreviewControl::plan_desired_state` performs a fresh `ObservationCoordinator::observe` call for the exact provider/resource/capability tuple. Only a validated current observation is projected into `PlanningObservation`.

Before any re-observation, `linura-control` normalizes the authenticated principal, typed route, semantic reason, and ordered desired-state map and checks retained state by request/plan ID:

- same authenticated principal + same ID + byte-for-byte equivalent normalized typed input returns the exact retained preview without observing again, even if a reconnect gives the client a new transport session identity;
- same authenticated principal + same ID + different normalized input fails with an idempotency conflict before observation;
- a new ID performs a new authoritative observation and may therefore bind to a new evidence ID;
- a different authenticated principal has an independent request-ID namespace and cannot read or replay another principal's retained preview.

For D-Bus, the retained preview keeps the actor derived from the first accepted D-Bus request for audit provenance, while the replay namespace uses the stable authenticated Unix UID-derived principal. This avoids both cross-user disclosure and false idempotency conflicts caused solely by D-Bus reconnects.

This pre-observation replay check is required because native provider reads may advance observation sequence numbers even when machine state is unchanged. Re-observing first would create a different evidence ID and would break response-loss retry idempotency.

The preview records the exact evidence ID used for the diff. A later observation is not silently substituted into a retained preview. A caller that wants a new assessment creates a new request ID and receives a new evidence-bound preview.

## Retention and idempotency

The v0.2.0 plan store is owned by `linura-control`, bounded, and process-local. `GetPlanPreview` and `ExplainPlanPreview` therefore work only for previews retained by the current daemon process and owned by the authenticated principal.

This is not durable prepare/commit state and is not crash-recovery evidence. Restart may discard previews. Durable replay prevention and transaction semantics remain later milestones.

Within one authenticated-principal namespace, a planning request ID identifies one evidence-bound preview. Reusing an identifier for a materially different normalized request is an idempotency conflict rather than an overwrite.

Retention is bounded by **both entry count and memory budget**. The initial implementation contract is:

- maximum retained entries: 256;
- maximum estimated retained bytes for one normalized request + preview: 128 KiB;
- maximum aggregate estimated retained bytes: 8 MiB;
- deterministic oldest-first eviction when either aggregate bound would otherwise be exceeded;
- reject a single request/preview that exceeds the per-entry budget instead of evicting the whole cache to accommodate it.

Byte accounting includes the authenticated-principal/key material and all retained variable-length strings and collection elements in the normalized request and preview, not only desired-state values. Tests cover aggregate-byte exhaustion, oversized single entries, replay after retention, cross-principal request-ID isolation, and ID reuse with different normalized input.

## Transport boundary

`linura-dbus` is an adapter, not a second control plane. Its planning module may:

- define D-Bus wire aliases;
- parse typed IDs and reject malformed wire data;
- reject duplicate map keys that would be lost during wire-to-map conversion;
- encode/decode `PlanPreview` values and assert transport-facing invariants such as `execution_authorized=false`.

It must not own or invoke the deterministic planner directly, decide replay/idempotency semantics, retain previews, or perform authoritative observation as part of plan orchestration. Those responsibilities belong to `linura-control`. The D-Bus crate therefore depends on the control-plane preview API rather than directly depending on `linura-planner` or `linura-observation` for this path.

## CLI surface

`linuractl` exposes explicit preview-oriented commands. The first narrow syntax targets one resource with ordered `key=value` desired attributes while retaining route inference for existing systemd/NetworkManager observation resources.

CLI output remains line-safe and machine-readable. It surfaces at least:

- plan/request ID;
- authenticated actor ID;
- provider/resource/capability;
- evidence ID;
- prospective risk;
- preview status;
- `execution_authorized=false`;
- ordered state changes;
- ordered findings/blockers;
- semantic origin.

## Disposable-VM acceptance

The exact-source VM qualification proves externally observable behavior, not only unit tests:

1. create a disposable systemd fixture in a known inactive state;
2. observe that state through Linura;
3. request a preview whose desired `active_state` differs;
4. assert `change-proposed`, `system-mutation`, and `execution_authorized=false`;
5. retry the identical request ID and prove the exact retained preview/evidence ID is returned without creating a new observation;
6. retrieve and explain the retained preview;
7. re-observe the fixture and verify it is still inactive;
8. verify native `systemctl` state is still inactive;
9. request an already-satisfied desired state and observe `no-change`;
10. request an attribute absent from authoritative evidence and observe a fail-closed `blocked` preview;
11. reuse a retained request ID with different desired input and prove it fails closed before observation.

Changes under `crates/linura-control/**` are part of the dedicated plan-preview VM workflow path filter, so moving or changing authority orchestration cannot bypass this acceptance evidence.

The existing authoritative-observation scenario remains a regression requirement. Planning qualification must not weaken v0.1.0 observation, authentication, graph, or evidence guarantees.

## Threat-model impact

This increment expands the authenticated local read/planning surface but does not expand mutation authority. The new attack surface is untrusted desired-state input, replay/identity confusion, resource consumption in plan retention, transport/control-plane semantic drift, and semantic/evidence confusion. Mitigations are typed bounded input enforced by the control plane, transport-derived actor provenance, a stable transport-neutral authenticated-principal namespace, centralized pre-observation replay semantics, fresh authoritative observation for first-seen requests, exact evidence binding, fail-closed validation, count-and-byte-bounded retention, deterministic eviction, a single control-plane orchestration implementation shared by transports, and an unrepresentable execution-authorized state.

Any implementation that duplicates replay/planning/retention authority inside a transport, adds an executor call, policy authorization, Polkit interaction, privileged helper, shell command, durable prepare record, or public apply operation is outside this increment and requires a separate authority review.
