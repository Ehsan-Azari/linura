# Release engineering

Linura separates candidate construction from publication.

## Candidate

The tag-triggered **Trusted release candidate** workflow:

1. verifies the tag commit belongs to `main` history;
2. runs the canonical `cargo xtask check` gate;
3. builds release binaries with `--locked`;
4. records exact `SOURCE_SHA` and `RELEASE_TAG`;
5. generates an SPDX 2.3 SBOM;
6. generates and locally verifies `SHA256SUMS`;
7. creates GitHub/Sigstore build-provenance attestations;
8. uploads one immutable candidate artifact set.

## Promotion

The manual promotion workflow accepts a successful candidate run ID, exact SHA, and existing tag. It verifies the candidate workflow identity, conclusion, source SHA, tag target, and checksums. It then publishes those **same bytes** to a GitHub Release. It does not rebuild.

## Independent publication verification

The post-publication workflow downloads release assets afresh, verifies `SHA256SUMS`, and verifies GitHub build provenance for executable assets.

Supported releases additionally require VM/profile/hardware evidence appropriate to the claim. Release metadata is not a substitute for system acceptance evidence.
