# Linura SDK

`linura-sdk` is the public, non-privileged Rust façade for Linura clients and integrations.

## Purpose

Applications should not need to depend directly on internal authority, provider or executor crates merely to understand Linura's system model. The SDK therefore re-exports the stable client-facing concepts required to inspect and request Linura state:

- typed IDs and actors;
- intents, requirements and machine profiles;
- system graph and removal-impact types;
- capability blueprints and resolution results;
- protocol requests/responses;
- explainability and semantic provenance types.

`linuractl` consumes this façade as an architectural proof that ordinary clients do not need authority internals.

## Explicit exclusions

The SDK does **not** expose:

- `linura-control` internals;
- policy-engine implementation details;
- provider registration/runtime internals;
- privileged executors;
- direct root/system mutation handles;
- model-provider credentials or model execution as authority.

Future transports may add an SDK client implementation around D-Bus or a separately authenticated remote gateway. The domain types remain transport-neutral.
