# Threat model

## Assets

- host integrity/availability and bootability;
- user data, credentials and secret references;
- network/security configuration;
- approved intent and desired state;
- reusable Setup/Profile definitions and Library history;
- system graph and semantic provenance;
- audit/policy/grant state;
- update/release trust roots;
- user understanding at approval time.

## Adversaries

- malicious/compromised local application;
- malicious extension/workflow/capability package;
- compromised hosted/local AI model or agent runtime;
- prompt injection embedded in web/files/repositories/messages;
- malicious package/update artifact;
- unprivileged local user on multi-user machine;
- remote attacker against an intentionally exposed service;
- accidental administrator/model/automation mistakes;
- malicious or malformed imported/synchronized Setup or MachineProfile.

## Primary threats and mitigations

### Confused deputy / privilege laundering
Low-privilege client or agent convinces Linura to perform privileged work.
- authenticated OS actor binding;
- explicit grants + policy evaluation;
- plan/approval before privilege;
- strict executor revalidation.

### Prompt injection / model compromise
External content manipulates an agent into dangerous proposals.
- model output has proposal authority only;
- trusted structured validation/solver/policy boundaries;
- no executor/tool handle in model runtime;
- context/source provenance and user-visible material effects;
- negative tests for escalation attempts.

### Malicious reusable setup/profile
An imported or synchronized artifact attempts to smuggle commands, unsafe state, authority or credentials onto the target machine.
- setup/profile formats are declarative and contain no executor command transcript;
- imported artifacts carry no grants/approvals;
- schema/composition/cycle validation before adoption;
- secret values prohibited; only secret refs are portable;
- fresh target observation + capability resolution + plan/policy/approval;
- unsupported/ambiguous requirements fail closed;
- provenance distinguishes imported definitions from locally approved/adopted lineage.

### Secret leakage through portability/sync
A setup/profile export or Library sync accidentally contains credentials.
- portable domain types expose secret-reference fields only;
- export validation/redaction tests;
- secret stores remain local/provider-specific;
- synchronized artifacts are treated as potentially public/untrusted unless explicitly protected by a future storage provider.

### Semantic-provenance spoofing
A client invents a false "why" chain to make dangerous state look legitimate.
- provenance creation is authority-owned;
- lineage links approved intent/setup/plan/effect IDs;
- clients cannot rewrite history;
- imported setup/profile provenance remains distinguishable from locally approved lineage.

### Unsafe intent/setup retirement
Removing one goal or reusable setup breaks another because resources are shared.
- system-graph ownership/dependency analysis;
- removal impact plan;
- shared resources retained;
- verification and rollback/snapshot where applicable.

### Dependency/conflict/setup composition manipulation
Malformed capability/setup definitions cause cycles, hidden conflicts or unsafe alternatives.
- schema validation;
- deterministic solver/composition resolution;
- explicit unsatisfied/conflict results;
- bounded resource usage/cycle detection;
- signed/trusted catalogs before supported third-party distribution.

### Command/injection attacks
- no generic shell interface;
- native APIs preferred;
- fixed executable + structured argv only where subprocesses are unavoidable;
- secret values never in argv.

### TOCTOU/stale plan
- observed-state freshness and preconditions;
- revalidation immediately before high-risk effects;
- plan expiry/replan rules;
- setup/profile adoption always plans against current target state.

### Approval fatigue/deception
- aggregate material effect summary;
- dedicated destructive/security approval classes;
- no model-generated UI allowed to disguise authoritative approval controls;
- policy can require step-up authentication.

### Tampered releases/extensions/library catalogs
- protected release pipeline, immutable tags, SBOM, signing/attestation and asset verification before supported releases;
- extension capability expansion requires approval;
- future shared catalogs need content identity/signature policy before being considered trusted distribution channels.

### Audit/provenance tampering
- append-only design;
- corruption detection/chaining/signing evaluated before supported release;
- optional external export for enterprise durability.

## Deferred threats

Fleet/remote orchestration and hosted/shared Library services receive dedicated threat-model extensions before any network control plane or trusted shared catalog is enabled.
