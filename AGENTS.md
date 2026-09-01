# AGENTS.md

Linura is operating-system authority software. AI agents are useful contributors and runtime interpreters but are never trusted authorities.

## Product invariant

> **Tell your computer what you want it to become.**

Implement that by converting human/model input into typed intent and deterministic desired state. Never implement it by letting a model improvise privileged shell commands.

## Mandatory rules

- Read `SECURITY.md`, `docs/security-model.md`, `docs/agent-architecture.md`, and `docs/vision-coverage.md` before modifying authority, agent, policy, executor, persistence or extension code.
- In a Codex/cloud development environment, run `bash scripts/preflight_codex_environment.sh` before modifying repository files. If it fails, report the environment mismatch rather than silently repairing the task environment.
- `scripts/setup_codex_environment.sh` is environment-creation/bootstrap tooling, not an ordinary task-time installer. Do not install mutable/latest development tools during delegated work; repository-owned Codex tool versions live in `tools/codex/versions.env`.
- Conversation/model output is never the durable source of truth; approved structured intent is.
- Every managed resource must be explainable through semantic provenance.
- Retiring intent must analyze shared ownership/dependencies before cleanup.
- Never introduce arbitrary elevated shell execution.
- Never move general orchestration into a privileged executor.
- Never bypass policy because the caller is local, root-owned, trusted by the user, or an AI agent.
- Agent/model code cannot depend on or receive a privileged executor handle.
- Never pass secrets in process arguments, logs, audit payloads, model prompts/context, panic messages or fixtures.
- Unknown/unsupported state fails closed for mutations.
- Provider/platform dependencies stay out of UI and core domain crates.
- D-Bus objects, Unix file descriptors, sockets, process/session handles and other transport primitives remain adapter details. Providers expose bounded mechanisms; cross-provider query budgets, deadlines, caching/coalescing and aggregation belong to Linura Control.
- Cached observations and retrieval/RAG context do not become authoritative current state merely because they are available; required authoritative facts retain provider/resource/capability/freshness requirements.
- Contract version is not contract stability. Before preserving, removing, or changing a public interface/schema/SDK/CLI surface, read `contracts/stability.toml` and `docs/api-versioning.md`.
- Do not create compatibility shims for Experimental contracts merely because an earlier development commit exposed them; replace the contract coherently and update all in-repo consumers/tests/docs in the same change.
- Stable compatibility obligations exist only for contracts explicitly marked `stable` in `contracts/stability.toml`; Stable breaking changes require a new major generation, overlap/migration documentation, and compatibility evidence.
- Generated UI must use typed constrained surfaces or isolated extensions.
- Preserve an offline/no-model path for deterministic control and recovery.

## Architecture ownership

- `linura-core`: stable IDs, action primitives, semantic reason invariants.
- `linura-intent`: intent/requirements/profile lifecycle.
- `linura-graph`: system causal/dependency/conflict/ownership/evidence projection.
- `linura-capability-sdk`: capability blueprints and relations.
- `linura-planner`: deterministic desired-state derivation.
- `linura-observation`: canonical authoritative observation envelope and freshness primitives.
- `linura-observation-control`: provider-neutral authoritative observation coordination and bounded retained evidence.
- `linura-linux-observation`: concrete Linux observation adapters; transport mechanisms remain internal.
- `linura-provenance`: why-chain lineage.
- `linura-policy`: policy/approval semantics.
- `linura-protocol`: versioned public contracts.
- `linura-provider-sdk`: Linux provider/observer/executor/verifier contracts.
- `linura-agent-runtime`: untrusted intent interpreters/specialists; no authority.
- `linura-control`: Linura Control; unprivileged authority/control-plane and future context-query orchestration.
- `linura-dbus`: local D-Bus transport adapter; does not own planning/authority semantics.
- `linura-sdk`: public non-privileged client/integration façade; must not expose authority/provider/executor internals.
- `executors/*`: minimal privileged effectors only.
- `apps/*`: clients/process entry points; no duplicate domain rules.

## Change procedure

1. Identify the user intent and durable domain object affected.
2. Identify graph/provenance consequences.
3. Identify trust/privilege boundary crossed.
4. Update core/intent/graph/protocol first.
5. Update planner/policy/provider/executor as applicable.
6. Add failure/denial/shared-ownership tests before UI work.
7. Update ADR/RFC for contract or trust-boundary changes.
8. Run the repository quality gate.

## Task-specific guides

Before changing a specialized subsystem, read the corresponding guide under `agents/skills/`:

- architecture;
- intent and system graph;
- protocol/SDK;
- providers;
- privileged executors;
- policy;
- migrations;
- platform profiles;
- VM acceptance;
- shell/UI;
- visual verification;
- security review;
- release engineering.

Use `cargo xtask check` as the canonical completion gate. A missing VM/hardware/visual dependency means that evidence was not run; never report it as passing.
