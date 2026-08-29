# System graph

The system graph is Linura's causal view of the machine. It unifies intent, requirements, capabilities, workflows and concrete resources without pretending their underlying Linux implementations are identical.

## Node families

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

## Why it matters

If `intent:ai-development` and `intent:gaming` both require an NVIDIA runtime, retiring AI development must retain the shared runtime. Linura reasons over semantic ownership in addition to package-manager dependency metadata.

The graph powers `explain`, conflict analysis, safe intent retirement, reconciliation, generated UI context, profile export, and future fleet reasoning.
