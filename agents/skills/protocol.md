# Protocol task guide

Public protocol changes are compatibility-sensitive.

- Read `docs/api-versioning.md` and `docs/sdk.md`.
- Add versioned schemas/interfaces rather than silently changing existing semantics.
- Keep `linura-sdk` free of policy/provider/executor internals.
- CLI and graphical clients should consume the same public concepts.
- Add contract tests and update protocol introspection/documentation.
