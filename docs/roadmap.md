# Roadmap

This document is Linura's canonical forward-looking roadmap. Released capability claims are defined by the frozen release contracts and qualification evidence under `docs/releases/` and `docs/qualification/`; this roadmap must never be used to expand a published release claim retroactively.

The machine-readable companion contract is `contracts/roadmap.toml`. Repository validation requires the release spine in this document and that contract to stay synchronized.

## Roadmap principles

1. **Trust boundaries before product breadth.** Linura proves identity, observation, planning, authority, durability, execution and verification as separate milestones before broadening to many system domains.
2. **Code presence is not support.** A crate, binary, provider stub, workflow artifact or UI surface does not become a supported capability until its release contract and qualification evidence say so.
3. **Models are untrusted proposers.** Agent/model integrations may produce typed proposals, but they do not gain policy, authorization or mutation authority.
4. **No generic privileged escape hatch.** System domains use narrow typed capabilities and provider-specific executors rather than a generic privileged shell runner.
5. **Independent observation proves effects.** Executor success is never sufficient evidence that intended machine state was reached.
6. **Local authority remains standalone.** Remote control, hosted Library sync, model providers and enterprise services remain optional integrations rather than prerequisites for local operation or recovery.
7. **Version numbers describe proven capability slices.** Pre-1.0 minor releases may introduce externally testable Experimental capability slices; patch releases repair an already-published minor line.

## Canonical managed-mutation architecture

Linura already locks the authoritative eleven-stage managed-mutation lifecycle. Roadmap changes may refine how a stage is implemented or split its proof across releases, but they must not silently redefine this lifecycle:

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

Canonical compact form: `request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile`.

Human/model proposal handling is upstream of the authoritative lifecycle. Requirements, capability resolution, normalized desired state, deterministic diffing, policy evaluation, persistence mechanics and provider routing are typed implementation machinery *inside* the relevant lifecycle stages; they do not create alternative lifecycle stages.

For example, the v0.2 planning implementation expands the `plan`/`validate` portion approximately as:

```text
request / intent
→ requirements + capability resolution
→ normalized desired state
+ authoritative observation
→ deterministic diff / plan
→ structural validation
```

Domain implementations must compose through the canonical lifecycle rather than bypassing or replacing it.

## Release spine

## v0.0.0 — architecture and trustworthy development foundation

**Status:** released  
**Claim class:** Experimental

Established the product/architecture contracts, repository-owned development and acceptance infrastructure, supply-chain/release controls, typed core boundaries and the canonical managed-mutation lifecycle model.

The presence of future-facing executor, First Boot, update, image or UI artifacts in the repository did not make those product capabilities supported.

## v0.1.0 — authoritative observation and causal system graph

**Status:** released  
**Claim class:** Experimental

Proved authenticated local caller identity, authoritative read-only Linux observation, evidence identity/freshness semantics, provider health and causal system-graph projection through deterministic public clients.

No Linura-managed privileged mutation was introduced.

## v0.2.0 — deterministic desired state and non-executable planning

**Status:** released  
**Claim class:** Experimental

Proved deterministic capability resolution, semantic-origin-preserving desired state, authoritative-evidence binding, deterministic diff/validation and retained non-executable plan previews.

Every public preview remains explicitly non-executable and carries `execution_authorized=false`. v0.2.0 absorbed the previously expected separate plan-only milestone, so the next milestone advances to the authority boundary instead of repeating planning work.

## v0.3.0 — policy, authorization, approval, and plan review

**Status:** planned  
**Target claim class:** Experimental

Complete the authority-side review boundary without introducing supported external mutation.

Target capabilities:

- typed deterministic policy outcomes such as allow, deny, require-approval and blocked;
- authenticated actor/principal and authorization context;
- evidence- and plan-bound authorization decisions;
- explicit approval requirements, approver constraints, expiry and revocation;
- policy/decision provenance and explainability;
- deterministic reviewed-plan lifecycle and fail-closed handling for unknown policy;
- transport-neutral authority semantics;
- proof that even an approved plan cannot mutate the machine in this milestone.

Models/agents remain unable to grant themselves authority.

## v0.4.0 — durable transaction and recovery foundation

**Status:** planned  
**Target claim class:** Experimental

