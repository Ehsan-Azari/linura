# Arch packaging and image profile

This directory contains the first Linura platform-profile packaging assets.

- `archiso/` is an overlay staged on top of ArchISO's installed `releng` profile by `tools/image.py`.
- `archiso/packages.linura` contains only Linura additions; it is merged with the base releng package set.
- `hooks/95-linura-update-guard.hook` is installed only when the matching guard binary is staged.

These files are development packaging, not evidence that an installer/image has passed release qualification.
