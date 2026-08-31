# Capability composition

Users request capabilities, not implementation trivia. A capability blueprint describes what a machine can provide and how that capability composes with others.

Relations include `requires`, `provides`, `conflicts`, `replaces`, `recommends`, and `optional`.

Example:

```text
development.ai
 ├─ requires development.python
 ├─ requires compute.gpu
 ├─ recommends containers
 └─ optional notebooks
```

The solver produces a deterministic resolution. Missing requirements and conflicts are explicit plan blockers until a policy/user decision selects a valid alternative.

## Declarative resource contributions

Resolved capabilities contribute **typed desired resources**, not command strings. Each resource contribution names:

- the authoritative observation provider;
- the resource identity;
- the observation capability used to establish current state;
- a normalized map of desired state attributes.

Multiple capabilities may contribute compatible attributes to the same resource. If two selected capabilities demand different values for the same resource attribute, deterministic desired-state compilation fails closed instead of selecting one implicitly.

Semantic origin is added when a capability contribution is compiled for an intent, so desired resources retain the intent, requirement and contributing-capability identities that explain why the state is wanted.

Blueprints must not contain arbitrary privileged shell text. A plan preview compares normalized desired state with authoritative provider evidence. Future platform mutation providers may map a separately authorized plan to trusted typed operations, but that authority is outside the v0.2.0 planning boundary.
