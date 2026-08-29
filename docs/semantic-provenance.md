# Semantic provenance: why state exists

Traditional audit logs answer **who changed what and when**. Linura must additionally answer **why this state exists**.

Every managed resource traces through a causal chain:

```text
User intent
  → requirement
  → capability
  → desired resource
  → plan
  → effect
  → verification
```

Example explanation:

```text
sshd.service is enabled
Reason: secure remote development
Requested by: intent:remote-development
Constraint: Tailscale-only exposure
Derived capability: remote.ssh
Current state: matches desired state
```

Provenance is append-only lineage; compensation or later intent retirement never rewrites historical evidence.
