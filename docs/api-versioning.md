# API and contract stability

Linura treats **contract version** and **contract stability** as independent axes.
A name such as `Control1` or `*.v1.schema.json` identifies a contract generation;
it does not, by itself, make that contract stable or frozen.

`contracts/stability.toml` is the machine-readable source of truth. Public
machine-readable artifacts also embed lifecycle metadata so contributors,
tooling, SDK authors, and automated reviewers see the same status where the
contract is defined.

## Stability levels

### Experimental

Experimental is the default while Linura is pre-1.0 unless a contract is
explicitly promoted.

- Breaking changes are allowed when they improve the architecture or remove an
  obsolete design.
- No overlap window or deprecation shim is required.
- A breaking change must update implementation, checked-in contract, schemas,
  in-repository clients/SDKs, tests, documentation, and the stability registry
  coherently in the same change.
- Compatibility shims must not be retained merely because an earlier
  development commit or Experimental release exposed a shape.
- Security, authority, provenance, validation, resource bounds, and fail-closed
  behavior remain mandatory; Experimental does not mean careless.

### Preview

Preview is for contracts intentionally exposed to named early adopters or design
partners.

- Breaking changes remain possible, but require a migration note and explicit
  release-note disclosure.
- Avoid gratuitous churn and preserve compatibility when inexpensive.
- Promotion is explicit in `contracts/stability.toml`; it is never inferred.

### Stable

Stable is a deliberate compatibility commitment.

- Existing semantics and wire shapes are preserved within the same contract
  major generation.
- Breaking changes require a new major contract/interface, an overlap window,
  migration documentation, and compatibility tests.
- Promotion requires an ADR or equivalent design record, a supported release,
  compatibility coverage, and explicit registry metadata (`since` and
  `compatibility`).
- Stability is never inferred from a `v1` filename, `Control1` suffix, public
  visibility, age, or prior inclusion in an Experimental release.

## Historical enforcement

Stable compatibility is checked against an accepted historical tree, not only
against metadata in the current checkout. The canonical validator selects the
pull-request merge base (or the previous protected-main commit) and enforces
that a Stable contract cannot be removed, downgraded, or rewritten in place
under the same generation.

D-Bus validation permits additive members but preserves every previously
published method, signal, property, interface annotation, argument shape, and
member annotation. JSON Schema, CLI, and Rust SDK contracts currently use a
conservative same-generation comparison: once Stable, their checked contract
artifact is immutable until a typed compatibility checker can prove a change
is backward-compatible.

Protected CI fetches full Git history so the historical comparison cannot
silently degrade into a current-tree-only check. Source archives and specialized
tooling can provide an explicit prior tree with `--baseline-root`; CI or local
Git workflows can override baseline discovery with `--baseline-ref` or
`LINURA_CONTRACT_BASELINE_REF`.

## Product SemVer and contract generations

Product versions and contract generations solve different problems. Linura may
ship `v0.x` releases containing `Control1` and `*.v1.schema.json` contracts that
remain Experimental. A contract may later be promoted independently without
renaming it merely because its stability changed.

## Durable state is different

Experimental wire APIs may be replaced, but persisted user state, migration
records, audit/provenance records, and durable evidence must never be silently
reinterpreted or discarded. Persisted-format changes require explicit versioning,
migration handling, validation, and recovery semantics regardless of API
stability.

## Promotion procedure

1. Identify the exact registry entry in `contracts/stability.toml`.
2. Document real consumers and compatibility requirements.
3. Add migration and compatibility tests appropriate to the target level.
4. Record the promotion rationale in an ADR or release contract.
5. Update registry and artifact-local lifecycle metadata atomically.
6. Run `cargo xtask check` plus applicable integration/acceptance evidence.

Downgrading a Stable contract is not an acceptable substitute for versioning a
breaking change.

## v0.1.0 contract posture

Linura v0.1.0 is Experimental. `org.linura.Control1`, the Rust SDK/CLI surface,
and checked-in JSON Schemas may evolve coherently until explicitly promoted.
The canonical Control1 contract for v0.1.0 is the authenticated read-only
observation surface. Obsolete pre-stable mutation and JSON compatibility stubs
are intentionally not part of it.

## v0.2.0 contract posture

Linura v0.2.0 remains Experimental. The canonical `org.linura.Control1`
generation now adds explicit `PlanDesiredState`, `GetPlanPreview`, and
`ExplainPlanPreview` operations to the authenticated observation/graph surface.
Those methods expose deterministic, evidence-bound plan previews only;
`execution_authorized=false` is part of the boundary and there is no public
`apply` operation or compatibility promise that turns Control1 generation 1
into a Stable API.

The planning JSON contracts and Rust SDK/CLI surface may evolve coherently with
the Experimental Control1 generation. Any future Stable promotion requires the
normal explicit registry/compatibility process; the v0.2.0 product version does
not imply that promotion.

## v0.3.0 contract posture

Linura v0.3.0 remains Experimental. The canonical `org.linura.Control1` generation extends the authenticated observation and non-executable planning surface with explicit plan-review/explanation operations. Reviewed risk, policy outcome, approval requirement, and semantic provenance are explanation/authority evidence only; they do not create an executable token or public mutation path.

The approval lifecycle is intentionally Control-owned and process-local in this release. D-Bus authenticates service callers but does not treat that identity, including UID 0, as trusted human/admin approval. Any future Stable promotion or executable authority requires the normal explicit compatibility process plus the later durable recovery and mutation-lifecycle milestones; product version v0.3.0 itself makes no Stable compatibility promise.
