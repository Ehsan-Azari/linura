# Supported release readiness checklist

A version may not be called production-supported until all applicable items are proven. This generic checklist supplements, but never replaces, the version-specific frozen release contract under `docs/releases/`.

## Release claim
- [ ] Matching `docs/milestones/vX.Y.Z.md` exit criteria are closed.
- [ ] Frozen `docs/releases/vX.Y.Z.md` exists before tagging.
- [ ] Claim class is no stronger than the evidence justifies.
- [ ] Supported platform profiles/hardware tiers are explicitly declared.
- [ ] User-visible capability and known unsupported states are explicit.
- [ ] Release-note change provenance contains canonical PR links and full-SHA commit links where exact immutable provenance is material.
- [ ] PR/commit provenance is not being used as a substitute for correctness/acceptance evidence.

## Architecture/security
- [ ] No unresolved high/critical threat-model findings.
- [ ] Privileged executor APIs reviewed independently.
- [ ] Policy denial/approval tests cover every privileged action class.
- [ ] Secret-handling review complete.
- [ ] Extension isolation proven or extensions excluded from release.
- [ ] Authority/security changes are stated in the frozen release contract.

## Quality
- [ ] Unit/provider/integration/VM acceptance suites green on exact candidate SHA.
- [ ] Upgrade from previous applicable supported release tested.
- [ ] Interrupted update/migration recovery tested.
- [ ] Reboot/suspend/resume relevant scenarios tested.
- [ ] Hardware support matrix published where the claim includes physical hardware.
- [ ] Version-specific negative/failure evidence in the release contract is satisfied.

## Compatibility and recovery
- [ ] Persistence/schema migration impact is documented.
- [ ] Upgrade source versions are explicit.
- [ ] Downgrade compatibility or non-support is explicit.
- [ ] Snapshot/rollback/compensation behavior is explicit.
- [ ] Native break-glass recovery documented/tested where applicable.

## Supply chain/release
- [ ] Dependencies locked/audited.
- [ ] Third-party CI actions pinned immutably.
- [ ] Frozen release contract validates against the release tag/workspace version.
- [ ] `RELEASE_NOTES.md` is copied from that exact source contract.
- [ ] `RELEASE-EVIDENCE.json` generated and verified.
- [ ] SPDX SBOM generated.
- [ ] Checksums include release notes/evidence and are verified.
- [ ] Artifacts signed/attested.
- [ ] Promotion publishes the exact candidate bytes and frozen notes without rebuilding.
- [ ] Published release body matches `RELEASE_NOTES.md`.
- [ ] Published release assets independently reverified.
- [ ] Release tag is immutable and bound to verified candidate source.

## Operations
- [ ] Diagnostic bundle redaction verified.
- [ ] Audit export/retention behavior documented where applicable.
- [ ] Known limitations and unsupported states published.
- [ ] Operational monitoring/rollback ownership for the declared support class is clear.
