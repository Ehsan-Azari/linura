# Threat model

## Assets

- host integrity/availability and bootability;
- user data, credentials and secret references;
- network/security configuration;
- approved intent and desired state;
- reusable Setup/Profile definitions and Library history;
- system graph and semantic provenance;
- audit/policy/grant state;
- trusted risk-policy state and classification provenance;
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
- authenticated OS principal binding distinct from client-supplied provenance;
- explicit grants + policy evaluation;
- plan/approval before privilege;
- strict executor revalidation.

### Risk downgrade, under-classification or classifier substitution
A caller, model, provider or configuration change attempts to make a dangerous canonical plan look like a lower-risk mutation, or relies on an unknown mutation shape receiving the ordinary mutation approval class.
- planner `prospective_risk` is a lower bound and cannot be reduced by authority classification;
- trusted deterministic risk classification is owned by Linura Control, not client/model/provider payloads;
- classification uses exact typed canonical plan material such as provider/resource/capability/change keys;
- no matching trusted rule means unclassified mutation risk and the review fails closed as `blocked`;
- classification below the planner floor is rejected and the review fails closed as `blocked`;
- overlapping rules choose the highest risk deterministically rather than the first/lowest match;
- risk-policy revision and matched rule identities are retained as material review findings/provenance;
- approval/review reuse across changed risk-policy provenance or resulting risk is rejected;
- the initial v0.3 rule set is deliberately narrow rather than guessing risk for future mutation domains.

### Approval replay, theft or policy substitution
A caller attempts to reuse approval for a different principal, plan, evidence snapshot, resource/capability, risk classification or policy revision, or to preserve approval after expiry/revocation.
- policy review is derived from the canonical `ReconciliationPlan`, not a client-authored executable plan;
- review binds authenticated principal + request/plan identity + authoritative evidence identity + provider/resource/capability + material risk-classification provenance + policy ID/revision;
- material planned changes/findings and semantic provenance are revalidated against the reviewed subject before approval evidence is accepted;
- `PlanId` alone is never sufficient authority evidence;
- approval evidence is authenticated, scoped to an explicit approval requirement, and checked for approver constraints, expiry and revocation at use time;
- approval issuance/validation/revocation and replay-tombstone pruning obtain current time inside Linura Control; callers cannot supply the authority clock used to keep evidence current or reopen replay IDs;
- authority time is monotonic within the process: a backward host wall-clock sample fails closed, so expired evidence cannot revive and replay-tombstone decisions cannot be reversed by clock rollback;
- cross-principal approval reuse and changed policy/risk-policy revision reuse fail closed;
- agents/models cannot mint approval evidence or satisfy their own protected human/admin approval requirement;
- v0.3 approval never creates prepare/executor authority; later prepare must revalidate exact review binding before effects.

### Prompt injection / model compromise
External content manipulates an agent into dangerous proposals.
- model output has proposal authority only;
- trusted structured validation/solver/policy/risk-classification boundaries;
- no executor/tool handle in model runtime;
- context/source provenance and user-visible material effects;
- negative tests for escalation attempts.

### Malicious reusable setup/profile
An imported or synchronized artifact attempts to smuggle commands, unsafe state, authority or credentials onto the target machine.
- setup/profile formats are declarative and contain no executor command transcript;
- imported artifacts carry no grants/approvals;
- schema/composition/cycle validation before adoption;
- secret values prohibited; only secret refs are portable;
- fresh target observation + capability resolution + plan/risk classification/policy/approval;
- unsupported/ambiguous requirements fail closed;
- provenance distinguishes imported definitions from locally approved/adopted lineage.

### Machine-class spoofing / cross-class adoption confusion
A malicious or malformed portable profile lies about its source machine class, or attempts to use a workstation/server/edge label to obtain capabilities, support status, weaker policy, or unsafe cross-class replay on the target machine.
- imported `machine_class` is untrusted declarative source metadata and never an authority, grant, executor selector, risk override, or support assertion;
- the Experimental portable-profile schema accepts only the canonical `workstation`, `server`, and `edge` values and rejects missing/unknown values;
- the target machine is independently observed and its local capabilities/platform support are resolved from authoritative local evidence rather than trusted from the imported class label;
- source/target class differences are compatibility inputs that require fresh resolution and a fresh reviewable plan; historical executor effects are never replayed;
- unsupported, ambiguous, or unsafe cross-class compatibility fails closed rather than coercing one class into another;
- policy/approval remains bound to the fresh target plan and authenticated local authority context, so changing a class label cannot preserve or manufacture prior approval;
- release-qualified platform support is established only by release evidence for the exact local class + platform/profile + architecture/hardware boundary, never by a portable artifact declaring a class;
- provenance retains imported origin so later explanation/audit can distinguish source metadata from locally observed and approved facts.

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
- review is bound to the exact authoritative evidence, material plan and trusted risk classification being approved;
- policy/risk-policy/approval changes invalidate stale review evidence;
- revalidation immediately before high-risk effects;
- plan expiry/replan rules;
- setup/profile adoption always plans against current target state.

### Approval fatigue/deception
- aggregate material effect/change summary;
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
