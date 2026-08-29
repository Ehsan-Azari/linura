# GitHub repository setup

After creating the GitHub repository, apply these settings before accepting implementation PRs.

## Repository

- Default branch: `main`.
- Default bootstrap target is `Ehsan-Azari/linura`; change repository metadata only if you choose a different owner.
- Enable Issues and private vulnerability reporting.
- Enable dependency graph, Dependabot alerts/updates, secret scanning, push protection, and CodeQL/default setup where available.
- Disable force-push and branch deletion on `main`.
- Prefer squash merge for normal feature PRs; preserve explicit release/history policy if later changed by ADR.

## Main branch ruleset

Require:
- pull request before merge;
- at least one approving review (two for security-sensitive/privileged code once a team exists);
- dismissal of stale approvals when code changes;
- conversation resolution;
- required checks: `CI / rust`, `Security / dependency-audit`, and `CodeQL / analyze` once workflow names are confirmed;
- branch up to date before merge for trust-boundary/release changes;
- signed commits/tags if the organization policy supports them;
- linear history unless a documented release process needs otherwise.

Protect these paths with CODEOWNERS review once teams exist:
- `executors/**`
- `polkit/**`
- `interfaces/**`
- `SECURITY.md`
- `docs/security-model.md`
- `docs/threat-model.md`
- release workflows.

## Actions

- Set workflow permissions to read-only by default; grant writes per job only.
- Do not allow unreviewed forks to obtain secrets.
- Pin third-party actions to immutable commit SHAs before the first supported release. Version tags in the bootstrap workflows are temporary development conveniences.
- Add an environment named `release` before publishing supported artifacts; require reviewer approval for release promotion.

## Releases

Before the first supported release add:
- trusted candidate proof on exact commit SHA;
- SBOM generation;
- checksums;
- artifact signing/attestation;
- published-asset verification;
- immutable release tag policy;
- rollback/recovery acceptance test.
