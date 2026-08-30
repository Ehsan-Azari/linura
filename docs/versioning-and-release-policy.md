# Versioning and public release policy

Linura uses Semantic Versioning as a **consumer contract**, not as a build counter.

A merged commit, successful CI run, nightly build or internal milestone does not automatically deserve a Git tag or GitHub Release. Public versions exist when a concrete audience needs a durable, attributable artifact and the release contract truthfully describes what that audience can rely on.

## Pre-1.0 policy

### `v0.0.0`

`v0.0.0` is a one-time architecture and trustworthy-development-foundation record. It proves the initial control-plane architecture, release contract and supply-chain path. It is not represented as a stable end-user Linux system or production hardware-qualified release.

### `v0.MINOR.0`

Use a new pre-1.0 minor version for a **meaningful externally consumable milestone**, for example:

- a developer preview that another developer can install or integrate;
- an end-to-end capability slice that testers can exercise;
- a materially expanded supported platform/profile contract;
- a deliberate compatibility or protocol boundary change;
- a supported-preview milestone with new acceptance evidence.

Do not increment the minor version merely because an internal work batch finished.

### `v0.MINOR.PATCH`

Use a patch version only to repair an already-published `v0.MINOR.x` line when consumers need the fix and the release does not intentionally redefine that line's capability/support contract.

Examples include a packaging correction, security fix, compatibility fix or release artifact correction for an already-used preview.

Do not create `v0.0.1`, `v0.0.2`, and so on simply to record ordinary development progress.

### Pre-release identifiers

Use `-alpha.N`, `-beta.N` or `-rc.N` only when an external testing cohort needs distinguishable intermediate artifacts before a milestone release. CI snapshots, branch builds and nightlies should normally remain Actions artifacts or another non-release distribution channel.

## `v1.0.0`

`v1.0.0` is the first stable end-user contract. It should not be declared merely because the feature list feels large enough.

The 1.0 release contract must have evidence appropriate to a stable system layer, including the supported hardware/platform profiles it claims, install/bootstrap behavior, privilege boundaries, upgrade and migration paths, recovery/rollback behavior, compatibility policy, and security/support expectations.

After 1.0, normal Semantic Versioning applies: incompatible stable-contract changes require a major version, backwards-compatible capabilities use a minor version, and backwards-compatible fixes use a patch version.

## Release qualification rule

A public release requires all of the following:

1. an identifiable external consumer or archival trust purpose;
2. a frozen version-scoped release contract;
3. claims no broader than the acceptance evidence supports;
4. exact-source permanent gates and configured review;
5. successful Trusted Release Proof and reproducibility qualification;
6. promotion of the exact proven bytes rather than a publication rebuild;
7. immutable GitHub publication and independent verification.

If these conditions are not met, keep developing without consuming a public version number.

## Practical roadmap convention

A plausible sequence is therefore **not** automatically:

```text
v0.0.0 -> v0.0.1 -> v0.0.2 -> ... -> v0.1.0
```

It is instead driven by real milestones, for example:

```text
v0.0.0   architecture / trustworthy-development foundation
v0.1.0   first externally usable developer preview
v0.1.1   only if v0.1.0 users need a compatible fix
v0.2.0   next meaningful capability/support milestone
v0.3.0   another meaningful pre-1.0 milestone
...
v1.0.0   first stable supported end-user contract
```

Some projects may need more or fewer pre-1.0 minors. The version sequence follows the product/support contract, not an arbitrary release cadence.

## Immutable publication policy

All future Linura GitHub Releases are expected to use GitHub's immutable-release protection. Publication follows the draft-first pattern: create the draft, reconcile and verify the complete sealed asset set, then publish once. Independent verification must prove the resulting release is immutable and verify its release attestation and each local asset against that immutable release.

Because GitHub enables release immutability as a repository/organization setting and applies it only to future releases, the setting must be enabled before the first public Linura release is published.
