# Action lifecycle

A managed mutation is downstream of approved intent. Direct deterministic API/CLI requests may exist, but persistent managed state still requires explicit semantic provenance.

```text
approved intent / explicit deterministic request
  → requirements / semantic reason
  → desired state
  → observe prerequisites/current state
  → diff + capability check
  → deterministic plan
  → policy decision
  → optional human/admin approval
  → execute narrow effects
  → independently verify postconditions
  → compensate if required/possible
  → update graph/provenance
  → append audit outcome
```

## Action plan

A plan includes:
- plan + request + actor + resource identifiers;
- intent/requirement/capability origin (`SemanticReason`);
- preconditions and freshness bounds;
- ordered effects;
- privilege/risk classification;
- reversibility/compensation metadata;
- expected postconditions and independent verification strategy.

## Idempotency

Every mutation has a request ID. Providers/executors define whether retry is naturally idempotent, deduplicated, or rejected after ambiguous execution. Replanning after observed state changes produces a new plan identity rather than mutating historical evidence.

## Verification

Verification re-observes the authoritative subsystem. The executor that issued a mutation is not automatically trusted as proof that the intended machine state exists.

## Graph/provenance commit

Desired-state/provenance updates and execution audit need crash-safe transaction semantics. If execution happened but persistence is uncertain, recovery marks the operation indeterminate and re-observes before making another effect.
