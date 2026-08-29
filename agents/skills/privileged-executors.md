# Privileged executor task guide

Privileged executors are intentionally narrow.

- One executor should own a constrained effect family, not arbitrary shell.
- Validate exact operation, arguments, actor authorization context, and preconditions.
- Do not accept model text as executable input.
- Add failure-injection tests for partial execution and verification mismatch.
- Document compensation/recovery when rollback is impossible.
