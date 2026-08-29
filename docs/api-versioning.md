# API versioning

Public local API starts under `Control1` / schema version `1` while the product version remains pre-1.0.

Rules:
- additive fields/methods may be compatible when old clients can ignore them;
- changing semantics of an existing field/action is breaking;
- resource/action IDs are stable once published in a supported release;
- breaking changes introduce a new major interface (`Control2`) with an overlap window;
- persisted audit records keep their original schema version forever;
- providers expose their own implementation version separately from protocol version.

Transport serialization will be selected by ADR. The domain model must not depend on D-Bus-specific types.