Establish durable state required before Linura is allowed to depend on real external effects.

Target capabilities:

- durable request/idempotency identity across restart;
- durable binding between reviewed authorization, exact plan and evidence;
- prepare records and transaction state transitions;
- corruption/version handling for persisted transaction state;
- restart, replay and duplicate-delivery recovery;
- explicit indeterminate-operation representation and recovery rules;
- durable audit foundation for later execution evidence.

This milestone is not permission to introduce managed external mutation merely because durable state exists.

## v0.5.0 — first narrow privileged executor and independent verifier

**Status:** planned  
**Target claim class:** Experimental

Implement and qualify the first narrow privileged executor and independent verifier components **without yet supporting a Linura-managed external mutation as a product capability**.

Target capabilities:

- least-privilege typed executor with no generic shell authority;
- exact prepared-plan/effect binding;
- constrained privileged authorization plumbing suitable for later lifecycle integration;
- one deliberately narrow effect exercised only through disposable qualification authority;
- independent authoritative re-observation after the qualification effect;
- postcondition verification that does not trust executor self-reporting;
- failure-path, timeout and indeterminate-state component evidence;
- no public `apply`/managed-mutation surface and no release claim that users may rely on the qualification-only effect.

Any external effect in v0.5 qualification is test-fixture evidence for the executor/verifier components, not supported Linura mutation authority. The first supported managed external effect remains blocked until v0.6 integrates these components with authorization, durable prepare/recovery, commit, audit and reconciliation through the complete lifecycle.

## v0.6.0 — complete eleven-stage managed mutation

**Status:** planned  
**Target claim class:** Experimental

Prove the first complete end-to-end execution of Linura's already-canonical lifecycle for one narrow capability, and only then permit the first bounded Experimental **supported managed external effect**:

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

The release must exercise success, denial, stale-evidence, crash/restart, executor failure, verification failure, indeterminate outcome and reconciliation paths in disposable system acceptance. A supported mutation must be bound to the exact authorized plan/prepare state and independently verified before commit.

Only after this milestone is complete should broader system-domain mutation become normal roadmap work.

## v0.7.0 — persistent intent lifecycle and local Linura Library

**Status:** planned  
**Target claim class:** Experimental

Turn the trustworthy mutation core into durable local user-owned configuration state.

Target capabilities:

- persistent intent, requirement, desired-state and causal-lifecycle records;
- durable local Linura Library for setup/profile revisions;
- save/export/import/adopt semantics;
- suspend, supersede and retire lifecycle operations;
- safe cleanup/removal-impact analysis based on causal ownership;
- format versioning, migration, corruption handling, backup and recovery;
- portable definitions without making hosted synchronization a dependency.

## v0.8.0 — agent interpretation / IntentProposal

**Status:** planned  
**Target claim class:** Experimental

Add provider-neutral model/agent interpretation only after the deterministic authority and lifecycle foundation exists.

Target capabilities:

- natural language and agent inputs normalized into typed `IntentProposal` values;
- local, hosted and enterprise model adapters behind one proposal boundary;
- explicit uncertainty/assumption provenance;
- deterministic validation before proposals can become authoritative typed intent;
- human/policy approval paths appropriate to risk;
- complete operation without any model provider.

A model never receives direct executor, policy-admin or unrestricted system authority.

## v0.9.0 — First Boot and supported reference environment

**Status:** planned  
**Target claim class:** Experimental

Prove a coherent install/adoption/recovery path on an explicitly bounded reference environment.

Target capabilities:

- First Boot with offline/default/Library/import paths;
- installation, update and native recovery integration;
- first explicitly supported Experimental distribution/desktop/hardware profile boundary;
- essential service/network/audio/power/package/security capabilities needed by the reference experience;
- initial Control Center and shell surfaces;
- upgrade/migration qualification for persistent Linura state.

## v1.0.0 — meaningful end-user Experimental Linura

**Status:** planned  
**Target claim class:** Experimental

Deliver the first coherent end-user Linura experience that combines the trustworthy managed-mutation core, persistent local intent/Library state, proposal-only agent interpretation, First Boot and a supported Experimental reference platform.

`v1.0.0` is a product milestone, not an automatic declaration of Stable or production-supported contracts. Stability is promoted explicitly through Linura's contract-stability policy and evidence, never inferred from the version number.

