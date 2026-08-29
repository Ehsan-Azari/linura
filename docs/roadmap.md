# Roadmap

## v0.0.x — intent-native trustworthy vertical slice
- Linura naming/product contracts;
- intent + requirements + system graph;
- capability blueprint/resolution + conflicts;
- semantic provenance;
- D-Bus local gateway and caller identity;
- systemd/NetworkManager observation;
- deterministic intent → desired state → plan with no LLM;
- first privileged systemd executor;
- execute/verify/audit/provenance vertical slice;
- VM integration harness.

## v0.1 — usable local Linura foundation
- persistent intents/graph/desired state;
- suspend/supersede/retire with safe cleanup analysis;
- explain/why/removal-impact API;
- agent provider adapters limited to `IntentProposal`;
- first-boot flow with offline/default/import path;
- service/network/Bluetooth/audio/power/package/firewall/storage basics;
- initial Control Center;
- Arch install/update/recovery path.

## v0.2 — personal operating environment
- machine profiles/personality composition;
- profile export/adopt/replay;
- shell surfaces and coherent design system;
- declarative workflows;
- constrained derived UI surfaces;
- accessibility/keyboard/mouse parity.

## v0.3 — extension ecosystem
- capability-isolated extensions;
- signed manifests/update policy;
- UI and workflow extension points;
- local model and enterprise model adapters.

## v0.4+ — optional fleet/enterprise
- enrollment/mTLS;
- central policy and audit export;
- fleet intent/state orchestration;
- staged deployment and rollback;
- enterprise model/provider controls.

Remote control never becomes a prerequisite for local authority/recovery.
