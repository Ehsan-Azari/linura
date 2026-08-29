# State model

Linura distinguishes four truth categories:

1. **Intent** — approved durable purpose/constraints.
2. **Desired state** — deterministic state derived from intent/capabilities/policy.
3. **Observed state** — what providers prove is currently true on Linux.
4. **Provenance/history** — why and how desired/observed state changed.

The local database is authoritative for Linura-managed intent and history; Linux providers are authoritative for actual current system state.

## Reconciliation

Reconciliation compares desired and observed state, then produces the same plan/policy/execute/verify lifecycle as an interactive request. It never silently overwrites an administrator's intentional out-of-band repair. Drift can be report-only, require approval, or reconcile according to explicit policy.

## Intent-aware cleanup

When intent is retired, desired state is recomputed through the system graph. Shared resources remain if another active intent/capability owns them. Cleanup is a plan with impact, policy, verification and provenance—not package-manager autoremove by itself.
