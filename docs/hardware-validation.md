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

`hardware/support-matrix.json` records the current evidence tier per domain. At bootstrap, no physical hardware is release-qualified.

## Sanitized fixtures

Fixtures under `hardware/fixtures/` contain structural observations only. They must not contain serial numbers, MAC addresses, hostnames, usernames, IP addresses, account identifiers, or other machine-owner secrets.

Target matrix areas include Intel/AMD/NVIDIA graphics, Wi-Fi/Bluetooth, USB-C docks, HiDPI and mixed-DPI displays, NVMe/SATA/Btrfs storage, suspend/resume, batteries, and audio devices.

A support claim must cite evidence. Unknown hardware should degrade explicitly rather than pretending to be supported.
