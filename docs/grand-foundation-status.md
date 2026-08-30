# Grand development-foundation status

This document distinguishes **implemented repository machinery** from **future release-qualified evidence**.

| Development discipline | Repository machinery | Release-qualified today? |
|---|---|---|
| Canonical developer/CI path | `cargo xtask check` | machinery yes; exact published-release evidence still pending |
| Task-specific agent guides | `agents/skills/*` | yes as contributor contract |
| Staged bootstrap | `linura-bootstrap`, install policy, Arch profile | contract yes; real installer not yet qualified |
| Migration framework | `linura-migrations`, versioned descriptors | contract yes; no production migrations yet |
| Coordinated update | `linura-update`, update guard/hook | contract yes; host update engine not yet qualified |
| Config ownership/drift | `linura-config` | model yes; provider integration pending |
| Disposable VM acceptance | QEMU/SSH harness + scenarios | harness yes; no Linura qcow2 support evidence yet |
| Hardware evidence | fixtures + support matrix + evidence tiers | framework yes; no physical hardware qualified |
| Visual regression | semantic tokens + manifest + comparator | harness yes; UI baselines intentionally null |
| Application supervision | systemd-user supervision contract | contract yes; shell integration pending |
| Lifecycle extension safety | typed lifecycle workflows | contract yes; runtime integration pending |
| Version-scoped release claims | milestone/release contracts, claim classes, traceability rules | contract yes; first protected published version pending |
| Release evidence index | `RELEASE-EVIDENCE.json` generator/verifier | machinery yes; first candidate evidence pending |
| Release candidate proof | exact SHA, frozen notes, binaries, SPDX, checksums, provenance | workflow yes; first real candidate pending |
| Build/publish separation | promotion workflow | workflow yes; first promotion pending |
| Published-byte/claim verification | checksum/evidence/body/provenance workflow | workflow yes; first release pending |

This distinction is deliberate. Linura must never turn the existence of a harness, document or workflow into a false support claim.
