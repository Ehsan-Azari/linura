# Release task guide

Linura treats a release as a bounded claim plus exact-source evidence. Build candidate bytes once, prove them, then promote those same bytes and frozen notes.

- Start each version with `docs/milestones/vX.Y.Z.md`; close it before tagging.
- Freeze `docs/releases/vX.Y.Z.md` with claim class, scope, security/authority boundary, migration/recovery compatibility, limitations and explicit non-goals.
- Use canonical PR links for change provenance. Add full 40-character commit URLs when exact immutable provenance matters for security, migration, recovery, release control or trust-boundary changes.
- Never treat a PR/commit link as correctness evidence; exact-source tests and acceptance evidence remain required.
- Tag exact verified source only after the release contract exists and workspace/tag versions agree.
- Candidate workflow must produce `SOURCE_SHA`, `RELEASE_TAG`, frozen `RELEASE_NOTES.md`, `RELEASE-EVIDENCE.json`, SPDX SBOM, `SHA256SUMS`, and provenance.
- Promotion must verify the successful candidate run ID, exact SHA, tag, evidence and checksums; do not rebuild.
- GitHub Release notes come from `RELEASE_NOTES.md`; do not generate an independent release narrative.
- Post-publication verification redownloads assets and validates tag/source binding, evidence, checksums, Release-body identity and provenance.
- Supported release claims additionally require system/profile/hardware/upgrade/recovery evidence appropriate to the declared claim class.
