# Architecture

Linura separates **experience**, **intelligence**, and **authority**. Only the authority plane can cause trusted system effects.

## Process and trust boundaries

```text
              user session / local machine

 FirstBoot   Agent UI   Control Center   Shell   CLI/SDK
     │          │             │            │       │
     └──────────┴─────────────┴────────────┴───────┘
                            │
                 versioned local protocol
                            │
               ┌────────────▼────────────┐
               │        linurad          │
               │ unprivileged authority │
               ├─────────────────────────┤
               │ intents / system graph │
               │ desired/observed state │
               │ planner / diff         │
               │ policy / approvals     │
               │ verification / audit   │
               └────────────┬────────────┘
                            │
                explicit privileged effect
                            │
                         Polkit
                            │
           ┌────────────────▼────────────────┐
           │ narrow privileged executors    │
           │ package/systemd/firewall/...   │
           └────────────────┬────────────────┘
                            │
       systemd / NM / UDisks / nftables / package APIs
                            │
                          Linux
```

Agent/model processes live outside the authority plane. They may read scoped context and emit structured `IntentProposal` objects. They do not receive privileged executor handles.

## Core data flow

```text
Conversation / API / imported profile
              │
              ▼
           Intent
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

## Layering

1. `linura-core`: stable IDs, action/effect/risk primitives and semantic reasons.
2. `linura-intent`: durable user intent, requirements, lifecycle and machine profiles.
3. `linura-graph`: causal/dependency/conflict/ownership graph of the managed machine.
4. `linura-capability-sdk`: declarative capability blueprints and composition relations.
5. `linura-planner`: deterministic intent/capability resolution and desired-state derivation.
6. `linura-provenance`: append-only semantic why-chain.
7. `linura-policy`: authorization and approval decisions.
8. `linura-protocol`: versioned local/public contracts.
9. `linura-provider-sdk`: Linux subsystem adapters and effectors.
10. `linura-agent-runtime`: provider-neutral intent interpreters and specialist advice.
11. `linura-control`: Linura Control; unprivileged authority/control-plane orchestration.
12. `linura-sdk`: public non-privileged domain/protocol façade for clients and integrations.
13. narrow privileged executors.
14. client applications.

Dependencies point inward. UI and agents do not import distro/provider implementations.

## Persistence boundary

Persistent local state must eventually include intents, requirements, graph edges, desired state, provenance, approvals, idempotency records, reconciliation state and audit events. Observed Linux state remains authoritative for what is actually true on the machine; the database never fabricates current system state.
