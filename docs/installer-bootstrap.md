# Installer and bootstrap

The Linura installer is a staged, checkpointed lifecycle rather than one large shell script.

## Stage order

`linura-bootstrap` defines the canonical stage sequence:

1. preflight;
2. disk layout;
3. encryption;
4. base system;
5. platform packages;
6. bootloader;
7. security baseline;
8. baseline snapshot;
9. user provisioning;
10. first-boot-ready.

A persisted ledger records completed stages. A restart resumes from the first incomplete stage and rejects impossible out-of-order ledgers.

## Modes

- **interactive** — normal human-guided installation;
- **non-interactive** — automation and image-test installation without a TTY;
- **recovery** — repair/bootstrap continuation with the minimum UI dependency.

## Supported security baseline

The first `arch-hyprland-v1` profile requires, for a supported installation:

- LUKS2-class disk encryption;
- inbound firewall deny-by-default;
- SSH disabled initially;
- untrusted/AUR-like package sources disabled initially;
- a native break-glass recovery path that does not depend on an agent or Linura UI;
- a baseline snapshot/factory-reset anchor when the selected storage profile supports it.

`packaging/arch/archiso/airootfs/etc/linura/install-policy.json` is the machine-readable development policy for the image profile.

The installer must never require a model provider. The first-boot agent is a primary experience, not a recovery dependency.

The development image build stages from ArchISO `releng`, overlays Linura policy/profile files, and merges `packages.linura`; this keeps boot infrastructure inherited from a known ArchISO base while Linura owns its explicit additions.
The image harness also stages the exact compiled Linura binaries into the image. The Arch ALPM update-guard hook is copied only in the same staging step that installs `linura-update-guard`, preventing a dangling package-manager hook from entering an image.
