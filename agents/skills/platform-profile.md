# Platform-profile task guide

The initial profile is Arch/systemd/Wayland/Hyprland, but core crates must stay distribution-neutral.

- Put distro/compositor assumptions in profiles, packaging, providers, or executors.
- Update `profiles/arch-hyprland-v1.toml` and hardware evidence when support changes.
- A package being available is not equivalent to a feature being support-qualified.
