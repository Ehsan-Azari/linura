# Agent architecture

## Absolute rule

**An agent is an untrusted proposer, never an authority.**

Agent providers translate natural language/context into typed `IntentProposal` objects and optional advisory material. They cannot directly invoke privileged executors.

## Provider neutrality

The intelligence layer supports adapters for hosted models, local models, enterprise models, deterministic/rule-based interpreters, or no model at all. No provider is a core architectural dependency.

## Specialist roles

Logical specialists can include coordinator, hardware, security, developer, desktop, productivity and future domain roles. Specialists share scoped system context and produce advice/proposals; they do not become independent authorities.

## Multi-agent disagreement

When specialist recommendations conflict, the planner records the conflict and surfaces alternatives. It never resolves a security-sensitive disagreement by allowing one model to execute directly.

## Context and secrets

Agent context is capability-scoped and minimized. Secrets are represented by references/handles and are not inserted into general model context by default.
