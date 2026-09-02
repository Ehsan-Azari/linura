# Changelog

All notable changes to Linura will be documented here. Version entries stay concise; detailed claims and acceptance boundaries live in `docs/releases/`.

## [Unreleased]

### Added
- Canonical `workstation`, `server`, and `edge` machine classes, with developer/AI development machines represented as workstation profiles and fleet/enterprise retained as an optional management overlay.
- Typed `MachineClass` in the intent domain, public SDK exposure, machine-class support/applicability governance, and ADR 0016.
- v0.3.0 milestone and qualification specifications for policy, authorization, approval, and review-only authority, plus ADR 0018 defining the canonical plan-review boundary.
- Typed authenticated-principal and policy revision identities, fail-closed policy outcomes, exact review binding, and Control-owned projection from the canonical `ReconciliationPlan` into policy review.
- Deterministic trusted authority risk classification that treats planner risk as a floor, conservatively elevates the initial typed systemd `active_state` review route, blocks unclassified mutation shapes and risk downgrades, and preserves risk-policy revision/rule provenance in review findings.
- Machine-enforced `authority_state` roadmap gates and an authority-foundation checker that rejects both reintroduction of superseded authority paths and accidental deletion of deliberate future lifecycle/executor scaffolds.

### Changed
- Experimental portable machine profiles now preserve a required `machine_class` through `MachineProfile` and `portable-profile.v1`, enabling future cross-class adoption checks without implying any current platform-support claim.
- Policy review now derives from the canonical non-executable planner lineage and binds the authenticated principal, request/plan, authoritative evidence, provider/resource/capability, semantic provenance, trusted risk classification, and policy revision.
- Removed the superseded Experimental `ActionPlan` / provider-owned planning / generic apply-runtime stack instead of retaining compatibility shims or a competing legacy authority model.
- Canonical `cargo xtask check` and `cargo xtask repo` now run the v0.3 authority-foundation anti-drift validation directly.

### Boundaries
- v0.3 authority remains review-only: policy allow, valid approval, and reviewed-plan status are not execution authority.
- Risk classification is not mutation support: unmatched future provider/domain mutation semantics remain blocked rather than guessed into a weaker approval class.
- No public apply path, durable prepare record, privileged executor grant, managed external mutation, or complete eleven-stage mutation support is introduced by these foundation changes.
- `linura-lifecycle`, authoritative observation, the canonical planner, and narrow executor package scaffolds remain intentionally present for later roadmap milestones; their code presence is not a current support claim.

## [0.2.0] - 2026-09-01

Experimental deterministic desired-state and non-executable planning milestone. Full release contract: [`docs/releases/v0.2.0.md`](docs/releases/v0.2.0.md).

### Added
- Typed declarative capability resource blueprints and deterministic capability resolution/conflict handling.
- Deterministic hand-authored semantic intent/requirements/capability-origin compilation into normalized desired resources.
- Evidence-bound deterministic reconciliation previews with `no-change`, `change-proposed`, and fail-closed `blocked` status.
- Exact authoritative evidence identity, ordered changes/findings, prospective risk, and explicit `execution_authorized=false` in the preview contract.
- Experimental Control1 `PlanDesiredState`, `GetPlanPreview`, and `ExplainPlanPreview` methods with matching checked D-Bus XML, SDK/client methods, CLI commands, and JSON contracts.
- Transport-neutral `linura-control::PlanPreviewControl` as the single orchestration owner for replay checks, authoritative observation, planning, retention, and retained preview lookup/explanation.
- Stable authenticated-principal replay/retention namespaces while preserving the first accepted transport actor as provenance.
- Bounded request decoding and process-local preview retention by entry count, per-entry bytes, and aggregate bytes with deterministic eviction.
- Exact-source `control1-plan-preview` disposable-VM acceptance proving change-proposed, exact retry replay, retained lookup/explanation, no-change, blocked unknown state, idempotency conflict, and unchanged native system state.
- v0.2.0 release qualification requiring both the authoritative-observation regression and Control1 plan-preview VM before release build/promotion.

### Changed
- Moved plan-preview authority orchestration out of `linura-dbus` into `linura-control`; D-Bus now authenticates credentials, adapts typed wire data, and delegates.
- Plan-preview VM path coverage now includes core, graph, control, planning, observation, protocol, SDK, D-Bus, interface, and acceptance/tooling dependencies so semantically relevant changes cannot bypass system qualification.
- Trusted Release Proof now requires both mandatory v0.2.0 VM scenarios before isolated release build and promotion.

