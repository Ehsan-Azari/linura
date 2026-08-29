# Enterprise and fleet architecture

Enterprise features extend the local control model; they do not replace it. A standalone machine must remain fully manageable without a SaaS control plane.

## Future topology

```text
                 Enterprise control plane (optional)
        policy / inventory / rollout / audit collection
                          |
                    mutually-authenticated
                       fleet gateway
                          |
            +-------------+-------------+
            |                           |
         machine A                    machine B
     local Linura API           local Linura API
            |                           |
      providers/executors          providers/executors
```

## Enterprise capabilities

- device enrollment and machine identity;
- organization/device/user policy layers;
- RBAC/ABAC and scoped service identities;
- desired-state policies and drift reporting;
- staged/canary rollouts and maintenance windows;
- approval workflows for high-risk actions;
- inventory and support matrix reporting;
- audit export to SIEM/object storage;
- compliance evidence views;
- offline-tolerant policy cache with bounded validity;
- revocation and break-glass procedures;
- remote diagnostics with explicit user/admin policy;
- update rings and rollback coordination.

## Non-negotiable properties

- Remote authority is explicit and revocable.
- A cloud outage cannot brick local management.
- Remote requests traverse the same local policy/action/verification path as local requests.
- Fleet services never receive a generic remote shell as the normal management primitive.
- Tenant/organization boundaries apply to control metadata if a hosted control plane is introduced.
- Enterprise connectors/exporters are optional providers/sinks, never the local system source of truth.

## Not in v0.1

No remote listener, fleet enrollment, or central SaaS dependency should enter the first release. The local security and audit semantics must be stable first.
