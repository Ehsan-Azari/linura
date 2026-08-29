# Recovery

Linura must never become a single point of failure for the host.

## Recovery invariants

- Native Linux tools remain usable when Linura is stopped.
- Linura does not encrypt/obscure its own essential state without a documented recovery mechanism.
- Updates create a snapshot when the platform profile supports reliable snapshots, but lack of a snapshot never hides update failure.
- Privileged executors can be disabled independently.
- A broken UI can be bypassed with CLI/native tools.
- A broken `linurad` does not block boot/login.

## Break-glass procedure (target)

1. Boot/login using native platform path.
2. Stop `linurad` and Linura executors.
3. Inspect audit/update logs.
4. Use native tools to restore critical networking/services/storage if necessary.
5. Roll back package/snapshot if update-related.
6. Export diagnostic bundle before destructive repair when possible.

This procedure must be tested in VM acceptance before the first supported release.
