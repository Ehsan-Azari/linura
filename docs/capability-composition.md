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

Blueprints must not contain arbitrary privileged shell text. Platform providers map resolved desired resources to trusted typed operations.
