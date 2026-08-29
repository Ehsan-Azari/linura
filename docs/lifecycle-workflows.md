# Lifecycle workflows

Linura supports lifecycle extensibility without recreating unrestricted privileged hook directories.

`linura-lifecycle` defines typed events such as before/after bootstrap, update, reconciliation, and recovery. A lifecycle workflow consists of typed capability operations and declared permissions.

Policy evaluates lifecycle steps through the same authority model as other effects. A package or extension does not gain root execution merely because it wants an `after-update` hook.

This design intentionally replaces the traditional pattern:

```text
lifecycle event → arbitrary shell script → ambient privilege
```

with:

```text
lifecycle event → typed workflow → policy → provider/executor → verification
```
