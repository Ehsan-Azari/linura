# Release task guide

Linura builds candidate bytes once, proves them, then promotes those same bytes.

- Tag exact verified source.
- Candidate workflow must pass canonical checks and produce SOURCE_SHA, RELEASE_TAG, SPDX SBOM, SHA256SUMS, and provenance.
- Promotion must verify the successful candidate run ID and exact SHA; do not rebuild.
- Post-publication verification redownloads assets and validates checksums/provenance.
- Supported release claims additionally require system/profile evidence appropriate to the release.
