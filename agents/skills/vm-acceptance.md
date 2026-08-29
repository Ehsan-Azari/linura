# VM acceptance task guide

- Use disposable qcow2 guests; never run destructive acceptance tests on a contributor workstation.
- Add scenarios under `tests/acceptance/` using the versioned scenario schema.
- Scenarios should verify externally observable results and recovery, not implementation details.
- Record the exact image digest used for evidence.
- Missing QEMU/KVM/SSH means the system test was not run.
