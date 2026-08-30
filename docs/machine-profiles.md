# Machine profiles, reusable setups, personality and replay

A machine profile is a portable description of **what a whole machine should become**, not a frozen list of package names.

Profiles compose reusable setups, standalone intents and portable constraints.

```text
Machine Profile
├── Setup: base development
├── Setup: Rust development
├── Setup: travel security
├── standalone intent: use external 4K display
└── portable constraints/preferences
```

Examples:
- AI development workstation
- distraction-minimized writing machine
- travel-security profile
- accessible high-contrast workstation

## Setup vs profile

A **Setup** is a reusable slice of configuration such as Rust development, PostgreSQL development or a security posture. It can be used independently and included by multiple profiles.

A **Machine Profile** composes a larger machine personality from setups and standalone intents.

See [Reusable setups and the Linura Library](reusable-setups.md).

## Portable replay

Export/replay preserves intent, setup composition and policy constraints while allowing the planner to select implementations appropriate to different hardware/platform profiles.

A portable profile export is self-contained: it carries the profile plus the referenced setup and intent definitions needed to adopt it elsewhere. It does not depend on another machine's local database merely to understand what the profile means.

Adoption always re-observes the target machine, resolves target capabilities and produces a fresh plan. A portable profile never replays historical executor effects.

## Secrets

Profiles and setups may declare required secret references such as `credential:github`, but exported artifacts never contain the underlying token/password/key. The receiving machine resolves missing secret refs through its local credential facilities before sensitive actions can proceed.

## Hardware

Hardware hints are advisory inputs to capability resolution. They can express preferences such as GPU use or display expectations, but they do not freeze exact drivers/packages as the portable source of truth.

## Personality

The user's machine "personality" is therefore an explainable composition of active intents, reusable setups and preferences, not hidden model memory.

## Snapshots are separate

A filesystem/system snapshot is an exact recovery artifact for one machine. It is useful for rollback but is not a substitute for a portable setup/profile. Linura deliberately keeps portable declarative configuration and exact recovery state as separate concepts.