### Boundaries
- Plan previews are non-executable and process-local; no public `apply` path exists.
- No policy approval, Polkit authority, durable prepare/commit, managed external mutation, post-effect verification/commit/audit/reconciliation, or complete eleven-stage lifecycle is claimed.
- No supported Linux distribution/profile or hardware tier is declared.
- No natural-language/model interpretation, First Boot, persistent Linura Library, or production readiness is release-qualified.

## [0.1.0] - 2026-08-31

Experimental authoritative-observation milestone. Full release contract: [`docs/releases/v0.1.0.md`](docs/releases/v0.1.0.md).

### Added
- Authenticated session D-Bus `org.linura.Control1` service with transport-derived caller identity.
- Deterministic `linuractl whoami`, `capabilities`, `observe`, `graph`, and evidence-only `explain` surfaces through the public SDK/protocol boundary.
- Provider health/capability discovery with explicit available, degraded, and unavailable states.
- Native read-only systemd and NetworkManager observation with provider/resource identity, authority, freshness, validity, sequence, and typed attributes.
- Projection of authoritative observations into the causal system graph with evidence/explanation linkage.
- Runtime D-Bus introspection lifecycle annotations matching the canonical checked interface contract.
- Historical Stable-contract enforcement against an accepted baseline, while v0.1.0 contracts remain Experimental.
- Repository-owned exact-source disposable-VM qualification using a dated SHA-256-pinned Ubuntu cloud image, ephemeral cloud-init/SSH identity, QEMU snapshot execution, and machine-readable VM evidence.
- Mandatory disposable-VM qualification in Trusted Release Proof before release build/promotion.

### Changed
- Explicit systemd observation resolves installed units through native `LoadUnit` before reading Unit properties, so inactive installed units remain observable after systemd garbage-collects their previous loaded-unit object without starting or rewriting the unit.
- VM acceleration selection now distinguishes `/dev/kvm` presence from usable KVM access; GitHub-hosted qualification uses deterministic TCG and fails immediately if QEMU exits before SSH readiness.
- Renumbered the originally planned `v0.0.1` implementation milestone to `v0.1.0` to follow Linura's pre-1.0 policy: new externally testable capability slices consume a minor version, while patch versions repair an already-published minor line.

### Boundaries
- No managed system mutation is claimed.
- No supported Linux distribution/profile or physical hardware tier is declared.
- No production persistence, migration, First Boot, agent interpretation, Polkit authority, or complete eleven-stage mutation lifecycle is release-qualified.

## [0.0.0] - 2026-08-30

Architecture/bootstrap release. Full release contract: [`docs/releases/v0.0.0.md`](docs/releases/v0.0.0.md).

### Added
- Rust workspace, canonical project layout, root/agent instructions, task-specific skill guides, issue/PR templates, code/security/community policies, and architecture/terminology/state/provider/permission/intent/update/recovery/First Boot/Library/Control Center/agent/packaging/test documentation.
- Shared domain crates for core IDs/reasons, intent/setup/profile objects, causal system graph, capability SDK, planner/policy/lifecycle/provenance/protocol/provider SDK/public SDK/agent runtime/update state, Linura Control, local D-Bus transport, Linux observers, narrow executor scaffolding, update guard, CLI/daemon entry points, First Boot/Control Center/agent UI placeholders, and packaging metadata.
- Versioned JSON schemas and D-Bus XML for intent/setup/profile/plan/audit/machine-profile/public Control1 contracts.
- Canonical eleven-stage mutation lifecycle and failure-aware state machine scaffold.
- Repository quality/security/release tooling including formatting, clippy, tests, docs, SPDX policy, dependency audit, CodeQL, release manifests, SHA-256 asset verification, SBOM generation, trusted release proof, promotion, independent verification, and repository hygiene checks.
- Architecture boundary governance through machine-checked layering rules, ADRs, contract-stability policy, release contracts, milestone/qualification documents, risk register, and threat model.

### Changed
- The originally planned patch-numbered roadmap was rebaselined to pre-1.0 minor releases for new capability slices; released milestone meanings remain immutable and future milestones require explicit rebaseline review.

### Boundaries
- Architecture and scaffolding only; no supported managed mutation, distro/profile, hardware tier, user-facing First Boot/Control Center, natural-language agent interpretation, persistent Library, fleet authority, or production-ready operating environment is claimed.
