# Dependency and conflict solver design

The bootstrap `CapabilityCatalog::resolve` is deliberately simple dependency closure. Production Linura needs an explainable deterministic solver, not model intuition.

## Inputs
- active intent requirements/constraints/preferences;
- supported platform/provider capabilities;
- blueprint relations (`requires/provides/conflicts/replaces/recommends/optional`);
- policy constraints;
- already managed/shared resources;
- user-selected alternatives/pins.

## Outputs
- selected capability/provider set;
- desired resources;
- explicit conflicts and unsatisfied requirements;
- rejected alternatives with reasons;
- deterministic explanation/trace suitable for provenance.

## Required properties
- deterministic result for equivalent normalized inputs;
- cycle detection and bounded execution;
- fail closed on unresolved hard conflicts;
- distinguish hard constraints from preferences;
- support alternatives without silently changing security posture;
- produce an unsatisfied/conflict explanation rather than guessing;
- version solver semantics so replay is auditable.

The implementation may eventually use SAT/SMT/constraint techniques if complexity warrants it, but the public model must not depend on a particular solver library.
