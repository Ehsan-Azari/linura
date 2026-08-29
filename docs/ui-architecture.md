# UI architecture

Linura has four principal local UX surfaces: Linura First Boot, Linura Agent, Linura Control Center and Linura Shell. All are clients of the same versioned protocol.

## Rules
- no distro/provider-specific backend logic in UI;
- every mutation is plan-first and shows material effects/risk/approval;
- `Explain` renders structured provenance/dependency evidence;
- intent retirement shows shared resources and cleanup impact;
- agent suggestions are visually distinguishable from approved desired state;
- unsupported capabilities are explicit, never silently hidden as success;
- offline/no-model control remains available;
- generated capability UI uses constrained derived surfaces or isolated extensions.

The signature first-boot prompt is a product surface, not a privilege boundary.
