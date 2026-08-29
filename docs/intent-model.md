# Intent model

Intent is the durable representation of **what the user wants**, not the command used to reach it.

## Lifecycle

`proposed → active ↔ suspended → superseded | retired`

- Proposed: agent/human/import created it but it is not authoritative yet.
- Active: contributes requirements and desired state.
- Suspended: retained but temporarily excluded from reconciliation.
- Superseded: replaced by another intent with preserved lineage.
- Retired: no longer desired; triggers removal-impact analysis rather than blind cleanup.

## Intent vs desired state vs plan vs action

```text
Intent       what should this machine be/be able to do?
Desired      what persistent state should exist?
Plan         what changes are necessary from observed state?
Action       what concrete effects are authorized now?
```

Conversation is never the durable source of truth. Once a proposal is approved, the structured intent/requirements are authoritative.

## Requirements

Each intent expands into goals, constraints, preferences and prohibitions. Ambiguities remain explicit until resolved; a model may not silently invent safety-critical constraints.