Expected v1.0 experience includes:

- a coherent local end-to-end intent → reviewed change → verified state workflow;
- manual operation that remains fully usable without AI;
- bounded agent-assisted intent interpretation;
- supported install/update/recovery path for the reference environment;
- setup/profile Library workflows;
- diagnostics, explanation and audit surfaces appropriate to the supported scope;
- explicit unsupported-domain, platform and compatibility boundaries.

## Beyond v1.0 — production hardening and broader support

Production support is an evidence-backed maturity track, not a date or version-number assumption. Reaching v1.0 does not waive the need for contract, platform, security, recovery and operational qualification.

Production-hardening work includes, as applicable to the supported scope:

- explicit promotion of individual contracts from Experimental/Preview to Stable;
- long-lived migration, backup/restore and downgrade/rollback guarantees;
- installer/update/recovery qualification across declared supported platform profiles;
- fault injection, crash/restart, power-loss and indeterminate-operation testing;
- privilege-boundary and threat-model review for every supported mutating provider;
- performance/resource bounds, soak testing and daemon resilience;
- corruption detection/repair and disaster-recovery procedures;
- diagnostics/support bundles with privacy/redaction guarantees;
- compatibility/deprecation policy and release-channel discipline;
- wider distribution, desktop and hardware qualification matrices only where evidence supports them.

No later capability may weaken the local-first authority model merely to enable remote, hosted or enterprise operation.

## Post-v1 strategic tracks

These tracks preserve Linura's long-term product ambitions without assigning speculative version numbers. Exact release inclusion must be chosen explicitly through future milestone rebaselines.

### Personal operating environment

- machine profile/personality composition;
- profile/setup capture from managed causal state;
- reusable workflow/Library integration and replay UX;
- coherent shell surfaces and design system;
- declarative workflows and constrained derived UI surfaces;
- accessibility and keyboard/mouse parity.

### Extension and sharing ecosystem

- capability-isolated extensions;
- signed manifests and update policy;
- canonical setup/profile serialization, content digests and optional signatures;
- optional Git, user-owned, hosted and enterprise Library sync providers;
- UI/workflow extension points;
- local, hosted and enterprise model adapters that retain proposal-only authority.

### General-purpose provider breadth

- deeper networking, packages/apps, firewall, storage/recovery and boot providers;
- containers such as Docker/Podman through typed provider contracts;
- virtualization through provider-neutral VM resources with optional libvirt/QEMU/KVM, Incus or future adapters;
- users/sessions, credentials, printers/scanners, diagnostics and other domain providers;
- domain-specific verification, recovery and policy semantics proportional to risk.

### Optional fleet and enterprise

- enrollment and mTLS identity;
- central policy and audit export;
- enterprise setup/profile catalog policy;
- fleet desired-state orchestration;
- staged deployment, health gates and rollback;
- enterprise model/provider controls.

Fleet, hosted sync and enterprise services remain optional integrations. They never become prerequisites for local authority, local Library use, setup adoption or recovery.

## Independent maturity axes

Release versions are only one axis. Linura tracks the following independently:

| Axis | Question |
|---|---|
| Trust/lifecycle maturity | Which managed-mutation boundaries are proven end to end? |
| Domain maturity | How far has a specific domain such as services, networking, packages or virtualization progressed? |
| Contract stability | Is an API/schema Experimental, Preview or Stable? |
| Platform support | Which exact distribution/desktop/hardware profiles are release-qualified? |
| Product experience | Which end-user workflows are coherent and supported? |

A domain can be implemented in source while still having no supported platform claim. A contract can remain Experimental even when the product version reaches 1.0. A VM can be used for qualification without VM lifecycle management being a Linura product capability.

## Domain maturity levels

System domains use a capability maturity scale independent of release numbering:

| Level | Meaning |
|---|---|
| D0 — identified | Domain exists in the long-term inventory only. |
| D1 — contracted | Typed resource/capability/provider contracts exist. |
| D2 — implemented | Core implementation exists but is not system-qualified. |
| D3 — integrated | Public/control-plane integration exists with negative-path tests. |
| D4 — system-tested | Disposable-machine/system acceptance exists. |
| D5 — release-qualified | Exact-source release evidence exists for the bounded domain claim. |
| D6 — Experimental supported | A published release explicitly supports the bounded capability. |
| D7 — Stable supported | Compatibility/support guarantees have been explicitly promoted and qualified. |

