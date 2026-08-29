# Linura

> **Linura — The intelligent system layer for Linux.**
>
> **Tell your computer what you want it to become.**

**Linura is an intent-driven, agent-native Linux system that turns human goals into declarative, policy-controlled, verified machine state.**

Status: architecture + executable bootstrap (`v0.0.0`). Linura is not yet production-ready; this repository defines and begins implementing the boundaries required to reach production readiness without giving an AI unrestricted authority over the machine.

## The product idea

A fresh Linura installation should be able to begin with a minimal, recoverable base and ask:

```text
┌──────────────────────────────────────────────┐
│                                              │
│     What do you want this computer           │
│              to become?                      │
│                                              │
│   > A minimal workstation for Rust and _     │
│                                              │
└──────────────────────────────────────────────┘
```

The answer is **not** converted into arbitrary shell commands. Linura converts it into durable structured intent, resolves capabilities and conflicts, derives desired state, shows a deterministic plan, applies policy and approvals, executes through narrow trusted providers/executors, independently verifies the result, and records why the resulting state exists.

```text
Human intent / automation / imported profile
                    │
                    ▼
          Intelligence plane
 intent → requirements → capability resolution
                    │
                    ▼
             Desired state
                    │
                    ▼
          Authority/control plane
 diff → plan → policy → approval → execution
                    │
                    ▼
       verification → provenance → audit
                    │
                    ▼
                  Linux
```

**Agents propose. Linura decides and executes.** Agent-native never means agent-dependent: CLI, Control Center, recovery, policy evaluation, state inspection, and deterministic execution must remain usable offline with no model provider.

## Two core ideas, one architecture

Linura deliberately combines two ideas in one repository while keeping their trust boundaries separate:

1. **Authority/control plane** — typed Linux model, providers, policy, plan-before-apply, narrow privilege, verification, compensation, reconciliation, audit.
2. **Intent-native system** — persistent user intent, system graph, capability composition, dependency/conflict solver, semantic provenance, specialist agents, first-boot agent UX, portable machine profiles, derived workflows and UI surfaces.

The control plane is reusable without AI. The intelligence plane can be replaced without changing the authority plane.

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│ EXPERIENCE                                                  │
│ First Boot │ Agent UI │ Control Center │ Shell │ CLI        │
├─────────────────────────────────────────────────────────────┤
│ INTELLIGENCE                                                │
│ Intent │ Context │ Agent Providers │ Specialists │ Planner  │
├─────────────────────────────────────────────────────────────┤
│ AUTHORITY                                                   │
│ Desired State │ Diff │ Policy │ Actions │ Verify │ Audit    │
├─────────────────────────────────────────────────────────────┤
│ SYSTEM GRAPH                                                │
│ Resources │ Dependencies │ Conflicts │ Ownership │ Why      │
├─────────────────────────────────────────────────────────────┤
│ CAPABILITIES                                                │
│ Blueprints │ Composition │ Workflows │ Derived Surfaces     │
├─────────────────────────────────────────────────────────────┤
│ PROVIDERS + NARROW PRIVILEGED EXECUTORS                     │
│ systemd │ NetworkManager │ BlueZ │ PipeWire │ UDisks │ ... │
├─────────────────────────────────────────────────────────────┤
│ LINUX                                                       │
└─────────────────────────────────────────────────────────────┘
```

## Non-negotiable invariants

- `linurad` runs unprivileged.
- No generic privileged shell execution API exists.
- Agents receive no privileged executor handle and never inherit the user's authority implicitly.
- Natural language produces an **IntentProposal**, never executable text.
- Conversation is input; approved structured intent and desired state are the durable source of truth.
- Managed state retains semantic provenance: **why it exists**, not only who mutated it.
- Removing an intent runs dependency/shared-ownership analysis before removing derived resources.
- Unknown/unsupported state fails closed for mutations.
- Every mutation is planned, policy-evaluated, verified, auditable, and compensatable where possible.
- UI contains no distro-specific backend knowledge.
- Generated/derived UI is constrained to typed resources/actions or isolated extensions.
- Local deterministic operation and recovery work without network/model access.

## Repository layout

```text
apps/
  linurad/                     unprivileged authority/control service
  linuractl/                   deterministic CLI
  linura-firstboot/            signature "what should this become?" flow
  linura-control-center/       planned typed GUI client
  linura-agent-ui/             planned conversational Linura Agent client
  linura-shell/                planned desktop shell
