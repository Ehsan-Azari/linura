# ADR 0001: Rust 2024 for core and control processes

Status: Accepted

## Context

Linura handles privileged-adjacent operating-system state and needs strong types, predictable binaries, good Linux systems integration, and memory safety.

## Decision

Use stable Rust 2024 for core libraries, control-plane services, CLI, and privileged executors. Pin the repository toolchain. Unsafe Rust is forbidden by default and requires a future narrowly scoped exception ADR if ever needed.

UI technology may differ if justified, but UI remains a client of the same protocol.
