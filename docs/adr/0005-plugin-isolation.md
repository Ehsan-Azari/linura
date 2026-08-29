# ADR 0005: No unsandboxed third-party plugins in control processes

Status: Accepted

## Decision

Third-party extensions cannot load arbitrary native/QML code inside `linurad` or privileged executors. Prefer out-of-process capability IPC or a WASM component runtime.

A plugin capability increase requires explicit re-approval.
