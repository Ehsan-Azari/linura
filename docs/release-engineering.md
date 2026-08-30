# Release engineering

Linura separates **what a version claims**, **which exact reviewed source is proven**, **how candidate bytes are constructed**, **how that proof is promoted**, **when the immutable version tag is created**, and **how publication is independently verified**.

See [Release contracts, claims and evidence](release-contracts.md) for the version-scoped documentation/evidence model.

The release control plane is implemented inside Linura. The mature ProdKit release lifecycle is used as a reference design, but Linura does not require `prodkit-workflows` or another repository at release time.

## Release documentation lifecycle

Every planned version starts with a mutable milestone contract at `docs/milestones/vX.Y.Z.md`.

Before release proof starts, implementation closes into a frozen release contract at `docs/releases/vX.Y.Z.md`. The matching release contract is mandatory input to proof and publication and declares the release claim class, supported platform scope, security/authority boundary, migration/recovery boundaries, known unsupported states and PR/commit traceability.

The GitHub Release body is not independently generated. The exact `RELEASE_NOTES.md` sealed during Trusted Release Proof is published verbatim.

### Release presentation convention

Linura follows one stable presentation contract across Git, GitHub Releases and frozen release notes:

- Git tag: `vX.Y.Z`.
- GitHub Release title: `Linura vX.Y.Z`.
- Frozen release-note first heading: `# vX.Y.Z — <implementation theme>`.

The Git tag deliberately stays product-name-free for SemVer-compatible tooling. The product name belongs in the GitHub Release title, while the frozen note heading carries the version plus a concise implementation theme. GitHub has no separate release subtitle field, so the first Markdown heading is the canonical subtitle-like presentation and is verified as part of the frozen Release body.

## Protected release intent

The normal release path begins with a commit merged into protected `main` whose subject is:

```text
release: vX.Y.Z — <implementation theme>
```

That commit does **not** create a tag. It expresses an unpublished release intent for the exact current `main` SHA.

`Release Proof Dispatch` observes completed `CI`, `Security` and `CodeQL` push runs for `main`. It has `contents: read` and `actions: write`, but no permission to create tags or Releases. For a release-intent source it:

1. requires the triggering SHA to still equal current protected `main`;
2. validates the frozen release contract and workspace version;
3. requires successful exact-SHA `CI`, `Security` and `CodeQL` evidence;
4. refuses a conflicting existing version tag;
5. avoids duplicate proof dispatches;
6. rechecks current `main` immediately before dispatch;
7. dispatches `Trusted Release Proof` at `main`.

If `main` advances before proof dispatch, the stale observer run exits without release authority. A later exact-main gate completion evaluates the new state.

## Trusted Release Proof

`Trusted Release Proof` is `workflow_dispatch`-only and starts with `github.sha` as the candidate source. The proof job has no repository-content write authority. It:

1. proves checkout `HEAD`, `github.sha` and `origin/main` are the same exact SHA;
2. validates the `release: vX.Y.Z — …` subject and frozen release contract;
3. re-verifies successful exact-SHA `CI`, `Security` and `CodeQL` runs;
4. runs canonical `cargo xtask check` and requires canonical validation to leave tracked source unchanged;
5. builds the release binaries once with locked dependencies;
6. constructs `SOURCE_SHA`, `RELEASE_TAG`, frozen `RELEASE_NOTES.md`, SPDX SBOM and `RELEASE-EVIDENCE.json`;
7. seals the payload with `SHA256SUMS` and verifies the complete contract locally;
8. records a machine-readable proof receipt that binds repository, source SHA, tag, version, workflow run and every payload digest;
9. creates GitHub/Sigstore build-provenance attestations for every promotable payload file;
10. uploads one exact-source proof artifact.

The promotable bytes are therefore produced during proof. Later stages consume those same bytes rather than rebuilding them.

After the proof job succeeds, a separate narrow `dispatch-promotion` job receives only `actions: write` and `contents: read`. It rechecks that the proven SHA is still current `main` and explicitly dispatches `Release Promotion` with the exact source SHA and Trusted Release Proof run ID. This explicit `workflow_dispatch` handoff is intentional: GitHub suppresses recursive workflow triggers for many events created by `GITHUB_TOKEN`, while `workflow_dispatch` is a documented exception. The proof job itself retains no tag or Release publication authority.

