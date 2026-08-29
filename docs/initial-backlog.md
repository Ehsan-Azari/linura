# Initial implementation backlog

Tasks are ordered by architectural dependency. Do not rush to a full desktop before proving the causal and trustworthy mutation model.

## Epic A — vocabulary, lifecycle and evidence contracts
1. Typed ID taxonomy for intent/requirement/capability/resource/plan/effect/provenance/audit.
2. Intent lifecycle + requirement schema.
3. Observation envelope and authority/freshness semantics.
4. Desired-state resource envelope.
5. Semantic provenance record and why-chain.
6. Audit event schema distinct from semantic provenance.
7. Canonical eleven-stage mutation enum/state machine.
8. Correlated prepare/execution/verification/commit/audit/reconciliation receipt contracts.
9. Failure/denial/indeterminate audit contract.

## Epic B — system graph
10. Resource/capability/intent graph node taxonomy.
11. Edge semantics: requires/provides/conflicts/replaces/recommends/optional/owns/shared-by/derived-from/realizes.
12. Graph query API: why, dependencies, dependents, conflicts.
13. Safe-retirement/shared-ownership impact analysis.
14. Graph invariant/property tests.

## Epic C — capability composition and deterministic planning
15. Capability blueprint schema/loader.
16. Deterministic dependency closure.
17. Conflict analysis and alternative-provider model.
18. Intent → requirement → capability mapping contract.
19. Desired-state derivation.
20. Observed/desired diff.
21. Plan generation with provenance origins.
22. Require authoritative observation as an explicit planning input.
23. Plan structural/precondition/verification validation.

## Epic D — local gateway and observation
24. `org.linura.Control1` D-Bus skeleton.
25. Caller credentials → `Actor`.
26. Provider registry and health.
27. systemd observer.
28. NetworkManager observer.
29. Observation freshness/staleness enforcement.
30. System graph population from observations.
31. CLI observe/graph/capabilities/explain.

## Epic E — durable authority transaction and recovery
32. Persistence ADR (expected SQLite/WAL unless evidence changes it).
33. Migrations/crash consistency/corruption detection.
34. Request/plan idempotency and retry semantics.
35. Durable `prepare` intent-to-execute record before external effects.
36. Indeterminate-operation recovery state and re-observation rules.
37. Verified `commit` transaction for desired state/graph/provenance.
38. Append-only success/failure/compensation audit persistence.
39. Recovery tests proving no blind replay after ambiguous execution.

## Epic F — first complete managed mutation
40. Secure SSH or test-service intent fixture.
41. Plan-only service mutation from authoritative observation.
42. Baseline policy + approval evidence records.
43. systemd privileged executor + Polkit.
44. Independent verifier consuming post-execution authoritative observation.
45. Full request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile orchestration.
46. Compensation/failure injection.
47. Denial, stale observation, wrong receipt correlation and verification-failure tests.
48. Reconciliation/drift with out-of-band admin repair safeguards.
49. Disposable VM acceptance proving all eleven stages.

## Epic G — persistent intent lifecycle
50. Persist intent activate/suspend/supersede/retire lineage.
51. Shared-ownership and removal-impact enforcement.
52. Safe cleanup plans routed through the same eleven-stage mutation lifecycle.
53. Explain/remove-impact end-to-end tests.

## Epic H — agent-native layer
54. Agent provider-neutral interface.
55. Deterministic/no-model interpreter fixture.
56. Hosted/local provider adapter boundary.
57. Specialist roles and scoped context.
58. Multi-specialist conflict reporting.
59. Prompt-injection/tool-escalation negative tests.
60. Agent-unavailable/offline behavior tests.

## Epic I — first boot and profiles
61. Minimal bootstrap image contract.
62. Hardware/capability discovery flow.
63. "What do you want this computer to become?" UI.
64. Plan review/modify/explain/approve.
65. Offline default and skip-agent path.
66. Portable profile export/import/adopt.
67. Snapshot/rollback/recovery acceptance tests.

## Epic J — composition UX
68. Workflow schema/runtime.
69. Screenshot-share reference workflow.
70. Derived surface schema/renderer.
71. Containers reference surface.
72. Capability permission UX.
73. Isolated extension runtime.

## Epic K — broader platform and enterprise
74. Remaining system-domain providers/executors/verifiers.
75. Control Center.
76. Desktop shell.
77. Installer/update/release supply-chain hardening.
78. Hardware compatibility matrix.
79. Optional remote/fleet gateway only after local trust model is proven.
