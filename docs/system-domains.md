# System domain map

Linura is intended to become a general-purpose system control plane, not only a quick-settings shell. This document is the long-term domain inventory and sequencing map.

Domain sequencing is intentionally independent of product release numbers. Exact release inclusion belongs in milestone/release contracts, and support exists only where a published release explicitly claims and qualifies a bounded capability.

Shipping a domain requires more than UI or code presence: a provider contract, typed capability/resource model, policy semantics, authoritative observation, validation, recovery/verification appropriate to risk, tests, documentation and release evidence are required for the supported slice.

See [Roadmap](roadmap.md) for the canonical trust-boundary release spine and domain maturity levels.

## Sequencing classes

| Class | Meaning |
|---|---|
| Core proof | Used to prove Linura's generic trust/mutation lifecycle before broad domain expansion. |
| Early product | Valuable for the first coherent local experience after the core lifecycle is proven. |
| Product experience | Primarily user-environment/desktop experience work that depends on the trustworthy core. |
| High risk | Requires domain-specific recovery, verification and stricter policy before mutation support. |
| Later provider | General-purpose provider/domain that should reuse the mature lifecycle rather than define it. |
| Ecosystem | Extension/sharing capability built after local authority and lifecycle semantics are trustworthy. |
| Optional enterprise | Remote/fleet capability that must never become a prerequisite for local authority or recovery. |

## Domain inventory

| Domain | Representative resources/actions | Sequencing class |
|---|---|---|
| System identity | OS/kernel/hardware/profile/hostname/time | Early product |
| Health/readiness | provider health, degraded capabilities, reboot requirement | Early product |
| Network | interfaces, Wi-Fi, Ethernet, DNS, routes, VPN | Early product |
| Bluetooth | adapters, pairing, connect, trust, forget | Early product |
| Audio/media | devices, routes, profiles, volume, microphone | Early product |
| Displays | outputs, mode, scale, layout, HDR/VRR capabilities | Product experience |
| Input | keyboard, mouse, trackpad, layout, accessibility | Product experience |
| Power/session | profiles, battery, suspend, lock, reboot, shutdown | Early product |
| Storage | disks, partitions, mounts, filesystems, SMART, encryption metadata | High risk |
| Snapshots/recovery | Btrfs/Snapper snapshots, rollback, recovery status | High risk |
| Packages/apps | inventory, install/remove/update, provenance | Early product |
| Services | systemd units, enable/start/stop/restart, health | Core proof |
| Processes | inspection and constrained lifecycle operations | Later provider |
| Firewall | zones/rules/ports/provider state | Early product / high risk |
| Remote access | SSH service/config exposure | High risk |
| Users/sessions | local users, sessions, groups, login policy | High risk |
| Credentials | secret references, keyrings, hardware auth | High risk |
| Security posture | Secure Boot, TPM, LUKS, kernel lockdown, firewall, SSH | High risk |
| Updates | system packages, Linura, migrations, reboot/restart needs | High risk |
| Boot | boot entries, kernel/initramfs state, recovery selection | High risk |
| Containers | Docker/Podman runtime/inventory/ports | Later provider |
| Virtualization | provider-neutral VM inventory and lifecycle; possible libvirt/QEMU/KVM, Incus or other adapters | Later provider / high risk |
| Printers/scanners | CUPS/SANE resources | Later provider |
| Time/locale | timezone, locale, formats | Product experience |
| Accessibility | text scale, motion, contrast, input aids | Product experience |
| Desktop/session | compositor workspaces/windows/theme/background | Product experience |
| Notifications | notification service/preferences/history | Product experience |
| Diagnostics | logs, device/provider diagnostics, redacted support bundle | Early product |
| Audit | local mutation evidence, export, retention | Core proof / early product |
| Agent permissions | grants, approvals, revocation, history | Core proof |
| Extensions | manifests, capabilities, lifecycle, updates | Ecosystem |
| Remote/fleet | enrollment, inventory, desired state, central policy | Optional enterprise |