## Release Promotion

A successful `Trusted Release Proof` explicitly dispatches `Release Promotion`. The legacy `workflow_run` trigger remains a safe redundant signal, but release correctness does not depend on it. Promotion has `contents: read` and `actions: write`; it cannot create a tag or GitHub Release.

Because the explicit dispatch can start while the proof workflow's narrow dispatch job is finishing, Promotion first waits a bounded interval for the referenced proof run to reach a terminal state. It then requires that run to be `completed/success` before any release handoff.

Promotion:

1. verifies the exact proof run identity, event, terminal status, conclusion and source SHA;
2. requires the proven SHA to still be current `main`;
3. validates the version/frozen contract again;
4. refuses a version tag already bound to another source;
5. avoids duplicate active Release runs;
6. rechecks current `main` immediately before handoff;
7. dispatches `Release` on `main` with the exact source SHA, proof run ID and version.

The final Release request itself verifies that its `github.sha` equals the exact promoted source. If `main` changes while the handoff is being resolved, the Release request fails closed rather than silently selecting a different commit.

## Release validation and source commit point

The final `Release` workflow begins read-only. Before any tag/publication authority is used, its validation job:

1. requires the workflow to have been dispatched on `main` at the exact promoted source SHA;
2. requires `origin/main` still to equal that source SHA;
3. validates the release-intent subject, requested version and frozen release contract;
4. re-verifies permanent exact-SHA gates;
5. verifies the exact successful Trusted Release Proof run;
6. downloads the exact proof artifact;
7. verifies the proof receipt and every sealed payload digest;
8. re-runs release-contract and payload verification;
9. verifies build provenance for every payload file;
10. refuses an existing version tag bound to another source.

Successful completion of this validation stage is the release source-selection commit point. At that point one exact reviewed, gated and proven source has entered the serialized `release-<version>` workflow. Later ordinary `main` development does not change the identity of that already-promoted release attempt.

If a correction is required **before** this validation succeeds, merge the correction and let the stale proof/promotion path fail closed. If a correction is discovered after the final Release validation has succeeded, cancel the release before publication or publish a subsequent version; never retarget an immutable version tag.

## Tag-last publication

Only the `publish` job of the final Release workflow has `contents: write`, and it runs through the `release` GitHub Environment.

Publication:

1. checks out the exact selected source again;
2. re-downloads the exact Trusted Release Proof artifact;
3. re-verifies the sealed bytes and their attestations;
4. creates `refs/tags/vX.Y.Z` only if absent, or proves an existing tag points to the same selected source;
5. creates or resumes a draft GitHub Release with title `Linura vX.Y.Z` and the frozen `RELEASE_NOTES.md` body;
6. reconciles the draft asset set to the sealed proof payload;
7. verifies every uploaded asset digest;
8. publishes the draft only after the remote asset set is exact.

Promotion and publication never rebuild the payload. The immutable version tag is therefore a publication result of a successful proof chain, not the trigger that grants proof authority.

## Independent publication verification

Because GitHub suppresses many recursive workflow triggers created by `GITHUB_TOKEN`, Release explicitly dispatches `Verify published release` after publication rather than relying only on the `release.published` event.

Independent verification:

1. resolves and checks out the published tag;
2. downloads published assets afresh;
3. proves the tag commit equals published `SOURCE_SHA`;
4. verifies `RELEASE-EVIDENCE.json`;
5. verifies `SHA256SUMS`;
6. downloads the GitHub Release body and compares it byte-for-byte with published `RELEASE_NOTES.md`;
7. verifies GitHub build provenance for every published payload asset.

Verification is serialized per tag so duplicate publication signals cannot race one another. Publication is incomplete until this independent verification succeeds.

## Traceability policy

Release notes use PR links as the default human change provenance. For security-sensitive, migration, recovery, release-control or trust-boundary claims, add a full-SHA commit URL when it materially improves immutable provenance.

PR/commit references are provenance, not acceptance evidence. The release's required exact-source tests remain authoritative for correctness/support claims.

## Supported release qualification

Supported releases additionally require VM/profile/hardware, upgrade, recovery and privilege-boundary evidence appropriate to the declared claim class and capability scope. Release metadata is never a substitute for system acceptance evidence.

The generic [supported release readiness checklist](operations/release-readiness.md) is applied together with the version-specific frozen release contract.
