# ADR 0016 — Machine classes and portable profile semantics

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

Linura is intended to manage multiple Linux machine roles without turning each role into a separate authority architecture. The roadmap now names three target machine classes—workstation, server and edge—while domain maturity remains tracked independently through D0–D7.

Machine profiles are also portable declarative artifacts. If a profile's source machine class exists only in documentation, an exported profile cannot reliably distinguish a workstation profile from a server or edge profile during adoption. That would make cross-class compatibility checks, qualification boundaries and recovery expectations ambiguous.

Enterprise/fleet management creates a second possible ambiguity: a fleet is a topology over machines, not a local machine role and not a replacement for each node's local Linura authority.

## Decision

Linura adopts these invariants:

1. The canonical target machine classes are **workstation**, **server** and **edge**.
2. `developer machine` is normally a workstation profile, not a fourth machine class.
3. `linura-intent` owns a typed `MachineClass` value and every `MachineProfile` carries exactly one machine class.
4. Portable machine-profile exports preserve the machine class through the embedded `MachineProfile`; the Experimental `portable-profile.v1` schema requires `profile.machine_class` with the same three canonical values.
5. Cross-class adoption never silently replays a source profile as if the target class were equivalent. Adoption re-observes the target, resolves capabilities and compatibility, and produces a fresh reviewable plan.
6. Machine class is orthogonal to system-domain maturity. D0–D7 remains the only domain capability maturity scale.
7. A machine-class declaration is not a support claim. Support requires release-qualified evidence for an exact class + platform/profile + architecture/hardware boundary.
8. Fleet/enterprise remains an optional management overlay across workstation/server/edge machines. It is not a fourth machine class and cannot replace local authority, recovery or Library semantics.
9. AI/agent interpretation remains an upstream proposal plane, not a machine class or authority source.

## Consequences

- Portable profiles retain enough semantic information to detect and review cross-class adoption.
- The Rust intent domain, public SDK and JSON profile contract share one canonical machine-class vocabulary.
- Workstation, server and edge can require different qualification evidence without forking Linura's authority lifecycle.
- Domain capabilities can mature independently and may have different support evidence on different machine classes.
- Fleet deployments can contain any mixture of workstation, server and edge nodes while every node remains locally authoritative.
- Existing published releases gain no new workstation/server/edge support claim from this decision; support remains evidence-bound and release-scoped.
