# Security Policy

Linura changes operating-system state and accepts untrusted model/user input. Security is a product requirement, not a later hardening phase.

## Supported versions

No production-supported version exists yet. Security fixes apply to the active development branch until a supported release line is declared.

## Security invariants

1. **Agents propose; Linura authorizes.** Model output can become an `IntentProposal`; it never becomes execution authority.
2. **Approved structured intent is durable.** Conversation/model text is not authoritative configuration.
3. **Semantic provenance is required.** Managed state must retain a trustworthy why-chain to approved intent/requirements/capabilities.
4. **No root monolith.** `linurad` runs unprivileged; privileged effects use narrow executors.
5. **No generic privileged shell.** Public/agent APIs cannot execute arbitrary elevated command text.
6. **Explicit actors and grants.** Human/service/agent identity and scope are authenticated and policy-evaluated.
7. **Deny by default.** Unknown actors, capabilities, resources, policy states and unsupported observations cannot mutate the machine.
8. **One ordered authority lifecycle.** Successful managed mutations follow request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile; providers cannot bypass or reorder it.
9. **Observe before plan.** Material plans consume authoritative current state rather than assumed state.
10. **Plan and validate before authority.** Material effects, risk, reversibility and verification are inspectable and validated before privilege.
11. **Prepare before external effects.** A durable intent-to-execute/recovery record exists before supported side effects cross the authority boundary.
12. **Verify independently after execution.** Executor success is not state proof; success requires authoritative postcondition evidence through a separate verification boundary.
13. **Commit only after verification.** Desired state, graph and semantic provenance are finalized only after required postconditions are proven.
14. **Audit success and failure.** Request, policy/approval, execution, verification, commit, compensation and indeterminate outcomes remain correlated and auditable.
15. **Safe retirement.** Removing intent performs dependency/shared-ownership analysis before cleanup.
16. **No secret argv or general model-context leakage.** Secrets use protected handles/channels and are minimized/redacted.
17. **No unsandboxed extension/generated code in authority processes.** Derived UI is constrained; custom extensions are isolated.
18. **Agent-native is not agent-dependent.** Deterministic offline control/recovery remains available if model providers fail or are compromised.
19. **Least privilege.** Each privileged executor exposes only a narrow typed method set with minimum OS access.

## Reporting a vulnerability

Do not open a public issue for suspected vulnerabilities. Use GitHub private vulnerability reporting when hosted, or the security contact published by the project owner.

Include the affected version/commit, component/trust boundary, reproduction/PoC, impact, required attacker access and suggested mitigation if known.

## Security-review triggers

A pull request requires a threat-model update or explicit “no threat-model impact” rationale when it changes:
- canonical mutation lifecycle ordering or stage semantics;
- intent/provenance trust semantics;
- agent/provider/context boundaries;
- authentication/actor identity or grants;
- policy/approval evaluation;
- privileged executor or verifier interfaces;
- D-Bus/remote exposure;
- persistence/prepare/commit/audit formats;
- dependency/conflict solver semantics;
- capability/workflow/extension loading;
- generated/derived UI authority;
- package/update/release verification;
- secret handling.

## Install, update, and recovery baseline

The first supported Linura OS profile is fail-closed by design: encrypted supported installs, inbound firewall deny-by-default, SSH initially disabled, and untrusted package sources disabled until explicitly enabled. The agent/model layer is never a recovery dependency.

Linura coordinates system upgrades so snapshot, migrations, reconciliation, and verification cannot be silently skipped. An Arch ALPM guard represents this policy for the initial profile. Administrators retain an explicit break-glass native recovery path (`LINURA_ALLOW_DIRECT_PACMAN=1`) because a broken Linura control plane must not make the host unrecoverable.

Hardware fixtures and acceptance evidence must be sanitized. Never commit serial numbers, MAC addresses, private account identifiers, or raw diagnostic archives containing user data.
