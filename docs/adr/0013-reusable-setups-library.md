# ADR 0013 — Reusable setups and local-first Linura Library

- Status: Accepted
- Date: 2026-08-30

## Context

Linura's intent-native model should let users preserve a useful configuration and reuse it later on the same machine or another supported machine. A whole-machine profile is too coarse for reusable slices such as Rust development, travel security or PostgreSQL development, while an exact filesystem snapshot is too machine-specific and carries implementation state rather than portable meaning.

The previous portable-profile contract also referenced intent IDs without carrying the definitions required to reconstruct them elsewhere. That is not sufficient for a genuinely portable artifact.

## Decision

Linura defines four distinct concepts:

1. **Intent** — one durable goal/constraint.
2. **Setup** — a versioned reusable composition of intents and other setups.
3. **Machine Profile** — a whole-machine composition of setups, standalone intents and portable constraints.
4. **Snapshot** — an exact machine recovery point, intentionally not a portable setup format.

The **Linura Library** is the storage/catalog abstraction for reusable declarative artifacts. Its baseline is local-first and offline-capable. File export/import, Git-backed catalogs, user-owned sync, hosted sync and enterprise catalogs are optional storage/sync adapters rather than authority dependencies.

Additional invariants:

- setup IDs are typed and setups participate in the causal system graph;
- setup revisions are explicit; historical meaning is not silently overwritten;
- portable setup exports are self-contained bundles carrying setup definitions and required intent definitions;
- portable profile exports carry their referenced setup and intent definitions;
- portable artifacts preserve intent/constraints, not package-manager transactions or command transcripts;
- secret values never appear in setup/profile exports; only secret references/requirements may travel;
- imported/synchronized artifacts are untrusted input and carry no authority grant;
- adoption always performs fresh target observation, capability resolution, desired-state derivation, planning, policy/approval and the canonical mutation lifecycle;
- exact snapshots remain separate recovery artifacts;
- future canonical serialization/content digests/signatures require a later ADR after representation details stabilize.

## Consequences

- Users can save and reuse useful machine configurations without cloning machine-specific state.
- The same setup can be realized differently across supported platform/hardware profiles.
- Setup provenance can remain in the why-chain after adoption.
- "Save my current setup" must derive portable intent from managed causal state rather than serialize every installed package.
- Missing credentials during adoption are resolved locally through secret references.
- Local operation remains complete without any hosted Linura service.
- Storage/sync providers can evolve independently from the authority plane.
