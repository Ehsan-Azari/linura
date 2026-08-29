# ADR 0004: D-Bus local boundary; remote gateway later

Status: Accepted

## Decision

Use D-Bus as the first local transport and authorization context. Keep domain/protocol types transport-neutral. Do not expose the session daemon directly over TCP.

Remote/fleet management, when introduced, is a separate authenticated gateway (likely structured RPC over mTLS) that calls the local control plane.
