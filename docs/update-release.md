# Update and release model

Linura treats update coordination and release proof as recoverability/security boundaries.

## Coordinated host update

The canonical state progression is:

```text
acquire lock → preflight → disk-space gate → snapshot → package transaction
→ migrations → reconciliation → restart assessment → verification → complete
```

Any stage may transition to `recovery-required` with an attributable reason. Updates should inhibit suspend while a critical transaction is active and require enough free space for package/cache/snapshot safety.

`linura-update` implements the state/policy foundation. The initial Arch profile includes an ALPM pre-transaction guard so direct upgrades cannot silently bypass Linura's snapshot/migration/verification path. `LINURA_UPDATE_CONTEXT=1` identifies coordinator-owned transactions. `LINURA_ALLOW_DIRECT_PACMAN=1` is an explicit break-glass recovery override, not a normal update mechanism.

Native package-manager recovery must remain possible even when `linurad`, the shell, or a model provider is unavailable.

## Release claim and proof

See [Release contracts, claims and evidence](release-contracts.md) and [Release engineering](release-engineering.md).

Each version has a mutable milestone contract while work is underway and a frozen version-scoped release contract before tagging. Release contracts declare claim class, supported platform/hardware scope, authority/security changes, migration/upgrade/recovery boundaries, known unsupported states, and PR/full-SHA commit traceability where appropriate.

Linura builds candidate bytes from an exact tagged source SHA, binds them to the frozen `RELEASE_NOTES.md`, generates a machine-readable `RELEASE-EVIDENCE.json`, SPDX SBOM and checksums, creates GitHub/Sigstore provenance, and uploads one candidate artifact set. A separate workflow verifies and promotes those exact bytes and notes. A third workflow redownloads published assets and independently verifies checksums, provenance, evidence metadata, tag/source identity, and GitHub Release-body identity.

Publishing never counts as supported system evidence by itself. Supported releases also require the VM/profile/hardware, upgrade and recovery proof appropriate to their declared claim class and capabilities.
