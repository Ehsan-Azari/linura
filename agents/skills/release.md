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
- A successful independent Release Verification triggers protected post-release closure. Closure generates one deterministic bookkeeping commit on a release-scoped automation branch, opens a PR, dispatches the canonical `canonical-check`, `dependency-audit`, and `analyze` checks on that exact PR SHA, squash-merges only through the normal `main` ruleset, dispatches the same checks on the exact resulting `main` SHA, and only then deletes obsolete release-scoped temporary/work branches that are not used by an open PR.
- Post-release closure may advance roadmap/current-next status and terminal evidence only. It must never mutate the frozen release contract, immutable release tag, published release body, or published assets.
- If GitHub Actions is not permitted to create or merge pull requests, closure must fail visibly and leave the verified release intact; do not bypass protected `main` or silently downgrade required checks.
- Supported release claims additionally require system/profile/hardware/upgrade/recovery evidence appropriate to the declared claim class.
