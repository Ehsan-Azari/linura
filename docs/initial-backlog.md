# Initial implementation backlog

Tasks are ordered by architectural dependency. Do not rush to a full desktop before proving the causal and trustworthy mutation model.

## Epic A — vocabulary, lifecycle and reusable-state contracts
1. Typed ID taxonomy for intent/setup/requirement/capability/resource/plan/effect/provenance/audit.
2. Intent lifecycle + requirement schema.
3. Reusable Setup domain model, revision validation and setup schema.
4. MachineProfile composition of setups + standalone intents.
5. Self-contained portable setup/profile export/adoption protocol.
6. Secret-reference-only portability contract.
7. Observation envelope and authority/freshness semantics.
8. Desired-state resource envelope.
9. Semantic provenance record and why-chain.
10. Audit event schema distinct from semantic provenance.
11. Canonical eleven-stage mutation enum/state machine.
12. Correlated prepare/execution/verification/commit/audit/reconciliation receipt contracts.
13. Failure/denial/indeterminate audit contract.

## Epic B — system graph
14. Resource/capability/setup/intent graph node taxonomy.
15. Edge semantics: requires/provides/conflicts/replaces/recommends/optional/owns/shared-by/derived-from/realizes.
16. Setup → adopted-intent provenance relationships.
17. Graph query API: why, dependencies, dependents, conflicts.
18. Safe-retirement/shared-ownership impact analysis.
19. Graph invariant/property tests.

## Epic C — capability composition and deterministic planning
20. Capability blueprint schema/loader.
21. Deterministic dependency closure.
22. Conflict analysis and alternative-provider model.
23. Intent → requirement → capability mapping contract.
24. Desired-state derivation.
25. Observed/desired diff.
26. Plan generation with provenance origins.
27. Require authoritative observation as an explicit planning input.
28. Plan structural/precondition/verification validation.

## Epic D — local gateway and observation
29. `org.linura.Control1` D-Bus skeleton.
30. Caller credentials → `Actor`.
31. Provider registry and health.
32. systemd observer.
33. NetworkManager observer.
34. Observation freshness/staleness enforcement.
35. System graph population from observations.
36. CLI observe/graph/capabilities/explain.

## Epic E — durable authority transaction and recovery
37. Persistence ADR (expected SQLite/WAL unless evidence changes it).
38. Migrations/crash consistency/corruption detection.
39. Request/plan idempotency and retry semantics.
40. Durable `prepare` intent-to-execute record before external effects.
41. Indeterminate-operation recovery state and re-observation rules.
42. Verified `commit` transaction for desired state/graph/provenance.
43. Append-only success/failure/compensation audit persistence.
44. Recovery tests proving no blind replay after ambiguous execution.

## Epic F — first complete managed mutation
45. Secure SSH or test-service intent fixture.
46. Plan-only service mutation from authoritative observation.
47. Baseline policy + approval evidence records.
48. systemd privileged executor + Polkit.
49. Independent verifier consuming post-execution authoritative observation.
50. Full request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile orchestration.
51. Compensation/failure injection.
52. Denial, stale observation, wrong receipt correlation and verification-failure tests.
53. Reconciliation/drift with out-of-band admin repair safeguards.
54. Disposable VM acceptance proving all eleven stages.

## Epic G — persistent intent lifecycle + Linura Library
55. Persist intent activate/suspend/supersede/retire lineage.
56. Shared-ownership and removal-impact enforcement.
57. Safe cleanup plans routed through the same eleven-stage mutation lifecycle.
58. Explain/remove-impact end-to-end tests.
59. Local Library persistence and listing for Setup/Profile revisions.
60. Setup include/composition resolution with cycle detection.
61. Setup/profile self-contained export/import validation.
62. Same-device dry-run/adoption and target-state diff.
63. Cross-device adoption against a different supported hardware/platform fixture.
64. Missing secret-reference reporting and local resolution flow.
65. Capture a Setup from Linura-managed causal state while excluding ephemeral/unmanaged state by default.
66. Setup revision/provenance/history tests.

## Epic H — agent-native layer
67. Agent provider-neutral interface.
68. Deterministic/no-model interpreter fixture.
69. Hosted/local provider adapter boundary.
70. Specialist roles and scoped context.
71. Multi-specialist conflict reporting.
72. Typed agent proposals to save/adopt setups without execution authority.
73. Prompt-injection/tool-escalation/imported-setup negative tests.
74. Agent-unavailable/offline behavior tests.

## Epic I — first boot and profiles
75. Minimal bootstrap image contract.
76. Hardware/capability discovery flow.
77. "What do you want this computer to become?" UI.
78. Library browse + setup/profile import path.
79. Plan review/modify/explain/approve.
80. Offline default and skip-agent path.
81. Portable setup/profile export/import/adopt acceptance tests.
82. Snapshot/rollback/recovery acceptance tests proving snapshots remain separate from portable configuration.

## Epic J — composition UX
83. Workflow schema/runtime.
84. Screenshot-share reference workflow.
85. Derived surface schema/renderer.
86. Containers reference surface.
87. Capability permission UX.
88. Isolated extension runtime.
89. Reusable workflow Library integration after workflow portability stabilizes.

## Epic K — broader platform, sharing and enterprise
90. Remaining system-domain providers/executors/verifiers.
91. Control Center including Library/setup/profile surfaces.
92. Desktop shell.
93. Installer/update/release supply-chain hardening.
94. Hardware compatibility matrix.
95. Canonical portable-artifact serialization/content digest/signature ADR + implementation.
96. Optional Git/user-owned/hosted Library sync adapters.
97. Optional enterprise catalog policy and remote/fleet gateway only after local trust model is proven.
