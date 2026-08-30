# Roadmap

## v0.0.x — intent-native trustworthy vertical slice
- Linura naming/product contracts;
- canonical eleven-stage managed-mutation lifecycle contract;
- intent + requirements + reusable setup + system graph;
- local-first Linura Library architecture and self-contained portable setup/profile contracts;
- capability blueprint/resolution + conflicts;
- semantic provenance;
- D-Bus local gateway and caller identity;
- systemd/NetworkManager observation;
- deterministic intent → desired state → plan with no LLM;
- first privileged systemd executor;
- real prepare/execute/verify/commit/audit/reconcile vertical slice for one narrow capability;
- VM integration harness.

## v0.1 — usable local Linura foundation
- persistent intents/graph/desired state;
- durable local Linura Library for setup/profile revisions;
- setup save/export/import/adopt on the same machine and another supported machine;
- profile export/adopt carrying referenced setup/intent definitions;
- missing-secret-reference resolution during adoption;
- suspend/supersede/retire with safe cleanup analysis;
- explain/why/removal-impact API including setup provenance;
- agent provider adapters limited to `IntentProposal`;
- first-boot flow with offline/default/library/import path;
- service/network/Bluetooth/audio/power/package/firewall/storage basics;
- initial Control Center;
- Arch install/update/recovery path.

## v0.2 — personal operating environment
- machine profiles/personality composition;
- profile/setup capture from managed causal state;
- reusable workflow/library integration;
- profile export/adopt/replay UX;
- shell surfaces and coherent design system;
- declarative workflows;
- constrained derived UI surfaces;
- accessibility/keyboard/mouse parity.

## v0.3 — extension and sharing ecosystem
- capability-isolated extensions;
- signed manifests/update policy;
- canonical setup/profile serialization, content digests and optional signatures;
- optional Git/user-owned/hosted/enterprise Library sync providers;
- UI and workflow extension points;
- local model and enterprise model adapters.

## v0.4+ — optional fleet/enterprise
- enrollment/mTLS;
- central policy and audit export;
- enterprise setup/profile catalog policy;
- fleet intent/state orchestration;
- staged deployment and rollback;
- enterprise model/provider controls.

Remote control, hosted Library sync and model providers never become prerequisites for local authority, setup adoption or recovery.
