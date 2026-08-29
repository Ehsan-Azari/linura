# Development plan

The development order proves the intent-native model **without depending on an LLM first** and preserves the canonical mutation lifecycle from the first real effect.

## Phase 0 — Linura architecture lock (current)

Exit criteria:
- Linura naming is complete;
- product vision and trust boundaries are explicit;
- intent, graph, capability, provenance, policy and provider contracts exist;
- the canonical eleven-stage mutation lifecycle is encoded in code and ADRs;
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

Implement typed plan, risk classification, structural validation, policy decision, approval requirement and planned provenance/audit. Every managed resource in the plan must retain an intent/requirement/capability origin. Planning must consume authoritative observation rather than assumed state.

## Phase 4 — durable prepare/commit and recovery foundation

Select local persistence via ADR and implement the transaction boundary required before any supported external mutation:
- request/plan idempotency;
- durable `prepare` intent-to-execute record;
- indeterminate-operation recovery state;
- verified `commit` transaction for desired state/graph/provenance;
- append-only success/failure audit records;
- migration and corruption-detection basics.

A crash after prepare must recover by re-observing authoritative state, never by blindly replaying an effect.

## Phase 5 — first narrow privileged executor and independent verifier

Implement `linura-executor-systemd` over systemd D-Bus with strict unit validation and Polkit. No arbitrary command execution.

Implement verification as a separate boundary consuming post-execution authoritative observation. Executor success alone is never state proof.

## Phase 6 — first complete eleven-stage vertical slice

Make one narrow capability traverse the entire canonical path:

```text
request / intent
→ observe
→ plan
→ validate
→ authorize
→ prepare
→ execute
→ verify
→ commit
→ audit
→ reconcile
```

Add denial tests, approval tests, failure injection, crash/indeterminate recovery, compensation where applicable, drift tests and VM acceptance evidence. A successful effect without successful verification/commit/audit is not a successful managed mutation.

## Phase 7 — persistent intent lifecycle + safe retirement

Persist intents, requirements, desired state, graph, provenance, approvals, audit and reconciliation state. Implement suspend/supersede/retire and shared-ownership removal impact using the same canonical mutation lifecycle for resulting changes.

## Phase 8 — agent interpretation

Introduce model/provider adapters whose only authority output is `IntentProposal`. Add specialist advice and disagreement/conflict handling. Test prompt injection, malicious tool proposals, stale context and provider unavailability.

## Phase 9 — first boot

Implement the signature flow: "What do you want this computer to become?" including offline/default/import paths, hardware discovery, plan review, approval, snapshot and recovery escape hatches.

## Phase 10 — expand system domains

Network, Bluetooth, audio, power, storage, packages, firewall, updates/snapshots, displays, containers/virtualization and other system domains. Every managed domain uses the same eleven-stage mutation lifecycle rather than defining a domain-specific authority shortcut.

## Phase 11 — workflows and derived UI

Add declarative workflow runtime and constrained derived surfaces. Custom code uses isolated extensions only. Workflow steps still enter Linura Control through typed requests and cannot bypass the mutation lifecycle.

## Phase 12 — Control Center and shell

Build clients over the same protocol. No provider-specific backend logic in UI and no privileged UI shortcut around Linura Control.

## Phase 13 — supported release hardening

Installer, migrations, snapshots, recovery drills, hardware matrix, security review, SBOM/signing/attestations, reproducible packaging, documentation and soak tests.

## Phase 14 — optional enterprise/fleet

Only after local trust is proven: enrollment, mTLS, remote policy, fleet desired intent/state, audit export, staged rollout and rollback. Remote/fleet requests enter the same local authority lifecycle.
