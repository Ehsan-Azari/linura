# Minimal bootstrap, installation and recovery

Linura's product architecture begins before the full desktop exists. Installation creates a small supported base capable of observation, profile selection, snapshots/recovery and first boot.

Recovery must not require `linurad`, a GUI, internet access or a model provider. The supported platform profile should provide documented TTY/snapshot/package-repair paths and must preserve administrator out-of-band repair.

Future installer work must define transaction boundaries, snapshot points, boot rollback, hardware support reporting and offline installation behavior before a supported release.
