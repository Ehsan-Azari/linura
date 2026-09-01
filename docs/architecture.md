# Architecture

Linura separates **experience**, **intelligence**, and **authority**. Only the authority plane can cause trusted system effects.

## Process and trust boundaries

```text
              user session / local machine

 FirstBoot   Agent UI   Library   Control Center   Shell   CLI/SDK
     │          │          │             │            │       │
     └──────────┴──────────┴─────────────┴────────────┴───────┘
                               │
                    versioned local protocol
                               │
                  ┌────────────▼────────────┐
                  │        linurad          │
                  │ unprivileged authority  │
                  ├─────────────────────────┤
                  │ intents/setups/graph    │
                  │ desired/observed state  │
                  │ planner / diff          │
                  │ policy / approvals      │
                  │ verification / audit    │
                  └────────────┬────────────┘
                               │
                   explicit privileged effect
                               │
                            Polkit
                               │
              ┌────────────────▼────────────────┐
              │ narrow privileged executors     │
              │ package/systemd/firewall/...    │
              └────────────────┬────────────────┘
                               │
          systemd / NM / UDisks / nftables / package APIs
                               │
                             Linux
```

Agent/model processes live outside the authority plane. They may read scoped context and emit structured `IntentProposal` objects. They do not receive privileged executor handles.

The Linura Library is also outside execution authority: it stores/catalogs reusable declarative artifacts. Loading or synchronizing an artifact cannot mutate the machine until Linura Control validates/adopts it through the normal planning and authority path.

## Core data flow

```text
Conversation / API / saved Setup / imported Profile
                    │
                    ▼
             Intent / adoption
                    │
              requirements
                    │
                    ▼
            Capability Solver
              │            │
        dependencies     conflicts
              └──────┬─────┘
                     ▼
              Desired State
                     │
             Observed State
                     │
                     ▼
                    Diff
                     │
                     ▼
                    Plan
                     │
              policy/approval
                     │
                     ▼
                  Effects
                     │
                Verification
                     │
                     ▼
            Provenance + Audit
                     │
              System Graph
```

Saved setup/profile adoption does not enter below intent/planning. It is never an executor replay mechanism.

## Context acquisition and query plane

Linura must answer increasingly broad semantic questions without turning D-Bus, shell calls or any other transport into its model of the machine. The observation side therefore follows a separate transport-neutral flow:

```text
Semantic context query
        │
        ▼
query planning/orchestration
        │
        ▼
   bounded probes
        │
   ┌────┼───────────────┬─────────────┐
   ▼    ▼               ▼             ▼
systemd hardware     containers     storage/...
provider provider      provider       provider
   │    │               │             │
 D-Bus sysfs/...   socket/API/...   native/...
   └────┴───────────────┴─────────────┘
        │
        ▼
 ObservationEnvelope
        │
        ▼
ObservationCoordinator
        │
        ▼
    System Graph
        │
        ▼
 Context Projection
    │            │
 planner       agent/RAG
```

A **probe** is one bounded provider-backed acquisition attempt. A **context query** may require one or many probes. A future query runtime may own deadlines, cancellation, bounded concurrency/fan-out, retries, query coalescing, cache/freshness policy, backpressure, partial-result semantics and aggregate resource budgets.

This query plane is not a second authority plane and does not add a stage to the managed-mutation lifecycle. When planning, policy or verification requires current machine truth, the required authoritative observation/freshness contract still applies.

D-Bus has two permitted adapter roles: local Linura client/control transport and provider-internal transport to upstream Linux services. D-Bus object paths, interfaces, signals, Unix file descriptors and wire values remain adapter details in both roles.

Cached observations, context projections and retrieval/RAG may improve efficiency or reasoning, but they do not become current machine truth merely by being available. Retrieval cannot manufacture observed state or grant authority.

## Layering

1. `linura-core`: stable IDs, action/effect/risk primitives and semantic reasons, including setup identity.
2. `linura-intent`: durable user intent, reusable setups, requirements, lifecycle and machine profiles.
3. `linura-graph`: causal/dependency/conflict/ownership graph of the managed machine, including setup provenance.
4. `linura-capability-sdk`: declarative capability blueprints and composition relations.
5. `linura-planner`: deterministic intent/capability resolution and desired-state derivation.
6. `linura-provenance`: append-only semantic why-chain.
7. `linura-policy`: authorization and approval decisions.
8. `linura-protocol`: versioned local/public contracts including setup/profile portability/adoption.
9. `linura-provider-sdk`: Linux subsystem adapters and effectors.
10. `linura-agent-runtime`: provider-neutral intent interpreters and specialist advice.
11. `linura-control`: Linura Control; unprivileged authority/control-plane orchestration.
12. `linura-sdk`: public non-privileged domain/protocol façade for clients and integrations.
13. narrow privileged executors.
14. client applications.

The observation path refines those layers without changing their authority direction:

- `linura-observation` owns the canonical authoritative observation envelope/freshness primitives;
- `linura-observation-control` owns provider-neutral observation coordination and bounded in-process retained evidence;
- `linura-linux-observation` contains concrete Linux observation adapters and may depend on transport libraries such as `zbus`;
- `linura-dbus` contains the local D-Bus transport and must delegate authority/planning semantics inward rather than owning them.

Dependencies point inward. UI, Library adapters and agents do not import distro/provider implementations. Semantic/planning crates do not import transport libraries or concrete Linux providers. The machine-readable layering contract under `contracts/layering.toml` is validated in repository checks to prevent transport/provider coupling from silently creeping inward.

## Persistence boundary

Persistent local state must eventually include intents, requirements, reusable setup/profile revisions, Library metadata, graph edges, desired state, provenance, approvals, idempotency records, reconciliation state and audit events. Observed Linux state remains authoritative for what is actually true on the machine; the database never fabricates current system state.

Portable export/import is separate from authority-state backup. Portable artifacts preserve reusable declarative meaning; authority-state backup preserves local operational/evidence records; filesystem snapshots preserve exact machine recovery state.
