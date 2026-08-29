# Initial implementation backlog

Tasks are ordered by architectural dependency. Do not rush to a full desktop before proving the causal model.

## Epic A — vocabulary and persistence contracts
1. Typed ID taxonomy for intent/requirement/capability/resource/plan/effect/provenance/audit.
2. Intent lifecycle + requirement schema.
3. Observation envelope and authority/freshness semantics.
4. Desired-state resource envelope.
5. Semantic provenance record and why-chain.
6. Audit event schema distinct from semantic provenance.

## Epic B — system graph
7. Resource/capability/intent graph node taxonomy.
8. Edge semantics: requires/provides/conflicts/replaces/recommends/optional/owns/shared-by/derived-from/realizes.
9. Graph query API: why, dependencies, dependents, conflicts.
10. Safe-retirement/shared-ownership impact analysis.
11. Graph invariant/property tests.

## Epic C — capability composition and planning
12. Capability blueprint schema/loader.
13. Deterministic dependency closure.
14. Conflict analysis and alternative-provider model.
15. Intent → requirement → capability mapping contract.
16. Desired-state derivation.
17. Observed/desired diff.
18. Plan generation with provenance origins.

## Epic D — local gateway and observation
19. `org.linura.Control1` D-Bus skeleton.
20. Caller credentials → Actor.
21. Provider registry and health.
22. systemd observer.
23. NetworkManager observer.
24. System graph population from observations.
25. CLI observe/graph/capabilities/explain.

## Epic E — first mutation
26. Secure SSH or test-service intent fixture.
27. Plan-only service mutation.
28. Baseline policy/approval records.
29. systemd privileged executor + Polkit.
30. Execute + independent verify.
31. Compensation/failure injection.
32. Audit + semantic provenance persistence.

## Epic F — persistence and intent lifecycle
33. Persistence ADR (expected SQLite/WAL unless evidence changes it).
34. Migrations/crash consistency.
35. Intent activate/suspend/supersede/retire.
36. Idempotency and retry semantics.
37. Reconciliation/drift with out-of-band admin repair safeguards.
38. Explain/remove-impact end-to-end tests.

## Epic G — agent-native layer
39. Agent provider-neutral interface.
40. Deterministic/no-model interpreter fixture.
41. Hosted/local provider adapter boundary.
42. Specialist roles and scoped context.
43. Multi-specialist conflict reporting.
44. Prompt-injection/tool-escalation negative tests.
45. Agent-unavailable/offline behavior tests.

## Epic H — first boot and profiles
46. Minimal bootstrap image contract.
47. Hardware/capability discovery flow.
48. "What do you want this computer to become?" UI.
49. Plan review/modify/explain/approve.
50. Offline default and skip-agent path.
51. Portable profile export/import/adopt.
52. Snapshot/rollback/recovery acceptance tests.

## Epic I — composition UX
53. Workflow schema/runtime.
54. Screenshot-share reference workflow.
55. Derived surface schema/renderer.
56. Containers reference surface.
57. Capability permission UX.
58. Isolated extension runtime.

## Epic J — broader platform and enterprise
59. Remaining system-domain providers/executors.
60. Control Center.
61. Desktop shell.
62. Installer/update/release supply-chain hardening.
63. Hardware compatibility matrix.
64. Optional remote/fleet gateway only after local trust model is proven.
