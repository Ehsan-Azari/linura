# Development plan

The development order proves the intent-native model **without depending on an LLM first**.

## Phase 0 — Linura architecture lock (current)

Exit criteria:
- Linura naming is complete;
- product vision and trust boundaries are explicit;
- intent, graph, capability, provenance, policy and provider contracts exist;
- first platform profile and recovery constraints exist;
- CI/repository checks reject legacy naming and missing vision artifacts.

## Phase 1 — observe and build the system graph

Implement:
1. session D-Bus service `org.linura.Control1`;
2. caller credential extraction and actor binding;
3. provider registry/capability discovery;
4. systemd + NetworkManager read-only observers;
5. observation envelopes/freshness;
6. resource nodes/edges in the system graph;
7. `linuractl observe`, `graph`, `capabilities`, `explain` (observed evidence only).

No root code required.

## Phase 2 — deterministic intent → desired state, no AI

Create hand-authored intents and capability blueprints. Prove:

```text
Intent → requirements → capability resolution → conflicts → desired state → diff → plan
```

Use a constrained example such as secure SSH on a disposable VM. No model provider yet.

## Phase 3 — plan-only mutation + semantic provenance

Implement typed plan, risk classification, policy decision, approval requirement and planned provenance/audit. Every managed resource in the plan must retain an intent/requirement/capability origin.

## Phase 4 — first narrow privileged executor

Implement `linura-executor-systemd` over systemd D-Bus with strict unit validation and Polkit. No arbitrary command execution.

## Phase 5 — execute → verify → provenance → audit

Add independent postcondition observation and failure injection. A successful effect without successful verification is not success.

## Phase 6 — persistence + intent lifecycle + safe retirement

Select persistence via ADR. Persist intents, requirements, desired state, graph, provenance, approvals, audit and idempotency. Implement suspend/supersede/retire and shared-ownership removal impact.

## Phase 7 — agent interpretation

Introduce model/provider adapters whose only authority output is `IntentProposal`. Add specialist advice and disagreement/conflict handling. Test prompt injection, malicious tool proposals, stale context and provider unavailability.

## Phase 8 — first boot

Implement the signature flow: "What do you want this computer to become?" including offline/default/import paths, hardware discovery, plan review, approval, snapshot and recovery escape hatches.

## Phase 9 — expand system domains

Network, Bluetooth, audio, power, storage, packages, firewall, updates/snapshots, displays, containers/virtualization and other system domains. For each: observe → graph → capability → desired state → plan → policy → execute → verify → provenance.

## Phase 10 — workflows and derived UI

Add declarative workflow runtime and constrained derived surfaces. Custom code uses isolated extensions only.

## Phase 11 — Control Center and shell

Build clients over the same protocol. No provider-specific backend logic in UI.

## Phase 12 — supported release hardening

Installer, migrations, snapshots, recovery drills, hardware matrix, security review, SBOM/signing/attestations, reproducible packaging, documentation and soak tests.

## Phase 13 — optional enterprise/fleet

Only after local trust is proven: enrollment, mTLS, remote policy, fleet desired intent/state, audit export, staged rollout and rollback.
