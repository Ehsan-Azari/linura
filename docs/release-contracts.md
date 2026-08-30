# Release contracts, claims, and evidence

Linura treats release notes as a **version-scoped claim contract**, not as generated prose after publication. A release must say exactly what it claims, what it does not claim, where each material change came from, and which exact-source evidence supports publication.

## Artifact lifecycle

```text
roadmap
  ↓
docs/milestones/vX.Y.Z.md
  mutable development / exit contract
  ↓
implementation + permanent tests
  ↓
docs/releases/vX.Y.Z.md
  frozen human-readable release claim
  ↓
exact tagged source
  ↓
RELEASE_NOTES.md + RELEASE-EVIDENCE.json
  ↓
verified candidate bytes
  ↓
promotion without rebuild
  ↓
GitHub Release body = frozen RELEASE_NOTES.md
  ↓
independent publication verification
```

`CHANGELOG.md` is intentionally concise. It summarizes versions and points readers to the full release contract rather than duplicating validation, compatibility and support evidence.

## Milestone contracts

`docs/milestones/vX.Y.Z.md` is the mutable engineering contract for a planned version. It defines:

- target claim class;
- goal and required capabilities;
- trust-boundary invariants;
- implementation scope;
- required permanent evidence;
- explicit non-goals;
- exit criteria;
- release handoff requirements.

A milestone can change while the work is in progress. It is not publication evidence.

## Frozen release contracts

`docs/releases/vX.Y.Z.md` is the human-readable claim for one exact version. Before a release tag is created, the document must be frozen in the tagged source and contain these headings in this order:

1. `Outcome`
2. `User-visible capability`
3. `Implemented scope`
4. `Authority and security boundary`
5. `Platform and hardware scope`
6. `Persistence, migration and upgrade`
7. `Recovery and rollback`
8. `Compatibility boundary`
9. `Required acceptance evidence`
10. `Known limitations and unsupported states`
11. `Explicit non-goals`
12. `Traceability`
13. `Artifacts and supply-chain evidence`
14. `Publication evidence`
15. `Next-version handoff`

The candidate workflow validates this structure. A release cannot rely on ad-hoc GitHub-generated notes as its authoritative narrative.

## Claim classes

Every release declares exactly one claim class:

| Claim class | Meaning |
| --- | --- |
| `Architecture` | Contracts, architecture and repository machinery only. No runtime/system support claim. |
| `Experimental` | Implemented behavior for development/testing. Not a supported host or production claim. |
| `Developer Preview` | End-to-end capability usable by developers on explicitly bounded profiles, with known limitations. |
| `Supported Preview` | Supportable pre-stable behavior on explicitly published platform/hardware boundaries. |
| `Supported` | Production-supported within published compatibility, upgrade, recovery and hardware boundaries. |
| `Stable` | Supported with mature compatibility/upgrade policy and release qualification appropriate to the declared stable surface. |

SemVer alone never upgrades the claim class. A `v1.0.0` tag cannot manufacture support evidence that does not exist.

## Human traceability

Release notes should make material changes easy to trace without turning the document into a commit dump.

### PR references are the default

User-visible, architectural, protocol, persistence, security, compatibility and release-control changes should normally cite the pull request that introduced or closed the work:

```md
- Added authenticated systemd observation through the local control plane.
  [PR #23](https://github.com/linura-org/linura/pull/23)
```

If several PRs form one capability, cite the relevant PRs once at the end of the paragraph or capability section rather than repeating them on every sentence.

### Full commit references are selective

Use a full 40-character commit URL when immutable source identity materially improves the claim, especially for:

- trust-boundary or privileged-executor changes;
- migration/recovery changes;
- release-control changes;
- security-sensitive fixes;
- architecture decisions that predate or span a normal PR boundary;
- exact historical source assertions.

Example:

```md
- Locked the canonical managed-mutation lifecycle.
  [`e58a8321313dde8fd869dd32a63c026a940a2bfe`](https://github.com/linura-org/linura/commit/e58a8321313dde8fd869dd32a63c026a940a2bfe)
```

Short SHAs may be used as visible labels, but the URL and machine evidence use the full SHA.

Mechanical formatting, generated-file churn and unrelated cleanup normally do not need individual release-note references.

**A PR or commit reference proves provenance, not correctness.** A merged PR does not replace CI, security analysis, VM acceptance, recovery tests, hardware evidence or release verification.

## Machine-readable release evidence

The candidate workflow generates `RELEASE-EVIDENCE.json` from the frozen release contract and exact candidate artifacts. The schema is [`schemas/release-evidence.v1.schema.json`](../schemas/release-evidence.v1.schema.json).

The evidence index records at least:

- evidence schema version;
- release tag and semantic version;
- exact source SHA;
- claim class;
- declared supported platform profiles;
- digest of `RELEASE_NOTES.md`;
- PR numbers and full commit SHAs referenced by the release contract;
- exact candidate artifact names and SHA-256 digests.

The evidence index is generated from trusted release inputs. It is not manually edited publication metadata and does not become runtime authority.

`RELEASE-EVIDENCE.json` intentionally does not digest itself or `SHA256SUMS`; `SHA256SUMS` instead covers the completed evidence file and the frozen notes, avoiding a circular digest graph.

## Candidate invariant

A candidate is acceptable only when:

- `docs/releases/<tag>.md` exists in the exact tagged source;
- its version matches the workspace version;
- its claim class and required sections validate;
- it contains material human traceability through canonical PR and/or full commit links;
- the exact notes are copied to `RELEASE_NOTES.md`;
- `RELEASE-EVIDENCE.json` is generated from those notes and the exact candidate artifacts;
- notes/evidence are included in `SHA256SUMS`;
- canonical CI and release verification pass before artifact upload;
- build provenance is produced for the completed candidate artifact set.

## Promotion invariant

Promotion takes a successful candidate run ID, exact SHA and immutable tag. It:

1. verifies candidate workflow identity and success;
2. verifies tag → exact source SHA;
3. downloads the exact candidate artifact set;
4. revalidates `RELEASE-EVIDENCE.json` and `SHA256SUMS`;
5. publishes those same bytes without rebuild;
6. uses `RELEASE_NOTES.md` verbatim as the GitHub Release body.

Generated GitHub notes may be useful during development, but they are never the authoritative published release claim.

## Independent publication verification

Post-publication verification must independently download the published assets and prove:

- tag, `SOURCE_SHA` and `RELEASE_TAG` agree;
- release evidence validates against published files;
- checksums match;
- the GitHub Release body matches published `RELEASE_NOTES.md`;
- build provenance verifies for every published candidate artifact.

Publication is therefore a verifiable statement about exact source, exact bytes and exact claims.

## Linura-specific support evidence

For claim classes above `Experimental`, the release contract must explicitly bound the system-level evidence relevant to its claims. Depending on the version, this can include:

- supported platform profiles and subsystem versions;
- physical/virtual hardware evidence;
- boot/reboot/suspend/resume behavior;
- upgrade from the previous supported version;
- downgrade compatibility or explicit non-support;
- interrupted update/migration recovery;
- break-glass recovery;
- snapshot/rollback behavior where applicable;
- privileged action classes and policy/approval coverage;
- all eleven mutation lifecycle stages for any capability claimed as managed mutation;
- unsupported states that fail closed.

A release can publish useful architecture or experimental artifacts without satisfying production support criteria, but its claim class and notes must make that boundary impossible to confuse.
