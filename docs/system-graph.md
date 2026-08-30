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

## Why setups belong in the graph

A setup is reusable declarative provenance, not merely an import-time container. When `setup:rust-development` is adopted, the resulting intent can retain a `derived-from` relationship to the setup/revision that supplied it. This lets Linura answer both:

- why does this resource exist?
- which saved setup/profile introduced the intent behind it?

If a setup is later removed from a profile, Linura still evaluates intent/resource ownership and shared dependencies before proposing cleanup.

## Why shared ownership matters

If `intent:ai-development` and `intent:gaming` both require an NVIDIA runtime, retiring AI development must retain the shared runtime. Linura reasons over semantic ownership in addition to package-manager dependency metadata.

The graph powers `explain`, conflict analysis, safe intent retirement, setup/profile adoption lineage, reconciliation, generated UI context, profile export, and future fleet reasoning.
