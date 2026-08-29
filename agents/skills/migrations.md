# Migration task guide

- Add a versioned descriptor under the correct `migrations/<scope>/` directory.
- Make preconditions explicit and application idempotent.
- Verify before writing the ledger.
- Require snapshot/checkpoint before risky transformations.
- Never reuse a released migration ID.
- Test rerun, not-applicable, verification failure, and recovery behavior.
