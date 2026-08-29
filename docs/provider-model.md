# Provider model

Providers adapt Linux subsystems to Linura domain contracts.

A provider declares:
- stable provider ID/version;
- supported capabilities;
- resources it can observe;
- actions it can plan;
- whether it can execute directly or needs a privileged executor;
- verification strategy;
- diagnostic metadata.

Expected first providers:

| Domain | Provider |
|---|---|
| Network | NetworkManager over D-Bus |
| Bluetooth | BlueZ over D-Bus |
| Audio/media | PipeWire/WirePlumber |
| Services | systemd D-Bus |
| Storage | UDisks2 + filesystem-specific helpers |
| Authorization | Polkit |
| Snapshots | Snapper |
| Firewall | nftables/firewalld profile, selected by platform profile |
| Packages | pacman on Arch profile |

Providers must not leak raw command output into public API types. Provider-specific diagnostics may be attached in explicitly namespaced diagnostic fields.
