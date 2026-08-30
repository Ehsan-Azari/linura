# Disposable VM acceptance

Linura system changes require disposable-machine evidence.

## Harness

- `tools/vm.py` constructs/starts a disposable QEMU/KVM guest using a qcow2 image and snapshot mode;
- `tools/acceptance.py` loads versioned scenarios from `tests/acceptance/` and executes steps over SSH;
- `.github/workflows/vm-acceptance.yml` can run a repository scenario against an exact SHA-256-verified qcow2 image.

The harness uses KVM when `/dev/kvm` exists and otherwise emits a TCG-compatible plan. A missing QEMU or SSH tool is a failed doctor check, not passing evidence.

## Initial scenarios

The repository reserves acceptance coverage for bootstrap resume, offline first boot, fail-closed security baseline, intent retirement, interrupted updates, and native recovery.

`authoritative-observation` is the first runtime release-gating scenario. It starts `linurad` on an isolated session bus, proves D-Bus/OS-derived caller identity, observes a disposable transient systemd unit through the native system bus, changes that fixture out of band, proves the new authoritative state becomes visible, checks graph/explanation evidence, and observes NetworkManager manager state when that service is available. The scenario may use passwordless `sudo` only to create and stop the disposable external systemd fixture; Linura itself remains unprivileged and read-only.

Real scenarios become release-gating only when the relevant Linura capability exists. Placeholder commands must never be interpreted as proof of an unimplemented feature.
