# Naming and product architecture

Linura is the only umbrella brand and repository namespace.

The name is inspired by **Linux + aura**: Linux underneath, with a coherent, intelligent, beautiful system layer around it. The construction is brand meaning, not an architectural dependency.

## Naming hierarchy

| Layer / surface | Canonical name | Code / namespace |
| --- | --- | --- |
| Umbrella project and ecosystem | **Linura** | `linura-*`, `org.linura.*` |
| Installable distribution | **Linura OS** | reserved until an installable supported OS product exists |
| Local authority subsystem | **Linura Control** | `linura-control`, `linurad`, `org.linura.Control1` |
| Agent experience/runtime | **Linura Agent** | `linura-agent-runtime`, `apps/linura-agent-ui` |
| Desktop shell | **Linura Shell** | `apps/linura-shell` |
| Graphical management client | **Linura Control Center** | `apps/linura-control-center` |
| First-boot experience | **Linura First Boot** | `linura-firstboot` |
| Developer-facing API facade | **Linura SDK** | `linura-sdk` |
| Deterministic CLI | `linuractl` | `linuractl` |
| Main local daemon | `linurad` | `linurad` |

## Architectural terminology

**System control plane**, **authority plane**, **intelligence plane**, **experience plane**, and **provider plane** are architecture terms, not separate brands.

Do not introduce a second proper-noun infrastructure brand for the control plane. Documentation may say:

> `linura-control` implements Linura's local system control plane.

## Product boundary

Linura may support two deployment forms without changing its core model:

1. **Linura OS** — an installable, opinionated distribution/profile with Linura integrated from first boot.
2. **Linura on another Linux platform** — Linura Control, Agent, Control Center, SDK, and supported providers installed on a compatible Linux profile.

The first supported platform remains Arch/Hyprland, but the Linura brand and control-plane contracts must not encode Arch as a permanent assumption.

## Positioning

Official descriptor:

> **Linura — The intelligent system layer for Linux.**

Primary product promise:

> **Tell your computer what you want it to become.**

Technical promise:

> Agents propose; Linura turns approved intent into policy-controlled, verified system state.
