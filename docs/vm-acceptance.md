# Disposable VM acceptance

Linura system changes require disposable-machine evidence.

## Harness

- `tools/vm.py` constructs/starts a disposable QEMU/KVM guest using a qcow2 image and snapshot mode;
- an optional read-only cloud-init seed can be attached for deterministic disposable guest provisioning;
- `tools/acceptance.py` loads versioned scenarios from `tests/acceptance/` and executes steps over SSH;
- `.github/workflows/vm-acceptance.yml` builds the exact checked-out observation binaries, provisions an ephemeral guest identity, and runs repository scenarios against a SHA-256-pinned released Ubuntu cloud image.

The harness defaults to `auto` acceleration: it selects KVM only when the current process can actually open `/dev/kvm` read/write and otherwise selects TCG. Device-node existence alone is not treated as KVM availability because hosted/containerized environments can expose `/dev/kvm` while denying access. Callers may explicitly select `--accel kvm` or `--accel tcg`; an explicit inaccessible KVM request fails before QEMU is launched. A missing QEMU or SSH tool is a failed doctor check, not passing evidence. Guest execution always uses QEMU snapshot mode so the verified base image is not mutated.

## Reproducible release qualification

The canonical automated guest for the authoritative-observation baseline is a dated Ubuntu 24.04 LTS amd64 cloud image from `cloud-images.ubuntu.com/releases/`, pinned by an exact repository-owned URL and SHA-256 digest. Floating `current` images are not accepted as release evidence.

GitHub-hosted qualification explicitly uses TCG rather than inferring acceleration from `/dev/kvm`, because hosted runners do not promise that a visible KVM device is accessible to the workflow job. Local and dedicated-runner users retain `auto`/KVM support through the general-purpose harness.

The workflow:

1. checks out the exact workflow source SHA and verifies the repository Rust toolchain contract;
2. builds `linurad` and `linuractl` from that exact source with locked dependencies;
3. downloads the pinned cloud image over HTTPS and verifies its SHA-256 before execution;
4. generates a one-run Ed25519 SSH identity and NoCloud seed for the disposable `linura` guest user;
5. boots the guest in snapshot mode with an explicit recorded accelerator and fails immediately if the QEMU process exits before SSH readiness;
6. waits for cloud-init to finish successfully and copies only the exact-source test binaries into the ephemeral guest;
7. executes the versioned repository acceptance scenario over SSH;
8. records `VM-ACCEPTANCE-EVIDENCE.json` with source SHA, scenario digest, base-image URL/digest, accelerator, tested binary digests, harness package versions and workflow identity;
9. uploads the evidence and QEMU diagnostics as run-scoped artifacts and destroys the guest.

`Trusted Release Proof` calls this workflow directly for `authoritative-observation`. The trusted release build and promotion jobs depend on its success, so an exact-source release cannot be promoted merely because CI/Security/CodeQL passed while required system evidence was absent.

The cloud-init guest has passwordless sudo solely so the acceptance harness can create, mutate and remove its external disposable systemd fixture. Linura itself remains unprivileged and read-only; the scenario does not introduce a privileged Linura execution path.

## Initial scenarios

The repository reserves acceptance coverage for bootstrap resume, offline first boot, fail-closed security baseline, intent retirement, interrupted updates, and native recovery.

`authoritative-observation` is the first runtime release-gating scenario. It starts `linurad` on an isolated session bus, proves D-Bus/OS-derived caller identity, observes a disposable transient systemd unit through the native system bus, changes that fixture out of band, proves the new authoritative state becomes visible, checks graph/explanation evidence, and observes NetworkManager manager state when that service is available. The scenario may use passwordless `sudo` only to create and stop the disposable external systemd fixture; Linura itself remains unprivileged and read-only.

Real scenarios become release-gating only when the relevant Linura capability exists. Placeholder commands must never be interpreted as proof of an unimplemented feature.
