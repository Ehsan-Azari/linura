# Agent architecture

## Absolute rule

**An agent is an untrusted proposer, never an authority.**

Agent providers translate natural language/context into typed `IntentProposal` objects and optional advisory material. They cannot directly invoke privileged executors.

## Provider neutrality

The intelligence layer supports adapters for hosted models, local models, enterprise models, deterministic/rule-based interpreters, or no model at all. No provider is a core architectural dependency.

## Specialist roles

Logical specialists can include coordinator, hardware, security, developer, desktop, productivity and future domain roles. Specialists share scoped system context and produce advice/proposals; they do not become independent authorities.

## Multi-agent disagreement

When specialist recommendations conflict, the planner records the conflict and surfaces alternatives. It never resolves a security-sensitive disagreement by allowing one model to execute directly.

## Context and secrets

Agent context is capability-scoped and minimized. Secrets are represented by references/handles and are not inserted into general model context by default.

Agent-facing context should be supplied through typed/bounded context projections rather than by granting a model direct unrestricted provider access. A future context-query runtime may aggregate current observations, historical evidence and retrieval sources under explicit resource/freshness budgets.

RAG and retrieval are advisory reasoning inputs. They may explain likely causes, surface documentation or select relevant history, but they cannot:

- fabricate an authoritative `ObservationEnvelope`;
- turn stale/cached evidence into current truth;
- satisfy a required provider observation merely by semantic similarity;
- authorize or execute an effect;
- override policy, approval, verification or reconciliation.

If an agent needs a fact that is authoritative for planning or policy, the control plane obtains and validates the required provider observation independently of the model's claim.
