# Dependency and conflict solver design

Linura needs an explainable deterministic solver, not model intuition. The v0.2.0 implementation deliberately begins with bounded dependency closure and deterministic desired-state compilation; richer alternatives and policy-aware constraint solving remain later work.

## Inputs
- active hand-authored intent requirements/constraints/preferences;
- supported platform/provider capabilities;
- blueprint relations (`requires/provides/conflicts/replaces/recommends/optional`);
- typed declarative resource contributions;
- policy constraints in later authority phases;
- already managed/shared resources once durable state exists;
- user-selected alternatives/pins once alternative selection is implemented.

## Outputs
- selected capability/provider set;
- normalized desired resources;
- explicit conflicts and unsatisfied requirements;
- rejected alternatives with reasons when alternative solving exists;
- deterministic explanation/trace suitable for provenance;
- an observation-bound non-executable plan preview once desired state is compared with current authoritative evidence.

## Required properties
- deterministic result for equivalent normalized inputs;
- cycle-safe, bounded resolution;
- fail closed on unresolved hard conflicts;
- fail closed when selected capabilities demand contradictory values for the same desired resource attribute;
- distinguish hard constraints from preferences as the solver matures;
- support alternatives without silently changing security posture;
- produce an unsatisfied/conflict explanation rather than guessing;
- version solver semantics so replay is auditable;
- never use a model, shell output or executor behavior as an implicit tie-breaker.

## v0.2.0 boundary

The initial deterministic planner resolves required capabilities, rejects declared conflicts and missing capabilities, merges typed desired-resource contributions in ordered collections, preserves semantic origins, and compares desired attributes with a validated current observation projection.

A missing observed attribute is a blocker rather than evidence that the desired value is absent. A stale or future observation cannot be used as current planning truth. Plan previews carry prospective risk but always report that execution is unauthorized.

The implementation may eventually use SAT/SMT/constraint techniques if complexity warrants it, but the public model must not depend on a particular solver library.
