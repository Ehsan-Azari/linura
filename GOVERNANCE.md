# Governance

Linura uses a maintainer-led model during pre-1.0 development.

## Decision hierarchy

1. Security invariants and accepted ADRs.
2. Published protocol/platform compatibility guarantees.
3. Maintainer decisions recorded in issues/PRs/RFCs.
4. Implementation convenience.

No implementation shortcut may override a security invariant.

## Architectural decisions

Use ADRs for durable local decisions and RFCs for changes requiring broader design discussion. Accepted ADRs are immutable; supersede them with a new ADR rather than rewriting history.

## Releases

A release is a promotion of a verified commit, not a build performed from an arbitrary tag. Release requirements are documented in `docs/update-release.md`.
