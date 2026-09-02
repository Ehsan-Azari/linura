# Policy and identity

Linura separates authenticated authority identity from request provenance. That distinction is mandatory for v0.3 review semantics.

## Principal

The **principal** is the authenticated authority identity used to namespace and bind policy, review and approval state. A transport derives it from trusted credentials; clients, agents and imported artifacts cannot self-assert it.

A change of principal creates a different review subject even when the plan contents are otherwise identical.

## Actor provenance

The **actor** is immutable request provenance carried into the canonical plan:

- `Human`: human-initiated request.
- `Service`: local service/automation request.
- `Agent`: AI/agent-originated proposal or request.
- `Remote`: authenticated remote-origin provenance for future remote surfaces.

Actor kind does not itself grant authority. In particular, `Human` does not mean approved, `Service` does not mean trusted to mutate, and `Agent` can never self-authorize.

## Policy subject

v0.3 policy evaluation consumes a review subject derived by Linura Control from the canonical `linura-planner::ReconciliationPlan`. It binds at least:

- authenticated principal;
- request and plan identity;
- actor provenance;
- provider, resource and capability;
- semantic provenance;
- exact authoritative evidence identity;
- prospective risk;
- deterministic changes/findings and blocked state;
- policy identity and revision through the evaluation binding.

Clients and transports do not construct a second policy-specific plan.

## Decisions

Policy produces one deterministic outcome:

- `Allow`;
- `Deny(reason)`;
- `RequireApproval(class, reason)`;
- `Blocked(reason)`.

Unknown, malformed, unsupported or structurally blocked state fails closed. `Blocked` is distinct from a policy denial: it means the plan is not valid review material and cannot become approvable merely by selecting a more privileged approver.

## Approval binding

Approval evidence must be usable only for the exact review binding that produced the requirement. A different principal, plan, request, authoritative evidence identity, provider/resource/capability or policy revision invalidates reuse. Expiry and revocation are part of the approval lifecycle introduced by v0.3 implementation work.

Most importantly:

```text
policy allow       != execution authority
valid approval     != execution authority
reviewed plan      != prepared mutation
```

v0.3 remains review-only. Durable prepare/recovery begins in v0.4, isolated executor/verifier qualification in v0.5, and the first bounded supported Experimental managed external effect is gated to v0.6.

## Grants

Future grants are scoped authority associated with authenticated principals, for example read or proposal capabilities. A scope such as `package.plan` must never imply `package.apply`. Grants require explicit policy treatment, bounded lifetime/revocation semantics where applicable, and cannot bypass exact plan/evidence review binding.
