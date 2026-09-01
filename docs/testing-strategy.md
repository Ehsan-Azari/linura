# Testing strategy

Linura changes host state **and** interprets user intent, so correctness requires both systems testing and adversarial intelligence-boundary testing.

## Layers

1. Pure unit/property tests — IDs, intent lifecycle, graph invariants, solver, policy, diff/planning.
2. Schema/contract tests — public JSON/TOML/D-Bus/API compatibility.
3. Provider contract tests — fake buses/APIs and deterministic observations.
4. Executor tests — namespaces/VMs with malformed input and failure injection.
5. Integration tests — session/system bus, policy, persistence and crash windows.
6. VM acceptance tests — fresh install, first boot, reboot, update, migration, rollback, recovery.
7. Agent boundary tests — prompt injection, malicious proposal, stale context, disagreement, provider outage.
8. Profile replay tests — equivalent intent across compatible hardware/profile variants.
9. Hardware matrix — Wi-Fi/Bluetooth/audio/GPU/storage/display/suspend quirks.
10. Supply-chain/release verification — SBOM/signatures/attestations/artifact digests.

## Required negative paths

Mutation work is incomplete without:
- unauthorized actor/grant;
- malicious agent proposal;
- unsupported/missing capability;
- dependency cycle/conflict/unsatisfied alternative;
- stale precondition or drift during approval;
- provider/executor unavailable;
- partial effect failure;
- verification mismatch;
- retry/idempotency ambiguity;
- rollback/compensation failure;
- crash after effect but before persistence;
- corrupted intent/graph/provenance state.

Intent lifecycle is incomplete without:
- retire an intent with exclusively owned resources;
- retire an intent sharing dependencies with another active intent;
- suspend/resume without accidental cleanup;
- supersede while preserving lineage;
- out-of-band administrator repair conflicting with reconciliation.

Agent-native UX is incomplete without:
- no network;
- no model configured;
- model provider outage/rate limit;
- malicious content attempting prompt/tool escalation;
- specialist disagreement;
- first boot using deterministic default/import path.

## Future context-query/probe acceptance contract

When a generalized context-query runtime is implemented, its work is incomplete without repeatable negative-path coverage for:

- per-probe timeout/deadline enforcement;
- cancellation without orphaned unbounded work;
- bounded concurrency/fan-out and resource budgets;
- provider unavailability and deterministic partial-result semantics;
- stale/future/mismatched cached evidence rejection when current authority is required;
- query coalescing that preserves caller isolation, freshness and provenance;
- bounded cache/history growth and deterministic eviction;
- backpressure and bounded response/result size;
- deterministic aggregation for contracts that require deterministic output;
- per-result source/provenance/freshness retention;
- retries that do not silently exceed the caller's budget;
- transport/provider failure without leaking implementation-specific authority into the domain model;
- proof that retrieval/RAG output cannot become authoritative observed state or an authority grant;
- fleet/cluster federation, when introduced, preserving local-machine authority and explicit partial-failure semantics.

These requirements are a future implementation contract, not a current release capability claim. They do not add a stage to the canonical managed-mutation lifecycle.

## Acceptance principle

A demo is not an acceptance test. Supported release claims require repeatable evidence from clean/disposable machines and recovery from injected failures.

## Executable harnesses

The repository now includes executable harness boundaries:

- `cargo xtask check` for canonical local/CI validation;
- `tools/acceptance.py` for versioned guest scenarios;
- `tools/vm.py` for disposable QEMU/KVM planning/start;
- `tools/image.py` for Arch image planning/build;
- `tools/visual.py` for reviewed visual-baseline comparison;
- `hardware/fixtures/` and `hardware/support-matrix.json` for evidence tracking.

A harness existing is not equivalent to evidence. Release claims must identify the exact image/hardware/profile and successful run used to support the claim.
