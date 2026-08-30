# Development infrastructure

Linura treats development infrastructure as part of the product safety boundary. The same repository-owned commands should be used locally, in CI, in VM acceptance, and in release proof whenever practical.

## Canonical entry point

```bash
cargo xtask check
```

This executes formatting, Clippy, workspace tests, repository invariants, and structured asset validation. Additional commands expose acceptance scenarios and build plans without requiring contributors to memorize implementation-specific scripts.

## Deterministic Codex/cloud environment

Codex Cloud environments are external execution containers, but Linura keeps their repository-owned setup contract in version control so delegated tasks do not depend on ad-hoc `latest` installs.

The canonical environment-creation command is:

```bash
bash scripts/setup_codex_environment.sh
```

The setup script:

- requires a Linux x86_64 host with the repository-declared Python major/minor and basic host primitives already available;
- never uses `apt install`, Homebrew, or an unversioned language/tool installer;
- installs the exact Rust toolchain declared by both `rust-toolchain.toml` and `tools/codex/versions.env`;
- installs exactly `cargo-audit` 0.22.2 with Cargo's locked install mode;
- downloads exactly actionlint 1.7.12 and verifies the same SHA-256 used by CI before extracting it;
- fetches only the locked Cargo dependency graph;
- fails if setup changes tracked repository state.

The canonical task-time preflight is:

```bash
bash scripts/preflight_codex_environment.sh
```

Use `--full` when a task should also run the complete `cargo xtask check` quality gate during preflight. The normal preflight is intentionally non-installing and non-mutating: it verifies host architecture, Python major/minor, Rust, cargo-audit, actionlint, offline locked Cargo metadata, workflow semantics and clean tracked state. A mismatch is reported as an environment defect rather than repaired during delegated implementation.

Repository-owned versions are declared in `tools/codex/versions.env`. Changing those pins is a reviewed development-infrastructure change and should remain aligned with CI/release tooling where the same tool is security-relevant.

The Codex product-side environment still has to be created/selected for `linura-org/linura`; the repository cannot create that account-level object by committing a setup script. Configure that environment to execute the canonical setup command during environment creation, with setup-phase network access only to the upstreams needed for the pinned Rust toolchain, crates.io dependency fetches and the pinned actionlint GitHub release. Ordinary task execution should not install replacement tool versions.

## Layers

```text
source + schemas
      ↓
 cargo xtask
      ↓
unit / contract / adversarial tests
      ↓
image and disposable-VM harnesses
      ↓
hardware and visual evidence
      ↓
exact-SHA release candidate
      ↓
promotion + post-publication verification
```

Repository tooling must fail clearly when host capabilities such as QEMU, KVM, mkarchiso, SSH, or ImageMagick are unavailable. Missing host tooling is not silently treated as passing system evidence.

## Development invariants

- no privileged shell script is the canonical system contract;
- all host-mutating behavior must eventually cross typed Linura authority boundaries;
- system-image/bootstrap/update tooling must be restartable or explicitly recoverable;
- platform-specific code stays under platform/provider/executor boundaries;
- generated artifacts and fixtures are versioned and machine-readable;
- CI actions are pinned to immutable commit SHAs;
- delegated/cloud development environments use repository-owned pinned tool contracts rather than task-time latest-version installation;
- release bytes are built once and promoted, not rebuilt during publication.

The Arch image harness stages from ArchISO's `releng` profile and overlays Linura additions instead of pretending a sparse custom profile is independently boot-complete.
