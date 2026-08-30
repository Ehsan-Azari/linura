# Release engineering

Linura separates **what a version claims**, **which exact reviewed source becomes that version**, **how exact candidate bytes are constructed**, **how those bytes are promoted**, and **how publication is independently verified**.

See [Release contracts, claims and evidence](release-contracts.md) for the version-scoped documentation/evidence model.

## Release documentation lifecycle

Every planned version starts with a mutable milestone contract at `docs/milestones/vX.Y.Z.md`.

Before tagging, the implementation closes into a frozen release contract at `docs/releases/vX.Y.Z.md`. The matching release contract is mandatory input to the candidate workflow and declares the release claim class, supported platform scope, security/authority boundary, migration/recovery boundaries, known unsupported states and PR/commit traceability.

The GitHub Release body is not independently generated. Promotion publishes the exact candidate `RELEASE_NOTES.md`, which is copied from the frozen repository release contract.

### Release presentation convention

Linura follows one stable presentation contract across Git, GitHub Releases and frozen release notes:

- Git tag: `vX.Y.Z`.
- GitHub Release title: `Linura vX.Y.Z`.
- Frozen release-note first heading: `# vX.Y.Z — <implementation theme>`.

The Git tag deliberately stays product-name-free for SemVer-compatible tooling. The product name belongs in the GitHub Release title, while the frozen note heading carries the version plus a concise implementation theme. GitHub has no separate release subtitle field, so the first Markdown heading is the canonical subtitle-like presentation and is verified as part of the frozen release body.

## Release-source selection

The irreversible choice of which commit becomes `vX.Y.Z` is explicit rather than delegated to a background race-prone dispatcher.

Before the immutable version tag is created:

1. the release changes and frozen release contract are merged through the protected pull-request path;
2. configured automated review has completed and every blocking review thread has been addressed;
3. the chosen exact `main` SHA has successful `CI`, `Security` and `CodeQL` push runs;
4. the release operator confirms that exact SHA is the intended source for the version;
5. `refs/tags/vX.Y.Z` is created once at that exact SHA.

There is no automatic workflow that chooses between multiple same-version release snapshots or creates the immutable version tag in response to asynchronous gate completions. This deliberately removes cross-ref and supersession races from the release authority boundary.

If a release attempt needs correction before tagging, merge the correction, let the new exact `main` SHA complete review and permanent gates, and tag only that corrected SHA. Older untagged attempts have no publication authority. Once `vX.Y.Z` exists, that version identity is consumed and must never be retargeted.

The release operator must perform the source-selection and tag-binding operation as one deliberate release action: read the current protected `main` SHA after review/gates, decide that SHA is the version source, and create the immutable tag at that exact SHA. If `main` changes before the tag is created, restart the source-selection check rather than tagging a previously observed head implicitly.

Tag creation is therefore a narrow explicit release-authority operation. All subsequent stages are evidence-driven and exact-source bound.

## Candidate

The tag-bound **Trusted release candidate** workflow runs on a `v*` tag push and also exposes `workflow_dispatch` as a recovery/retry path. It:

1. verifies the tag commit belongs to `main` history;
2. requires `docs/releases/<tag>.md`;
3. requires the tag version to equal the workspace version;
4. validates canonical release-contract structure, claim class and PR/commit traceability;
5. runs the canonical `cargo xtask check` gate;
6. builds release binaries with `--locked`;
7. records exact `SOURCE_SHA` and `RELEASE_TAG`;
8. copies the frozen contract to `RELEASE_NOTES.md`;
9. generates an SPDX 2.3 SBOM;
10. generates `RELEASE-EVIDENCE.json` containing claim metadata, release-note digest, extracted PR/full-SHA commit traceability and candidate artifact digests;
11. generates and locally verifies `SHA256SUMS`, including the notes and evidence record;
12. verifies the evidence record against the exact candidate;
13. creates GitHub/Sigstore build-provenance attestations;
14. uploads one immutable candidate artifact set.

`RELEASE-EVIDENCE.json` is an index of the exact claim/artifact set. It does not replace VM, hardware, migration, recovery, security or other acceptance evidence required by the release contract.

## Promotion

A successful **Trusted release candidate** automatically enters the promotion workflow. Manual dispatch remains available as a recovery path and requires the exact candidate run ID, source SHA and tag.

Promotion verifies:

- candidate workflow identity and successful conclusion;
- candidate head SHA;
- checked-out exact source;
- tag-to-source binding;
- embedded `SOURCE_SHA`/`RELEASE_TAG`;
- `RELEASE-EVIDENCE.json`;
- release-note identity/digest;
- `SHA256SUMS`.

It then publishes those **same bytes** to a GitHub Release with title `Linura <tag>` and uses `RELEASE_NOTES.md` verbatim as the Release body. It does not rebuild and does not use generated release notes. An already-existing canonical release is never overwritten; it is preserved and sent to independent verification.

Because GitHub suppresses most recursive workflow triggers created by `GITHUB_TOKEN`, promotion explicitly dispatches the independent verification workflow after publication instead of assuming that a `release.published` event will always create another run.

## Independent publication verification

The post-publication workflow:

1. resolves and checks out the published tag;
2. downloads release assets afresh;
3. proves the tag commit equals published `SOURCE_SHA`;
4. verifies `RELEASE-EVIDENCE.json`;
5. verifies `SHA256SUMS`;
6. downloads the GitHub Release body and compares it with published `RELEASE_NOTES.md`;
7. verifies GitHub build provenance for every published candidate asset.

Verification is serialized per tag so duplicate publication signals cannot race one another. Publication is incomplete until this independent verification succeeds.

## Traceability policy

Release notes use PR links as the default human change provenance. For security-sensitive, migration, recovery, release-control or trust-boundary claims, add a full-SHA commit URL when it materially improves immutable provenance.

PR/commit references are provenance, not acceptance evidence. The release's required exact-source tests remain authoritative for correctness/support claims.

## Supported release qualification

Supported releases additionally require VM/profile/hardware, upgrade, recovery and privilege-boundary evidence appropriate to the declared claim class and capability scope. Release metadata is never a substitute for system acceptance evidence.

The generic [supported release readiness checklist](operations/release-readiness.md) is applied together with the version-specific frozen release contract.
