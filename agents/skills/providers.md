# Provider task guide

Providers observe and plan against a Linux subsystem; they do not become ambient root helpers.

- Fail closed on unknown/ambiguous state.
- Prefer native D-Bus/netlink/library APIs over parsing human CLI output.
- Emit capability support with a reason/evidence level.
- Make observations deterministic enough for sanitized fixtures.
- Provider tests must cover unavailable service, malformed state, unsupported feature, and stale observation.
