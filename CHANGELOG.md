# Changelog

All notable changes to Linura will be documented here. Version entries stay concise; detailed claims and acceptance boundaries live in `docs/releases/`.

## [Unreleased]

## [0.0.0] - 2026-08-30

Architecture/bootstrap release. Full release contract: [`docs/releases/v0.0.0.md`](docs/releases/v0.0.0.md).

### Changed
- Renamed the project from the bootstrap working name to **Linura**.
- Promoted Linura from a control-plane-with-agent-client concept to an intent-driven, agent-native Linux umbrella architecture.
- Defined "Tell your computer what you want it to become" as the signature product flow.
- Locked **Linura** as the only product/code namespace; "system control plane" remains architectural terminology.
- Renamed the authority orchestration crate to `linura-control` and retired the generic runtime name.
- Renamed planned application directories to `linura-agent-ui`, `linura-control-center`, and `linura-shell`.
- Locked the canonical managed-mutation lifecycle as request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile.
- Made portable machine-profile exports self-contained by carrying referenced setup and intent definitions.
- Adopted version-scoped milestone/release contracts with explicit claim classes, PR/full-SHA commit traceability, and exact-source acceptance boundaries.
- Release promotion now publishes the frozen repository release contract as the GitHub Release body rather than generating an independent narrative.
- Aligned the release lifecycle to a proof-first/tag-last model with exact-main gate observation, build-once attested payloads, proof-only promotion, final publication authority, and independent verification of publication metadata and evidence.

### Added
- Persistent intent and requirement model.
- Reusable, revisioned `Setup` domain model between individual intents and whole-machine profiles.
- Local-first **Linura Library** architecture for storing/cataloging reusable setups/profiles with optional future sync providers.
- Self-contained portable setup export/adoption protocol with missing-secret-reference reporting.
- Setup nodes in the causal system graph so adopted configurations retain setup provenance.
- Secret-free portability rule: setup/profile exports carry credential references only, never credential values.
- Explicit distinction between portable declarative setups/profiles and exact machine recovery snapshots.
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
- ADR 0012 defining the canonical trustworthy mutation lifecycle.
- ADR 0013 defining reusable setups and the local-first Linura Library.
- ADR 0014 defining version-scoped release contracts and machine-readable release evidence.
- `RELEASE-EVIDENCE.json` generation/verification binding claim metadata, frozen notes, PR/commit traceability and candidate artifact digests.
- v0.0.1 milestone contract for authenticated authoritative read-only observation and the first real observed system graph.

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
