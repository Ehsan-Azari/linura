# Vision coverage and architecture lock

This matrix converts the design review into repository requirements. "First-class" means the concept now has an explicit module/contract/document/backlog owner; it does **not** claim the implementation is production complete at `v0.0.0`.

| Idea | Previous state | Linura architecture after this update |
| --- | --- | --- |
| Single Linura product/code namespace | Not previously explicit | **Locked; control plane is architectural terminology, not a second brand** |
| Typed Linux control plane | Strong | Preserved as authority plane |
| Agent cannot directly own/root machine | Strong | Preserved as non-negotiable invariant |
| Narrow privileged executors | Strong | Preserved |
| Provider abstraction | Strong | Preserved |
| Capability discovery | Strong | Preserved; distinct from capability blueprints |
| Plan before apply | Strong | Preserved |
| Policy + approvals | Strong | Preserved |
| Verification | Strong | Preserved |
| Compensation/rollback | Strong | Preserved |
| Audit/provenance of mutations | Partial | **First-class semantic provenance/why chain added** |
| Observed state | Strong | Preserved |
| Desired state | Partial | **Explicitly derived from intent/capability resolution** |
| Reconciliation/drift | Strong foundation | Preserved; intent-aware reconciliation required |
| Agent as untrusted proposer | Strong | Preserved and moved into agent-runtime contract |
| Natural-language intent layer | Missing | **First-class intelligence-plane contract added** |
| First-boot agent | Missing | **`linura-firstboot` + product architecture added** |
| “What should this computer become?” workflow | Missing | **Defining product flow** |
| Persistent intent model | Missing | **`linura-intent` added** |
| Reusable setup abstraction | Missing | **Typed/versioned `Setup` between intent and whole-machine profile** |
| Linura Library | Missing | **Local-first reusable artifact catalog/storage architecture added** |
| Same-device setup reuse | Missing | **Adoption explicitly re-observes and replans instead of replaying commands** |
| Cross-device setup reuse | Partial via profile idea | **Self-contained setup bundles + target-specific capability resolution** |
| Secret-safe portability | Missing | **Portable artifacts carry secret refs only; values stay local** |
| Full system graph | Missing | **`linura-graph` added; Setup is now a causal node family** |
| Dependency solver | Insufficient | **Capability resolver + graph relations added; advanced solver remains roadmap work** |
| Conflict analysis | Missing | **Explicit relation/result added** |
| Capability composition | Missing | **`linura-capability-sdk` + blueprint examples added** |
| Remember why something exists | Missing | **SemanticReason + provenance chain added** |
| Safely remove intent/derived dependencies | Missing | **Removal/shared-ownership analysis added** |
| Build new workflows from primitives | Missing | **Declarative workflow model + examples added** |
| Dynamically add UI for new capabilities | Missing | **Constrained derived-surface model added** |
| Specialist agent architecture | Missing | **Provider-neutral specialist roles added** |
| Machine personality/profile derived from user | Missing | **MachineProfile composes setups/intents/constraints** |
| Intent export/replay on another machine | Missing | **Self-contained portable setup/profile protocol + schema added** |
| Snapshot vs portable configuration distinction | Ambiguous | **Locked as separate recovery vs declarative portability concepts** |
| Minimal install/bootstrap/recovery UX | Missing | **Bootstrap/recovery and first-boot architecture added** |
| Enterprise/fleet | Sufficient future foundation | Preserved; remains post-local-trust work |

## Gate

A future architecture change may not bypass intent provenance, setup/profile adoption validation, the authority plane, or the narrow privilege boundary simply to make agent behavior or synchronization easier.
