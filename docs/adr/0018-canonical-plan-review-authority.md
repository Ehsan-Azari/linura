# ADR 0018: Canonical plan-review authority and exact approval binding

Status: Accepted

## Context

Linura v0.2 established a canonical deterministic, evidence-bound, non-executable plan lineage through `linura-planner::ReconciliationPlan` and the public `PlanPreview` projection. Earlier bootstrap code from v0.0.0 also contained a separate generic path built around `ActionRequest`, provider-owned `Provider::plan`, executable-looking `ActionPlan` values and a generic `ControlPlane::apply`/`MutationRuntime` orchestration scaffold.

That bootstrap path was useful to make the eleven-stage lifecycle concrete during initial architecture work, but it is not the implemented v0.2 product path, has no live production provider/application caller, and conflicts with the current architecture in two important ways:

1. planning authority would sit partly in providers rather than Linura's deterministic planner/control plane;
2. policy could evaluate a second executable-looking plan model instead of the exact canonical plan/evidence the user reviewed.

v0.3 introduces policy and approval semantics. Carrying both plan lineages into the authority milestone would make stale approval, plan substitution, provider-policy coupling and accidental execution promotion materially harder to reason about.

## Decision

Linura has one authority-side plan lineage:

```text
authenticated request
→ authoritative observation
→ deterministic ReconciliationPlan
→ validation
→ policy-review projection
→ policy decision
→ approval requirement/evidence when required
→ reviewed-plan result
```

`linura-policy` receives a `PolicySubject` assembled from the canonical `ReconciliationPlan` plus an authenticated `PrincipalId`. Because Rust has no sibling-crate friend visibility, the domain constructor is necessarily visible to its sole workspace consumer; the architectural guarantee is therefore enforced at the dependency and API boundaries rather than by claiming impossible language-level exclusivity. `contracts/layering.toml` requires `linura-control` to be the only workspace package that consumes `linura-policy`, and the public SDK/wire contracts do not expose or accept `PolicySubject` as independently authored review material.

Linura Control owns the canonical `ReconciliationPlan` → `PolicySubject` projection and authenticated-principal binding. A transport, UI, agent, provider or public client must not create an alternate policy-review path.

The policy evaluation is bound to an explicit `PolicySnapshot` and emits a `ReviewBinding` containing at least:

- authenticated principal;
- request ID;
- plan ID;
- authoritative evidence ID;
- provider/resource/capability identity;
- policy ID/revision.

The v0.3 implementation must additionally verify that material planned changes/findings and semantic provenance still match the reviewed subject before approval evidence is accepted.

Policy outcomes are typed as:

- `Allow`;
- `Deny`;
- `RequireApproval`;
- `Blocked`.

`Blocked` is distinct from `Deny`: a blocked plan lacks sufficient valid planning state to enter approval review at all.

### Approval is not execution authority

The following are separate concepts and may not be collapsed:

```text
policy allow          != execution authority
valid approval        != execution authority
reviewed plan         != prepared mutation
review binding        != executor credential
```

v0.3 stops before the canonical `prepare` stage. Public reviewed-plan results remain explicitly non-executable and must not expose an executor handle, Polkit grant, generic command path or conversion to `apply`.

Durable authorization/prepare binding belongs to v0.4. Narrow executor/verifier qualification belongs to v0.5. The first supported managed external effect remains v0.6 after complete lifecycle integration.

### Exact binding and invalidation

Approval evidence may satisfy only the exact review subject for which it was issued. A changed principal, plan, material plan content, authoritative evidence identity, provider/resource/capability, policy revision, approval requirement, expiry state or revocation state invalidates reuse.

A `PlanId` alone is explicitly insufficient authority evidence. This matters while pre-1.0 plan identity remains intentionally simple and while v0.3 review retention may be process-local.

### Identity boundary

`Actor` and authenticated principal remain distinct:

- `Actor` records request provenance and may identify a human, service, agent or remote source;
- `PrincipalId` identifies the authenticated authority namespace derived by the trusted transport/control boundary.

Client payloads and models cannot choose their authenticated principal. Agent identity never implies elevated authority and an agent cannot satisfy its own required human/admin approval.

### Cleanup of superseded bootstrap scaffolding

The unused `ActionPlan`/`ActionRequest`/`PlanResponse`, generic `Provider::plan`, compact duplicate provider `Observation`, generic `ControlPlane::apply`/`MutationRuntime`, and associated fake provider/execution receipts are removed rather than retained as legacy APIs.

This cleanup does **not** remove future roadmap scaffolding that remains architecturally valid:

- `linura-lifecycle` and the canonical eleven-stage state machine remain;
- authoritative `Observer`/provider infrastructure remains;
- `executors/linura-executor-systemd` remains as a narrow future executor package scaffold;
- future prepare/recovery, executor and verifier contracts remain roadmap requirements.

Executor/verifier authority interfaces will be introduced against the durable prepared-transaction model when v0.4/v0.5 require them rather than preserving an obsolete generic interface prematurely.

## Security assessment

This change is a security-review trigger because it changes policy/approval and authenticated-principal semantics.

The required threat-model controls are:

- approval is bound to exact plan/evidence/policy/principal identity;
- stale or substituted plans/evidence fail closed;
- client-supplied actor data cannot replace transport-authenticated principal identity;
- cross-principal approval reuse is rejected;
- policy revision downgrade/substitution is rejected;
- agents/models cannot mint or satisfy protected approval classes;
- expiry/revocation is checked at authority use, not merely at UI display time;
- no v0.3 approval path reaches prepare/executor privilege;
- future TOCTOU between review and prepare must revalidate the exact binding before external effects.

`docs/threat-model.md` records these cases explicitly.

## Consequences

- v0.3 builds directly on the v0.2 plan/evidence model instead of preserving two competing plan systems.
- Policy can inspect material canonical plan information without depending on D-Bus, concrete providers or executors.
- Linura Control is machine-enforced as the only workspace consumer/orchestrator of the policy domain, preventing a second authority-side review path from appearing in another crate.
- Public SDK/wire clients cannot submit independently authored `PolicySubject` values even though the internal cross-crate constructor remains visible to its sole allowed consumer.
- Experimental bootstrap API cleanup is coherent rather than hidden behind compatibility shims.
- The lifecycle/executor scaffolds needed by later milestones remain available, but their authority contracts are defined only when durable prepare/execution semantics exist.
- Approval/review tests can focus on exact binding, replay and non-execution rather than reconciling two plan models.
- Any future attempt to reintroduce provider-owned planning, add another policy orchestrator, or turn approval into execution authority requires an explicit architecture rebaseline and threat-model review.
