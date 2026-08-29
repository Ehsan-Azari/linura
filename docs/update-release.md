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

## Release proof

See [Release engineering](release-engineering.md). Linura builds candidate bytes from an exact tagged source SHA, records the source/tag, generates an SPDX SBOM and checksums, creates GitHub/Sigstore provenance, and uploads one candidate artifact set. A separate workflow verifies and promotes those exact bytes. A third workflow redownloads published assets and independently verifies checksums/provenance.

Publishing never counts as supported system evidence by itself. Supported releases also require the VM/profile/hardware proof appropriate to their claims.
