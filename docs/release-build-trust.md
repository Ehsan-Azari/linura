# Trusted release build boundary

Linura separates release authorization from the build instructions that produce promotable bytes.

## Deterministic stage graph

The release control plane uses explicit, authenticated workflow dispatches between automated stages:

```text
protected main release intent
  -> exact-SHA CI / Security / CodeQL
  -> Release Proof Dispatch
  -> Trusted Release Proof
  -> Reusable Trusted Release Build
  -> Release Promotion
  -> Release
  -> Verify published release
```

`workflow_run` is not used as an implicit message bus between release stages. Every receiver independently validates the source SHA, parent run identity, release contract and current repository state before granting the next capability.

## Reusable trusted builder

`.github/workflows/reusable-release-build.yml` is the canonical release builder. The calling proof workflow validates authorization and then delegates build instructions to that reusable workflow with only the exact source SHA, release tag and version.

The reusable builder:

- runs on the explicit `ubuntu-24.04` runner family rather than the moving `ubuntu-latest` alias;
- installs Rust 1.98.0 and targets `x86_64-unknown-linux-gnu` explicitly;
- builds with locked dependencies and disabled incremental compilation;
- derives `SOURCE_DATE_EPOCH` from the source commit;
- normalizes timezone and locale;
- remaps the workspace path from Rust debug/build metadata;
- records the runner, operating system, Rust/Cargo and build-envelope details in `BUILD-ENVIRONMENT.json`;
- constructs the release payload once, seals it with checksums and release evidence, and creates GitHub/Sigstore build-provenance attestations;
- asserts the source tree remains unchanged throughout the build.

Using a reusable workflow for the trusted build and generating artifact attestations follows GitHub's recommended isolation model for strengthening SLSA v1 build provenance. Linura treats this as SLSA-3-style build isolation; it does not claim independent SLSA certification merely because the workflow exists.

## Independent reproducibility check

A second fresh `ubuntu-24.04` job rebuilds the exact source with the same pinned toolchain, target and deterministic environment. It downloads the sealed proof payload and compares each distributable binary byte-for-byte:

- `linurad`
- `linuractl`
- `linura-firstboot`
- `linura-update-guard`
- `linura-executor-systemd`

A mismatch fails Trusted Release Proof and prevents promotion. Metadata such as the proof receipt and recorded runner environment is intentionally not required to reproduce byte-for-byte; the qualification applies to the distributable binaries.

## Authority boundary

The reusable builder has no repository-content write permission and no tag or GitHub Release authority. Promotion can dispatch the final Release workflow but cannot publish. Only the final Release publication job, behind the `release` environment, receives `contents: write` and can create the immutable version tag and publish the already-proven bytes.