crates/
  linura-core/                 IDs, actions, semantic reasons, invariants
  linura-intent/               persistent intents, requirements, machine profiles
  linura-graph/                full system graph + removal/shared ownership analysis
  linura-capability-sdk/       composable capability blueprints and resolution
  linura-planner/              intent/capabilities → desired-state planning
  linura-provenance/           semantic "why" chain
  linura-agent-runtime/        provider-neutral interpreters + specialist roles
  linura-policy/               policy/approval decisions
  linura-protocol/             versioned public contract
  linura-provider-sdk/         Linux provider/executor contracts
  linura-sdk/                  public non-privileged developer API facade
  linura-control/              unprivileged authority orchestration
capabilities/                  declarative capability blueprint examples
workflows/                     composable workflow definitions
surfaces/                      constrained derived UI definitions
agents/                        agent provider/specialist contracts and manifests
executors/                     narrow privileged effectors
interfaces/                    local D-Bus contracts
schemas/                       machine-readable contracts
profiles/                      platform and portable machine profiles
bootstrap/                     installer/first-boot/recovery architecture
packaging/                     system integration assets
docs/                          product, architecture, security, ADRs, operations
```

## Product and namespace naming

**Linura** is the umbrella brand and code namespace. **Linura OS** is reserved for the installable distribution. **Linura Control**, **Linura Agent**, **Linura Shell**, **Linura Control Center**, **Linura First Boot**, and **Linura SDK** are product surfaces/subsystems under that umbrella. “System control plane” and “authority plane” remain architectural terms, not separate brands.

The name is inspired by **Linux + aura**: Linux underneath, with a coherent, intelligent and beautiful layer around it. See [`docs/naming.md`](docs/naming.md).

## First platform profile

The first supported target stays deliberately narrow: Arch Linux + systemd + Wayland/Hyprland + NetworkManager + PipeWire/WirePlumber + BlueZ + UDisks2 + Polkit + Btrfs/Snapper. This is a **platform profile**, not an architectural dependency of the core model.

## Development order

We will prove the entire model with a narrow vertical slice before building a broad desktop:

1. stable vocabulary: intent → graph → capability → desired state → plan → action → provenance;
2. read-only observations and system graph;
3. deterministic intent/capability planning without an LLM;
4. one plan-only mutation;
5. one narrow privileged executor + Polkit;
6. execute → verify → provenance → audit;
7. persist intents/graph/desired state and support explain/removal impact;
8. agent interpretation to `IntentProposal` only;
9. first-boot experience;
10. expand system domains, Control Center, shell, workflows, derived surfaces, and enterprise/fleet.

See [`docs/development-plan.md`](docs/development-plan.md) and [`docs/vision-coverage.md`](docs/vision-coverage.md).

## Bootstrap quality gate

Rust `1.98.0` is pinned.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
python3 scripts/check_repository.py
```

The bootstrap deliberately keeps Rust crates dependency-light while public contracts are still stabilizing.

## License

Apache License 2.0. See [`LICENSE`](LICENSE).

## Development and system proof

Linura keeps its production-oriented development path in the repository rather than in maintainer folklore.

```bash
cargo xtask check
cargo xtask acceptance-list
cargo xtask vm-plan
cargo xtask image-plan
```

The grand development foundation includes checkpointed bootstrap, migrations, coordinated updates, config ownership/drift, sanitized hardware evidence, disposable QEMU/KVM acceptance, visual-regression contracts, exact-SHA release candidate proof, build/publish separation, and independent release-asset verification.

See [Development infrastructure](docs/development-infrastructure.md) and [Development lessons adopted from Omarchy](docs/omarchy-development-lessons.md). Linura adopts Omarchy's strong distro-development discipline while deliberately rejecting unsandboxed plugins, shell strings as the authority API, arbitrary privileged hooks, and model-to-root execution.
