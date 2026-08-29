# ADR 0003: Start with one explicit Arch/Hyprland profile

Status: Accepted

## Decision

Develop and test against `arch-hyprland-v1` before claiming support for other distributions/desktops.

## Consequence

Providers are architected for reuse, but cross-distro conditionals do not enter the core prematurely. New distributions become explicit profiles with their own acceptance matrices.
