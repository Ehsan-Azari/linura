# vX.Y.Z — implementation theme

**Status:** implementation complete; publication evidence is established only by the protected release lifecycle.
**Claim class:** Experimental
**Supported platform profiles:** none

## Outcome

Summarize the bounded result of this version.

## User-visible capability

Describe what a user/operator can actually do that this release claims.

## Implemented scope

Describe the implementation that supports the claim.

## Authority and security boundary

State privilege, identity, policy, secret, fail-closed and trust-boundary effects.

## Platform and hardware scope

Declare exactly which platform profiles/hardware tiers are supported by this claim, or explicitly say none.

## Persistence, migration and upgrade

State schema/migration impact, upgrade source versions, backup requirements and downgrade compatibility.

## Recovery and rollback

State recovery, compensation, snapshot/rollback and break-glass behavior. Explicitly call out unavailable recovery mechanisms.

## Compatibility boundary

State API/protocol/profile compatibility and any breaking changes.

## Required acceptance evidence

List the permanent exact-source evidence required before publication.

## Known limitations and unsupported states

List meaningful unsupported environments, capabilities and failure modes.

## Explicit non-goals

State what this release deliberately does not claim.

## Traceability

Use PR references by default. Add full-SHA commit URLs when exact immutable provenance materially improves a security, migration, recovery, release-control or trust-boundary claim.

- Change summary. [PR #123](https://github.com/linura-org/linura/pull/123)
- Security-sensitive example.
  [PR #124](https://github.com/linura-org/linura/pull/124) ·
  [`0123456`](https://github.com/linura-org/linura/commit/0123456789abcdef0123456789abcdef01234567)

## Artifacts and supply-chain evidence

State required binaries, SBOM, checksums, attestations and any version-specific artifacts.

## Publication evidence

Publication evidence exists only after the exact source passes the protected lifecycle, the tag is bound to that source, the exact candidate bytes are promoted, and independent verification succeeds.

## Next-version handoff

State what the next milestone can build on and what gaps remain intentionally open.
