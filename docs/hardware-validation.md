# Hardware validation

A Linux system cannot claim production support based only on unit tests or one developer laptop.

## Evidence tiers

`linura-hardware` orders support evidence from weakest to strongest:

1. unknown;
2. fixture-only;
3. virtual machine;
4. community hardware;
5. maintainer hardware;
6. release-qualified.

`hardware/support-matrix.json` records the current evidence tier per domain and the set of release-qualified profiles per canonical target machine class. At bootstrap, no physical hardware and no workstation/server/edge profile is release-qualified.

The canonical target classes are:

- `workstation`;
- `server`;
- `edge`.

Declaring these classes in the support matrix is **not** a support claim. Each class starts with an empty `release_qualified_profiles` list. A release may add an exact profile only when its distribution/desktop-or-headless boundary, architecture, hardware assumptions, relevant domain capabilities and qualification evidence are explicitly bounded by the release contract.

Enterprise/fleet is not a machine class. It is an optional management topology over individually authoritative workstation, server and edge nodes.

## Profile-qualified support

Support must be stated against an exact profile rather than an entire machine class. Conceptually, evidence should become increasingly specific:

```text
machine class
→ platform/profile
→ architecture
→ hardware/domain evidence
→ release qualification
```

For example, evidence for one future `workstation/arch-hyprland-amd64` profile would not automatically qualify `server/ubuntu-amd64`, an arm64 edge gateway, or every workstation.

Domain maturity D0–D7 and machine/profile qualification answer different questions. A domain capability can be mature in implementation while remaining unqualified on a specific machine profile.

## Class-specific qualification concerns

### Workstation

Typical matrix areas include Intel/AMD/NVIDIA graphics, Wi-Fi/Bluetooth, USB-C docks, HiDPI and mixed-DPI displays, NVMe/SATA/Btrfs storage, suspend/resume, batteries, audio devices, interactive session behavior and accessibility-relevant input/display paths.

### Server

Typical matrix areas include headless boot, NIC/storage/controller variants, long-running service behavior, remote recovery, container/virtualization host features, GPU/accelerator use where declared, maintenance/reboot behavior and resilience without a desktop session.

### Edge

Typical matrix areas include arm64 and other explicitly supported architectures, constrained CPU/RAM/storage, intermittent/offline networking, power loss, unattended recovery, image/OTA update and rollback behavior, device identity, removable/flash storage and specialized peripherals/accelerators.

## Sanitized fixtures

Fixtures under `hardware/fixtures/` contain structural observations only. They must not contain serial numbers, MAC addresses, hostnames, usernames, IP addresses, account identifiers, or other machine-owner secrets.

A support claim must cite evidence. Unknown hardware should degrade explicitly rather than pretending to be supported.
