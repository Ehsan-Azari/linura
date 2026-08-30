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

Dependencies point inward. UI, Library adapters and agents do not import distro/provider implementations.

## Persistence boundary

Persistent local state must eventually include intents, requirements, reusable setup/profile revisions, Library metadata, graph edges, desired state, provenance, approvals, idempotency records, reconciliation state and audit events. Observed Linux state remains authoritative for what is actually true on the machine; the database never fabricates current system state.

Portable export/import is separate from authority-state backup. Portable artifacts preserve reusable declarative meaning; authority-state backup preserves local operational/evidence records; filesystem snapshots preserve exact machine recovery state.
