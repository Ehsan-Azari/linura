# Product vision

> **Linura — The intelligent system layer for Linux.**

## Defining sentence

> **Tell your computer what you want it to become.**

Linura is not an AI chat panel embedded in Linux Settings. It is an intent-configured operating environment in which approved intent becomes durable structured state.

## Product loop

```text
user intent
  → structured intent proposal
  → requirements and constraints
  → capability/dependency/conflict resolution
  → desired system state
  → observed-state diff
  → deterministic action plan
  → policy and approval
  → trusted execution
  → independent verification
  → semantic provenance + audit
  → reconciliation over time
```

The loop also runs in reverse when an intent is retired: Linura computes derived state, shared ownership and removal impact before proposing cleanup.

## Starts minimal, becomes personal

The first-boot experience can start from a recoverable base rather than a fully opinionated desktop. A user can describe roles, preferences and constraints such as development, creative work, security posture, travel, accessibility, visual style, or hardware usage. Linura composes supported capabilities and shows the resulting system plan.

The result is not a bag of packages. Linura retains the causal graph explaining why each managed resource exists.

## Agent-native, not agent-dependent

Model providers are optional adapters. A Linura machine remains inspectable, controllable, explainable, and recoverable through deterministic clients without network/model access.

## Product surfaces

- Linura First Boot: establish initial intents/profile.
- Linura Agent: conversationally propose/change/retire intent.
- Linura Control Center: inspect current/desired state, approvals, drift, graph, and provenance.
- Linura Shell: cohesive desktop surfaces consuming the same APIs.
- CLI/Linura SDK: deterministic automation, integrations and recovery.
- Enterprise/Fleet: optional remote policy/orchestration built on the same local authority model.

## Product hierarchy

Linura is the umbrella. **Linura OS** is the installable distribution when that product exists; **Linura Control** is the local authority subsystem; **Linura Agent** is the intent experience; **Linura Shell** and **Linura Control Center** are graphical clients; **Linura SDK** and `linuractl` are deterministic developer/automation surfaces. All use the Linura namespace and the same authority model.
