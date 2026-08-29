# Provider model

Providers adapt Linux subsystems to Linura domain contracts. They supply authoritative observation and deterministic planning inputs; they do not own Linura's global authority sequence.

A provider declares:
- stable provider ID/version;
- supported capabilities;
- resources it can authoritatively observe;
- deterministic actions it can plan from request + observation;
- whether an effect can execute unprivileged or needs a narrow privileged executor;
- expected postconditions and verifier strategy;
- diagnostic metadata.

## Required separation

The provider SDK intentionally separates three concerns:

1. `Provider::observe` returns an authoritative observation for the requested resource.
2. `Provider::plan` consumes both the typed request and that observation to produce an immutable `ActionPlan`.
3. `EffectExecutor` and `EffectVerifier` are separate contracts. Executor success is not accepted as proof that the intended state exists; verification consumes post-execution authoritative observation.

A single implementation package may supply more than one of these roles during early development, but the interfaces remain separate so production deployments can isolate privilege and verification independently.

Providers/executors/verifiers cannot skip `authorize`, `prepare`, `commit`, `audit` or `reconcile`; those are owned by Linura Control's canonical mutation lifecycle.

Expected first providers:

| Domain | Provider |
|---|---|
| Network | NetworkManager over D-Bus |
| Bluetooth | BlueZ over D-Bus |
| Audio/media | PipeWire/WirePlumber |
| Services | systemd D-Bus |
| Storage | UDisks2 + filesystem-specific helpers |
| Authorization | Polkit |
| Snapshots | Snapper |
| Firewall | nftables/firewalld profile, selected by platform profile |
| Packages | pacman on Arch profile |

Providers must not leak raw command output into public API types. Provider-specific diagnostics may be attached in explicitly namespaced diagnostic fields. Observation and verification evidence should be structured enough to support freshness, correlation and audit without making provider-specific text the source of truth.
