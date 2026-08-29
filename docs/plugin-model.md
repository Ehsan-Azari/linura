# Plugin and extension model

Extensions are not loaded as arbitrary native code inside `linurad` or privileged executors.

## Preferred models

1. Out-of-process extension communicating over a capability-limited IPC contract.
2. WASM component runtime with explicit host capabilities where feasible.

## Capability examples

```text
system.network.read
system.audio.read
notification.send
ui.panel.register
```

A weather widget does not receive filesystem, process, network-control, or root capabilities unless explicitly granted.

## Supply chain

Extensions require:
- manifest with ID/version/publisher;
- declared capabilities;
- content digest;
- optional signature/attestation;
- explicit enablement;
- update policy and rollback path.

A plugin update cannot silently add new capabilities; capability expansion requires approval.
