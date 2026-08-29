# ADR 0002: No privileged monolithic daemon

Status: Accepted

## Decision

The session control plane is unprivileged. Root-required operations live in separate domain-specific executors with minimal APIs and systemd sandboxing. Authorization is explicit (Polkit/local policy) and not delegated to arbitrary clients.

## Consequence

More IPC/process boundaries are required, but compromise of the main UI/session process does not automatically become unrestricted root code execution.
