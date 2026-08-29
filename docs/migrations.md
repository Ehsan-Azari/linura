# Migrations

Migrations are versioned data/system transformations with explicit scope, preconditions, verification, and recovery behavior.

## Scopes

- system;
- user;
- intent;
- system graph;
- machine profile.

`linura-migrations` provides the descriptor, ledger, and runner contracts. Migration descriptors are also represented by `schemas/migration.v1.schema.json` so release tooling can reason about migrations without loading Rust code.

## Rules

- identifiers are immutable once released;
- application must be idempotent through the migration ledger;
- verification happens before a migration is marked applied;
- reversible migrations must provide a compensation path;
- non-reversible migrations must declare recovery requirements;
- migrations that could invalidate large amounts of state should require a snapshot/checkpoint first;
- user and system migrations are separately attributable.

Baseline descriptors under `migrations/*/0000-baseline.json` reserve the scopes and exercise validation before real migrations exist.