## Current release-qualified slices

This inventory must not be read as a support matrix. Current published evidence is deliberately much narrower:

- v0.1.0 release-qualified authenticated authoritative read-only observation and causal-graph behavior for the bounded provider/scenario claims stated in its release contract;
- v0.2.0 release-qualified deterministic desired-state/planning behavior over the bounded authoritative observation route used by its plan-preview claim;
- no published release currently supports Linura-managed external mutation;
- no published release currently declares a supported Linux distribution, desktop, hardware or virtualization product profile.

Future releases promote only the exact bounded slices they qualify.

## VM qualification versus VM management

Linura already has repository-owned disposable QEMU/KVM guest infrastructure for system acceptance and release qualification. That is **test infrastructure, not a product virtualization capability**.

Product virtualization is a future provider domain. Linura must represent virtual machines through provider-neutral typed resources/capabilities rather than binding its architecture to one backend. Potential adapters may include libvirt/QEMU/KVM, Incus or other local/remote implementations, but all remain optional providers.

A future VM lifecycle capability must preserve Linura's canonical eleven-stage lifecycle:

```text
request / intent
→ observe VM state
→ plan typed desired VM state / diff
→ validate
→ authorize
→ prepare
→ execute through a narrow provider executor
→ verify through independent re-observation
→ commit
→ audit
→ reconcile
```

Planning internals may resolve VM requirements, provider capabilities, images, networks, storage, devices and desired state, but those remain implementation details of `plan`/`validate`; they do not redefine the lifecycle.

Destructive operations such as VM deletion, disk replacement, snapshot rollback or host-device/GPU passthrough require explicit risk classification, authorization and recovery semantics; they must not be exposed through a generic privileged command path.

## Capability maturity

The roadmap defines the canonical domain maturity scale:

- **D0 identified** — inventory only;
- **D1 contracted** — typed resource/capability/provider contracts;
- **D2 implemented** — implementation exists without system qualification;
- **D3 integrated** — control/public integration plus negative-path coverage;
- **D4 system-tested** — disposable-machine/system evidence;
- **D5 release-qualified** — exact-source release evidence for the bounded claim;
- **D6 Experimental supported** — published release explicitly supports the bounded capability;
- **D7 Stable supported** — compatibility/support guarantees explicitly promoted and qualified.

A domain can have different maturity per capability. For example, `service.read` can be release-qualified while `service.start` remains unsupported. Do not assign one maturity level to an entire domain when only a subset is proven.

## Boundary rule

Domains share core primitives but not implementation shortcuts. Package installation, firewall changes, storage operations and VM lifecycle may all require privilege, but they must not be routed through a generic privileged command runner.

Provider-specific execution stays behind narrow typed capabilities, and authoritative observation/verification remains independently defined from execution.

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
virtualization.vm.read
virtualization.vm.plan
virtualization.vm.start
virtualization.vm.stop
virtualization.vm.create
virtualization.vm.delete
firewall.read
firewall.rule.write
```

Avoid monolithic grants such as `system.admin`, `virtualization.admin` or unrestricted shell execution in normal agent/plugin policy.

## Domain roadmap rules

1. Domain inventory entries do not imply implementation or support.
2. Exact version targeting is recorded in active milestone contracts, not permanently baked into this long-term inventory.
3. High-risk mutation requires the generic durable/recovery lifecycle plus domain-specific failure and recovery semantics.
4. New providers must not bypass capability resolution, policy, authorization, prepare, verification or audit boundaries.
5. Backend adapters remain replaceable. Linura's source of truth is its typed intent/desired-state/evidence model, not libvirt, Incus, Docker, NetworkManager or any other external provider.
6. Remote/fleet providers and hosted services remain optional; loss of them must not destroy local authority, local recovery or the user's portable Library definitions.
