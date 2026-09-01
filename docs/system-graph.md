# System graph

The system graph is Linura's causal view of the machine. It unifies reusable setups, intent, requirements, capabilities, workflows and concrete resources without pretending their underlying Linux implementations are identical.

## Node families

- setup
- intent
- requirement
- capability
- workflow
- application/service/package/configuration/resource
- device/hardware resource
- observation evidence and provider/capability relationships where projected by the observation control plane

## Edge semantics

- `requires`
- `provides`
- `conflicts`
- `replaces`
- `recommends`
- `optional`
- `owns`
- `shared-by`
- `derived-from`
- `realizes`

## Observation evidence and materialization

**The System Graph is a materialized causal/evidence projection, not current-state authority.**

Authoritative current machine truth still comes from the responsible provider through a validated `ObservationEnvelope`. The observation control plane may project current and retained evidence into the graph so Linura can explain provider/resource/capability relationships, correlate provenance, answer bounded context queries and reuse still-valid evidence.

A graph node does not become permanently current merely because it was once derived from authoritative evidence. Freshness remains attached to the underlying observation and must be re-evaluated before a consumer treats retained evidence as current. When planning, policy or verification requires fresh authority, the graph cannot waive that requirement.

Future context projections may read the graph together with current observations, history, diagnostics or retrieval indexes. Those projections remain derived views and do not create a second state authority.

## Why setups belong in the graph

A setup is reusable declarative provenance, not merely an import-time container. When `setup:rust-development` is adopted, the resulting intent can retain a `derived-from` relationship to the setup/revision that supplied it. This lets Linura answer both:

- why does this resource exist?
- which saved setup/profile introduced the intent behind it?

If a setup is later removed from a profile, Linura still evaluates intent/resource ownership and shared dependencies before proposing cleanup.

## Why shared ownership matters

If `intent:ai-development` and `intent:gaming` both require an NVIDIA runtime, retiring AI development must retain the shared runtime. Linura reasons over semantic ownership in addition to package-manager dependency metadata.

The graph powers `explain`, conflict analysis, safe intent retirement, setup/profile adoption lineage, reconciliation, generated UI context, profile export, and future fleet reasoning.
