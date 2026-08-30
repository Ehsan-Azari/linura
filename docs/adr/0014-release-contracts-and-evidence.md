# ADR 0014 — Version-scoped release contracts and machine-readable evidence

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

Linura can change bootability, privileged system state, persistence, recovery behavior and hardware/platform support. Generic changelog bullets and GitHub-generated release notes cannot precisely communicate or prove those claims.

The repository already separates exact-source candidate construction, promotion of the same bytes and independent publication verification. The missing boundary is a version-scoped release claim that is itself bound to those exact bytes.

## Decision

Linura adopts:

1. mutable milestone contracts at `docs/milestones/vX.Y.Z.md`;
2. frozen release contracts at `docs/releases/vX.Y.Z.md`;
3. explicit release claim classes;
4. canonical PR links as default human change provenance;
5. full-SHA commit URLs for exact immutable provenance when materially useful;
6. candidate-generated `RELEASE-EVIDENCE.json` binding exact source, claim metadata, release-note digest, traceability and artifact digests;
7. GitHub Release bodies sourced from the frozen candidate `RELEASE_NOTES.md`, never independently generated;
8. independent post-publication verification of tag/source binding, evidence, checksums, Release-body identity and provenance.

PR/commit traceability is provenance of implementation history, not evidence that the release claim is correct. Exact-source CI/security/acceptance evidence remains necessary.

## Consequences

- SemVer cannot silently imply production support.
- A release declares supported platform/hardware/capability and recovery boundaries explicitly.
- Release-note drift between repository source and GitHub Release is detectable.
- Review history remains navigable through PR links while security-critical provenance can point to immutable commits.
- Release candidates gain an additional deterministic evidence artifact.
- Historical release contracts become part of the release record and must not be silently rewritten after tagging.

## Non-goals

This ADR does not define long-term stable-support policy, hardware certification policy, signing-key governance or a remote release service. Those may require later ADRs as Linura reaches supported/stable claim classes.
