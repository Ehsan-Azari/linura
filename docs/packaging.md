# Packaging

The first target is native Arch packaging.

Expected packages:
- `linura` — unprivileged daemon, CLI, schemas, docs, profile data;
- `linura-executors` or per-domain executor packages;
- `linura-control-center` — GUI when introduced;
- `linura-shell` — desktop shell client when introduced.

Privileged executors install system D-Bus and Polkit policy separately from the unprivileged client package.

Do not make `/usr/share/linura` user-editable. User configuration belongs under XDG config/state locations; packaged defaults and schemas remain package-owned.
