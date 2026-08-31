# Changelog

All notable changes to Linura will be documented here. Version entries stay concise; detailed claims and acceptance boundaries live in `docs/releases/`.

## [Unreleased]

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
- First implementation milestone contract for authenticated authoritative read-only observation and the first real observed system graph (completed as v0.1.0).

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
