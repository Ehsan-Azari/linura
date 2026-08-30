# GitHub repository setup

Apply these settings before accepting implementation PRs.

## Repository

- Default branch: `main`.
- Canonical repository: `linura-org/linura`.
- Canonical organization: `linura-org`.
- Canonical project domain: `linura.org`.
- Enable Issues and private vulnerability reporting.
- Enable the dependency graph, Dependabot alerts and security updates, secret scanning, push protection, and code scanning where available.
- Disable force-push and branch deletion on `main`.
- Prefer squash merge for normal feature PRs; preserve an explicit release/history policy if later changed by ADR.
- Delete merged feature branches automatically unless a documented workflow requires otherwise.

## Main branch ruleset

Target the default branch (`main`) and require:

- a pull request before merge;
- all permanent required status checks to pass;
- resolution of all review conversations;
- dismissal of stale approvals when code changes once approving reviews are required;
- at least one approving review as soon as a second trusted reviewer exists; until then, do not configure an approval count that makes the repository impossible for its maintainer to merge through the protected workflow;
- two approving reviews for security-sensitive or privileged-code changes once a suitable reviewer team exists;
- these exact required status-check contexts, proven by the permanent workflows:
  - `canonical-check`;
  - `dependency-audit`;
  - `analyze`;
- the branch to be up to date before merge for trust-boundary, security, packaging, and release changes;
- linear history unless a documented release process requires otherwise;
- signed commits and tags when the organization signing policy is established;
- force-pushes and branch deletion disabled.

Do not add one-time/bootstrap workflow checks to the ruleset. Required checks must be produced by permanent workflows on both pull requests and the protected branch where appropriate. Keep bypass permissions restricted to deliberate recovery/administration rather than ordinary development.

Protect these paths with CODEOWNERS review once teams exist:

- `executors/**`
- `polkit/**`
- `interfaces/**`
- `SECURITY.md`
- `docs/security-model.md`
- `docs/threat-model.md`
- `.github/workflows/**`

Prefer organization teams as CODEOWNERS once they exist, for example `@linura-org/maintainers` for ordinary ownership and `@linura-org/security` for security-sensitive paths. Do not use the bare organization handle as a CODEOWNER.

## Actions

- Set workflow permissions to read-only by default; grant writes per job only when the job requires them.
- Do not allow unreviewed forks to obtain repository or environment secrets.
- Keep every third-party action pinned to an immutable full commit SHA. Repository validation fails if a workflow introduces a floating action ref.
- Create an environment named `release` before publishing supported artifacts; require reviewer approval for release promotion when a second trusted reviewer exists, and otherwise use a deliberate maintainer-controlled release gate that does not expose credentials to ordinary CI.
- Keep release credentials scoped to the `release` environment and unavailable to ordinary CI or pull-request jobs.

## Security features

Enable, where the GitHub plan supports them:

- dependency graph;
- Dependabot alerts;
- Dependabot security updates;
- secret scanning;
- push protection;
- code scanning / CodeQL;
- private vulnerability reporting.

Treat findings as gates for supported releases. Do not weaken a failing security gate merely to produce a release.

## Releases

Before the first supported release require:

- trusted candidate proof on the exact commit SHA;
- SBOM generation;
- checksums;
- artifact signing or provenance attestation;
- independent published-asset verification;
- immutable release-tag policy;
- rollback/recovery acceptance testing;
- release-environment approval or an equivalent deliberate maintainer-controlled gate until a second trusted reviewer exists.

The repository already contains workflows and tooling for candidate construction, promotion, and independent verification. A registry or package publication must not begin until product naming/trademark clearance and the corresponding registry ownership strategy are settled.
