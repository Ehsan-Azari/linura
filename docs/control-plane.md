# Linura Control and the system control plane

`linura-control` implements Linura Control, the local authority subsystem. The **system control plane** is the architectural role it fulfills and is not a separate product brand.

The control plane is the canonical mediator between intent and operating-system effects.

## Responsibilities

- expose versioned local API;
- authenticate/identify callers;
- discover platform/provider capabilities;
- observe current state;
- accept desired changes;
- create deterministic action plans where possible;
- evaluate policy and approval requirements;
- execute allowed effects through providers/executors;
- verify resulting state;
- record audit evidence;
- reconcile persistent desired state when enabled.

## Non-responsibilities

The control plane does not:
- replace NetworkManager/BlueZ/PipeWire/systemd;
- parse arbitrary natural language itself;
- accept arbitrary shell scripts as system actions;
- provide a generic root RPC endpoint;
- make UI-specific layout decisions.

## Gateway transport

The first local transport should be D-Bus because Linux system/session integration and Polkit identity naturally fit it. The domain protocol remains transport-neutral. A remote gRPC/mTLS gateway may be added later as a separate process rather than exposing the session daemon directly to the network.
