# Design system

A polished intent-native Linux experience requires shared visual and interaction semantics, not per-panel styling.

## Token groups

- semantic colors: background/surface/foreground/muted/accent/success/warning/danger;
- spacing and type scales;
- radii, borders/elevation;
- motion duration/easing;
- density/control sizes;
- focus/selection states;
- shell geometry.

Themes provide tokens, not arbitrary code. Security/failure/approval meaning uses protected semantic roles so themes cannot visually disguise authority state.

## Cross-surface consistency

First Boot, Agent UI, Control Center, quick settings, OSD, notifications and approval dialogs share vocabulary and primitives.

The UI must clearly distinguish:
- **agent proposal**;
- **approved intent**;
- **desired state**;
- **observed state**;
- **drift/error**;
- **plan awaiting approval**;
- **verified result**.

## Signature first-boot experience

“What do you want this computer to become?” should be calm and minimal, but always offer deterministic/offline/default/import/recovery paths. The conversational surface is never the only route to system management.

## Security UX

High-risk approvals show actor, originating intent, concrete material effects, resource/exposure scope, persistence, risk class, reversibility, verification plan and significant conflicts/shared-resource consequences.

Never use vague prompts such as “Linura wants to make changes,” and never allow generated UI to imitate authoritative approval chrome.