The current [system domain map](system-domains.md) records sequencing classes and known release-qualified slices without assigning obsolete product-version promises to every domain.

## VM and virtualization boundary

Linura currently uses disposable QEMU/KVM virtual machines as **qualification infrastructure**. That harness is repository-owned test/release infrastructure and is not a product virtualization claim; see [Disposable VM acceptance](vm-acceptance.md).

Linura-managed virtualization remains a future system domain. A future virtualization provider may support libvirt/QEMU/KVM, Incus or other backends through provider-neutral typed VM resources and capabilities. No backend becomes a mandatory Linura architectural dependency.

A future VM lifecycle must use the same canonical eleven stages: request/intent → observe VM state → plan typed desired VM state/diff → validate → authorize → prepare → execute through a narrow provider executor → verify through independent re-observation → commit → audit → reconcile.

## Dependency gates

These gates are architectural, not merely scheduling preferences:

- no supported managed external mutation may appear before v0.6 proves the complete eleven-stage lifecycle;
- v0.4 may establish durable prepare/recovery state but still has no external effect authority;
- v0.5 may exercise a narrow executor/verifier only through disposable qualification authority and must not expose or claim supported managed mutation;
- the first supported Experimental effect in v0.6 must depend on durable recovery, narrow execution, independent verification, commit, audit and reconciliation;
- broader domain mutation must not become normal scope before the complete v0.6 lifecycle is proven;
- agent interpretation must remain proposal-only and must not bypass deterministic planning, policy or authorization;
- First Boot/support claims require an explicit platform profile and install/update/recovery evidence;
- high-risk domains such as storage, boot, security posture and virtualization must define domain-specific recovery/verification semantics before mutation support;
- fleet/remote authority must remain optional and must not replace local authority or recovery.

## Anti-drift governance

1. **Released milestone meaning is immutable.** A published version's capability meaning comes from its frozen release contract and evidence. Later roadmap edits may summarize it but may not broaden it.
2. **Future milestones may be rebaselined only explicitly.** A scope change requires a dedicated roadmap change in review with rationale, dependency impact and any necessary ADR or milestone-contract updates.
3. **The machine-readable roadmap contract must change atomically.** `contracts/roadmap.toml` and this document are validated together.
4. **Scope absorption is recorded, not duplicated.** If an earlier milestone legitimately completes work previously expected later, the later milestone is redefined explicitly rather than implementing redundant work to preserve an obsolete label.
5. **Implementation does not self-promote.** Existing code, schemas, binaries or tests do not change the support matrix unless a release contract explicitly claims and qualifies them.
6. **Every active milestone gets a bounded milestone contract.** It must define goal, dependencies, required capabilities, trust invariants, explicit non-goals, evidence and exit criteria before release preparation.
7. **Every release writes the next-version handoff.** The frozen release contract must state what trust boundary can safely come next and what remains prohibited.
8. **Domains do not own release numbers.** Domain sequencing lives in `docs/system-domains.md`; exact release inclusion belongs in milestone/release contracts.
9. **Architecture shortcuts require explicit review.** Any proposal that bypasses typed capability resolution, policy, durable prepare, independent verification or local authority requires an ADR-level decision before implementation.
10. **Evidence wins over aspiration.** If qualification cannot support a planned claim, the claim is reduced or deferred; tests are never weakened to preserve roadmap optics.

## Roadmap-change procedure

A roadmap rebaseline should answer all of the following in the reviewing PR:

- What new evidence or implementation learning makes the change necessary?
- Does it alter any already-published claim? If yes, the change is invalid; published claims remain frozen.
- Which future milestone goals/dependencies move?
- Does any domain sequencing change?
- Does the change introduce or move mutation authority, persistence, agent authority, platform support or remote authority?
- Are milestone, domain, architecture, qualification and machine-readable roadmap documents still consistent?
- Can repository validation detect accidental reintroduction of the superseded roadmap shape?

The purpose of this process is not to prevent learning. It is to ensure that learning becomes an explicit architectural decision rather than silent roadmap drift.
