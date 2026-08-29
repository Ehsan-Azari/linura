# System domain map

Linura is intended to become a general-purpose system control plane, not only a quick-settings shell. This document is the long-term domain inventory. Shipping a domain requires a provider, capability model, policy semantics, verification, tests, and documentation—not merely UI.

| Domain | Representative resources/actions | Initial priority |
|---|---|---|
| System identity | OS/kernel/hardware/profile/hostname/time | v0.1 read |
| Health/readiness | provider health, degraded capabilities, reboot requirement | v0.1 |
| Network | interfaces, Wi-Fi, Ethernet, DNS, routes, VPN | v0.1 |
| Bluetooth | adapters, pairing, connect, trust, forget | v0.1 |
| Audio/media | devices, routes, profiles, volume, microphone | v0.1 |
| Displays | outputs, mode, scale, layout, HDR/VRR capabilities | v0.2 |
| Input | keyboard, mouse, trackpad, layout, accessibility | v0.2 |
| Power/session | profiles, battery, suspend, lock, reboot, shutdown | v0.1 |
| Storage | disks, partitions, mounts, filesystems, SMART, encryption metadata | v0.1/v0.2 |
| Snapshots/recovery | Btrfs/Snapper snapshots, rollback, recovery status | v0.1 |
| Packages/apps | inventory, install/remove/update, provenance | v0.1 |
| Services | systemd units, enable/start/stop/restart, health | first vertical slice |
| Processes | inspection and constrained lifecycle operations | later |
| Firewall | zones/rules/ports/provider state | v0.1 |
| Remote access | SSH service/config exposure | v0.1 controlled |
| Users/sessions | local users, sessions, groups, login policy | later |
| Credentials | secret references, keyrings, hardware auth | later |
| Security posture | Secure Boot, TPM, LUKS, kernel lockdown, firewall, SSH | v0.2 |
| Updates | system packages, Linura, migrations, reboot/restart needs | v0.1 |
| Boot | boot entries, kernel/initramfs state, recovery selection | later, high risk |
| Containers | Docker/Podman runtime/inventory/ports | later provider |
| Virtualization | libvirt/VM inventory and lifecycle | later provider |
| Printers/scanners | CUPS/SANE resources | later |
| Time/locale | timezone, locale, formats | v0.2 |
| Accessibility | text scale, motion, contrast, input aids | v0.2 |
| Desktop/session | compositor workspaces/windows/theme/background | v0.2 client/profile-specific |
| Notifications | notification service/preferences/history | v0.2 |
| Diagnostics | logs, device/provider diagnostics, redacted support bundle | v0.1 |
| Audit | local mutation evidence, export, retention | v0.1 |
| Agent permissions | grants, approvals, revocation, history | v0.1 |
| Extensions | manifests, capabilities, lifecycle, updates | v0.3 |
| Remote/fleet | enrollment, inventory, desired state, central policy | v0.4+ |

## Boundary rule

Domains share core primitives but not implementation shortcuts. For example, package installation and firewall changes may both require privilege, but they should not be routed through a generic privileged command runner.

## Capability granularity

Prefer narrow capabilities:

```text
network.read
network.wifi.scan
network.wifi.connect
network.dns.read
network.dns.write
service.read
service.start
service.enable
package.read
package.plan
package.apply
firewall.read
firewall.rule.write
```

Avoid monolithic grants such as `system.admin` in normal agent/plugin policy.
