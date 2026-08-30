# Release engineering

Linura separates **what a version claims**, **how exact candidate bytes are constructed**, **how those bytes are promoted**, and **how publication is independently verified**.

See [Release contracts, claims and evidence](release-contracts.md) for the version-scoped documentation/evidence model.

## Release documentation lifecycle

Every planned version starts with a mutable milestone contract at `docs/milestones/vX.Y.Z.md`.

Before tagging, the implementation closes into a frozen release contract at `docs/releases/vX.Y.Z.md`. The matching release contract is mandatory input to the candidate workflow and declares the release claim class, supported platform scope, security/authority boundary, migration/recovery boundaries, known unsupported states and PR/commit traceability.

The GitHub Release body is not independently generated. Promotion publishes the exact candidate `RELEASE_NOTES.md`, which is copied from the frozen repository release contract.

## Candidate

The tag-triggered **Trusted release candidate** workflow:

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

The manual promotion workflow accepts a successful candidate run ID, exact SHA and existing tag. It verifies:

- candidate workflow identity and successful conclusion;
- candidate head SHA;
- checked-out exact source;
- tag-to-source binding;
- embedded `SOURCE_SHA`/`RELEASE_TAG`;
- `RELEASE-EVIDENCE.json`;
- release-note identity/digest;
- `SHA256SUMS`.

It then publishes those **same bytes** to a GitHub Release and uses `RELEASE_NOTES.md` verbatim as the Release body. It does not rebuild and does not use generated release notes.

## Independent publication verification

The post-publication workflow:

1. resolves and checks out the published tag;
2. downloads release assets afresh;
3. proves the tag commit equals published `SOURCE_SHA`;
4. verifies `RELEASE-EVIDENCE.json`;
5. verifies `SHA256SUMS`;
6. downloads the GitHub Release body and compares it with published `RELEASE_NOTES.md`;
7. verifies GitHub build provenance for every published candidate asset.

Publication is incomplete until this independent verification succeeds.

## Traceability policy

Release notes use PR links as the default human change provenance. For security-sensitive, migration, recovery, release-control or trust-boundary claims, add a full-SHA commit URL when it materially improves immutable provenance.

PR/commit references are provenance, not acceptance evidence. The release's required exact-source tests remain authoritative for correctness/support claims.

## Supported release qualification

Supported releases additionally require VM/profile/hardware, upgrade, recovery and privilege-boundary evidence appropriate to the declared claim class and capability scope. Release metadata is never a substitute for system acceptance evidence.

The generic [supported release readiness checklist](operations/release-readiness.md) is applied together with the version-specific frozen release contract.
