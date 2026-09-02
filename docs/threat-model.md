# Threat model

## Assets

- host integrity/availability and bootability;
- user data, credentials and secret references;
- network/security configuration;
- approved intent and desired state;
- reusable Setup/Profile definitions and Library history;
- system graph and semantic provenance;
- audit/policy/grant state;
- durable authority transaction/idempotency/recovery state;
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
- review binds authenticated principal + request/plan identity + complete authoritative observation digest + provider/resource/capability + material risk-classification provenance + policy ID/revision;
- material planned changes/findings and semantic provenance are revalidated against the reviewed subject before approval evidence is accepted;
- `PlanId` alone is never sufficient authority evidence;
- approval evidence is authenticated, scoped to an explicit approval requirement, and checked for approver constraints, expiry and revocation at use time;
- approval issuance/validation/revocation and replay-tombstone pruning obtain current time inside Linura Control; callers cannot supply the authority clock used to keep evidence current or reopen replay IDs;
- authority time is monotonic within the process: a backward host wall-clock sample fails closed, so expired evidence cannot revive and replay-tombstone decisions cannot be reversed by clock rollback;
- cross-principal approval reuse and changed policy/risk-policy revision reuse fail closed;
- agents/models cannot mint approval evidence or satisfy their own protected human/admin approval requirement;
- durable prepare revalidates the exact current review, full fresh authoritative observation and approval binding instead of trusting a prior UI/review result;
- approval or a durable prepare record never becomes executor authority by itself.

### Authoritative observation substitution or expiry at prepare
A caller preserves an observation/evidence identifier while changing authority, freshness or observed attributes, or attempts to prepare from evidence that became stale after planning/review.
- v0.4 binds a deterministic digest of the complete validated authoritative observation envelope, including authority/freshness material and canonical attributes;
- Control-owned prepare-time validation checks the complete retained observation binding and current freshness using trusted current time;
- changed authority, validity/freshness window, attributes, provider/resource/capability or other material observation content changes the binding;
- stale/expired observation cannot prepare and instead requires fresh authoritative observation followed by replan/review;
- evidence ID alone is never sufficient prepare evidence.

### Durable prepare substitution or idempotency rebinding
A caller attempts to reuse a durable request/transaction identifier with changed authority material after process restart, or to substitute plan/observation/policy/risk/approval content behind a retained identifier.
- durable idempotency is namespaced by authenticated principal + request ID;
- the durable prepare row binds a deterministic domain-separated digest of exact trusted review, complete observation and authorization material;
- same idempotency namespace + changed binding fails as conflict, including after database reopen;
- SQLite uniqueness constraints backstop in-process checks;
- transaction ID, request ID, plan ID, evidence ID and approval ID are references, not sufficient authority evidence;
- immutable authority-binding fields cannot be silently rewritten after prepare.

### Dispatch-before-indeterminate / ambiguous external effect
A future executor is called while the transaction is still durably `Prepared`, then the process crashes before recording that an effect may have occurred.
- `Prepared` carries no dispatch capability;
- the exact current generation must atomically commit `Prepared` → `Indeterminate` plus its audit event before any executor call;
- any future dispatch capability/token may be created only after the durable `Indeterminate` handoff commits;
- `Indeterminate` survives restart and means effect outcome is unknown until authoritative recovery observation resolves it;
- v0.4 qualifies this handoff without invoking an executor so v0.5/v0.6 cannot introduce a weaker crash window.

### Blind replay after crash / ambiguous external effect
A crash occurs around a future effect boundary and restart incorrectly assumes the effect did or did not happen.
- `Indeterminate` is an explicit durable transaction state and survives restart;
- restart, duplicate delivery, a retained prepare row, executor self-report or local database state never implies retry permission;
- fresh authoritative re-observation is required to resolve ambiguity;
- proof that intended state already exists advances to `Verified`;
- proof that the intended effect did not occur permits only current-authority revalidation/re-prepare eligibility;
- current observation freshness, policy and required approval must be re-established before a new prepared generation, so recovery cannot revive expired/revoked authority;
- conflicting state becomes `RecoveryBlocked`;
- stale/insufficient/ambiguous evidence keeps the transaction `Indeterminate`;
- recovery decisions and attempt generation changes are append-only audited;
- v0.4 exposes no executor/managed effect, so these semantics are qualified before privileged execution is introduced.

