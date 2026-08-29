# Platform profiles

Linura does not claim generic “Linux support.” It supports explicit profiles that define the substrate and tested compatibility envelope.

`arch-hyprland-v1` defines the first profile.

Profiles specify:
- distro/base package assumptions;
- init/session/compositor;
- selected providers;
- minimum supported subsystem versions;
- enabled Linura features;
- packaging/update/recovery expectations.

## Compatibility states

- **supported:** covered by CI/VM acceptance suite.
- **experimental:** implementation exists, not a release guarantee.
- **unsupported:** deliberately outside contract.
- **unknown:** capability detection could not establish support.

Adding Fedora/Ubuntu should create new profiles sharing providers where possible; do not add distro conditionals throughout the core.
