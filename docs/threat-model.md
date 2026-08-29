# Threat model

## Assets

- host integrity/availability and bootability;
- user data, credentials and secret references;
- network/security configuration;
- approved intent and desired state;
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
- malicious or malformed imported machine profile.

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

### Semantic-provenance spoofing
A client invents a false "why" chain to make dangerous state look legitimate.
- provenance creation is authority-owned;
- lineage links approved intent/plan/effect IDs;
- clients cannot rewrite history;
- imported intent/profile provenance remains distinguishable from locally approved lineage.

### Unsafe intent retirement
Removing one goal breaks another because resources are shared.
- system-graph ownership/dependency analysis;
- removal impact plan;
- shared resources retained;
- verification and rollback/snapshot where applicable.

### Dependency/conflict solver manipulation
Malformed capability definitions cause cycles, hidden conflicts or unsafe alternatives.
- schema validation;
- deterministic solver;
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
- plan expiry/replan rules.

### Approval fatigue/deception
- aggregate material effect summary;
- dedicated destructive/security approval classes;
- no model-generated UI allowed to disguise authoritative approval controls;
- policy can require step-up authentication.

### Tampered releases/extensions
- protected release pipeline, immutable tags, SBOM, signing/attestation and asset verification before supported releases;
- extension capability expansion requires approval.

### Audit/provenance tampering
- append-only design;
- corruption detection/chaining/signing evaluated before supported release;
- optional external export for enterprise durability.

## Deferred threats

Fleet/remote orchestration receives a dedicated threat model before any network control plane is enabled.
