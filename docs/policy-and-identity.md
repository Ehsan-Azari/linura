# Policy and identity

## Actor types

- `Human`: interactive local user.
- `Service`: trusted local service with explicit identity.
- `Agent`: AI/automation principal with scoped grants.
- `Remote`: authenticated remote principal (future).

Actor type does not itself grant authority.

## Policy input

A decision evaluates:
- actor identity/type;
- requested action and resource;
- capability/provider;
- risk/privilege class;
- session context (interactive/headless/remote);
- machine policy;
- requested persistence/recurrence;
- approval evidence.

## Decisions

- `Allow`
- `Deny(reason)`
- `RequireApproval(class)`

Unknown policy state defaults to deny.

## Agent grants

Agents receive explicit scopes such as:

```text
network.read
network.wifi.connect
service.read
package.plan
```

A grant like `package.plan` does not imply `package.apply`. The system should support short-lived delegation and revocation.
