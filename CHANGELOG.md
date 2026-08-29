# Changelog

All notable changes to Linura will be documented here.

## [Unreleased]

### Changed
- Renamed the project from the bootstrap working name to **Linura**.
- Promoted Linura from a control-plane-with-agent-client concept to an intent-driven, agent-native Linux umbrella architecture.
- Defined "Tell your computer what you want it to become" as the signature product flow.
- Locked **Linura** as the only product/code namespace; "system control plane" remains architectural terminology.
- Renamed the authority orchestration crate to `linura-control` and retired the generic runtime name.
- Renamed planned application directories to `linura-agent-ui`, `linura-control-center`, and `linura-shell`.
- Made authoritative observation an explicit input to provider planning.
- Split effect execution from independent verification in the provider SDK.

### Added
- Persistent intent and requirement model.
- Full causal system graph with dependency/conflict/shared-ownership relations.
- Capability blueprint/composition and deterministic resolution contracts.
- Semantic provenance/why-chain distinct from mutation audit.
- Deterministic planner boundary for intent → desired state.
- Provider-neutral agent runtime and specialist-role contracts.
- First-boot application bootstrap and offline/no-model requirements.
- Portable machine profile/export/replay architecture.
- Safe intent retirement/removal-impact model.
- Declarative workflow and constrained derived-UI surface architecture.
- Expanded schemas, ADRs, development plan, backlog and vision coverage matrix.
- Public non-privileged `linura-sdk` façade; `linuractl` now consumes the SDK rather than internal protocol crates directly.
- Naming/product architecture documentation and ADR 0011.
- Canonical eleven-stage managed-mutation lifecycle: request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile.
- Mutation lifecycle state machine, correlated stage receipts and injectable runtime ports for later concrete approval, persistence, execution, verification, audit and reconciliation implementations.
- ADR 0012 locking the trustworthy mutation lifecycle and its stage invariants.

## [0.0.0] - Unreleased
- Initial control-plane architecture bootstrap.

### Grand development-foundation update

- added canonical Rust `xtask` developer/CI orchestration;
- added checkpointed bootstrap, migration, update, config-ownership, hardware-evidence, lifecycle, and testkit crates;
- added task-specific contributor/agent guides;
- added disposable QEMU/KVM and SSH acceptance harnesses plus versioned scenarios;
- added Arch image profile and secure supported-install policy;
- added coordinated-upgrade guard and explicit native break-glass semantics;
- added hardware fixtures/support matrix and visual-regression contracts;
- added exact-SHA release candidates, SPDX SBOM, checksums, GitHub/Sigstore provenance, candidate promotion, and post-publication verification;
- pinned GitHub Actions to immutable commits and expanded repository invariants.
