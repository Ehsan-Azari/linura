# Architecture task guide

Use this guide for changes that cross Linura subsystem boundaries.

1. Read `docs/architecture.md`, `docs/control-plane.md`, and relevant ADRs.
2. Preserve the layers: experience → intelligence → authority → graph/capabilities → providers/executors.
3. Agents and UI clients never gain direct executor authority.
4. Prefer new typed domain concepts over shell-output parsing in core code.
5. Add or update an ADR when changing a long-lived boundary, trust assumption, persistence invariant, or protocol compatibility rule.
6. Run `cargo xtask check`.

A refactor is incomplete if docs, schemas, tests, and repository invariants disagree with code.
