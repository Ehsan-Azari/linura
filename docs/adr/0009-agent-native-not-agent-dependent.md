# ADR 0009: Agent-native does not mean agent-dependent

Status: Accepted

Model providers are optional replaceable adapters. CLI, Control Center, authority, policy, execution, explanation and recovery must function without a model provider or internet connection. Agents emit `IntentProposal` only.