### Durable state schema substitution or corruption
Local authority persistence is malformed, partially corrupted, migrated incorrectly, or replaced by an unsupported future schema.
- repository-owned SQLite application ID and explicit supported schema/user version;
- migration IDs/checksums are verified transactionally;
- SQLite integrity and foreign-key checks are part of store integrity validation;
- unsupported newer schema and migration checksum mismatch fail closed;
- exact binding digests and retained audit-chain/state consistency are validated;
- automatic recovery does not silently rewrite corrupted authority history.

### Coherent whole-database rollback / snapshot restore
An attacker or operator replaces the complete authority database with an older internally consistent copy, or restores a VM/filesystem snapshot containing older authority history.
- v0.4 explicitly does **not** claim that SQLite integrity checks, migration checksums or an unkeyed internal audit hash chain can detect this case;
- transparent authority-database restore/rollback and host/VM snapshot rollback are unsupported v0.4 authority states;
- v0.4 idempotency/audit guarantees assume monotonic continuity of the qualified local authority database;
- a future supported restore protocol must use an independently protected monotonic epoch/anchor or otherwise invalidate retained authority and require fresh re-establishment;
- release qualification and documentation must not describe internal hash chaining as an anti-rollback anchor.

### Persistence crash/power/I/O failure
The process or guest loses power around a SQLite transaction boundary, or storage rejects/fails writes.
- WAL + `synchronous=FULL` are required within the qualified local filesystem/storage assumptions;
- state transition and its audit append are one SQLite transaction;
- release qualification injects process `SIGKILL`, abrupt disposable-guest power loss and representative write/I/O failures around transaction boundaries;
- reopen must either show the complete committed transition/audit pair or the prior complete state, never a half semantic transition;
- failed commits do not authorize state advancement;
- filesystem/storage/hypervisor assumptions are named explicitly; storage that violates required locking/sync semantics is unsupported;
- `synchronous=FULL` is not represented as a universal hardware durability guarantee.

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
- review is bound to the exact authoritative observation digest, material plan and trusted risk classification being approved;
- policy/risk-policy/approval changes invalidate stale review evidence;
- durable prepare revalidates exact full observation/review/approval material immediately before crossing the prepare boundary;
- a future executor remains unavailable until the durable generation-bound indeterminate handoff commits;
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
- append-only transaction audit design;
- transaction transition + audit append are one local database transaction;
- ordinary audit UPDATE/DELETE is rejected by the v0.4 SQLite schema;
- deterministic event sequencing and chained integrity digests detect non-coherent deletion/reordering/tampering within the retained database history;
- store integrity validation cross-checks retained audit continuity against current durable transaction state;
- an internal unkeyed hash chain does not detect replacement by an older complete internally consistent database and is not claimed to;
- detected corruption fails closed rather than being silently repaired;
- optional external export/signing/monotonic anchoring remains future enterprise/hardening work.

### Persistence resource exhaustion
A local client tries to consume unbounded disk/memory with authority or audit records, or maintenance silently discards old authority to make room.
- per-record and aggregate durable authority/audit bounds;
- bounded lock/busy waiting;
- oversized inputs rejected before avoidable retained cloning/serialization;
- capacity exhaustion fails closed;
- no silent eviction of authority/audit history in v0.4;
- WAL checkpoint/maintenance may reclaim physical space without changing semantic history.

## Deferred threats

Fleet/remote orchestration and hosted/shared Library services receive dedicated threat-model extensions before any network control plane or trusted shared catalog is enabled.
