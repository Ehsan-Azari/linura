# Application supervision

The desktop compositor is not Linura's process supervisor.

Desktop applications and long-lived helpers should be launched into systemd user scopes/units when practical so resource accounting, restart policy, logs, and OOM isolation remain observable independently of the shell.

The shell is a client of the system authority and a presentation surface. Crashing or restarting the shell must not implicitly terminate unrelated managed applications or invalidate system state.

Future supervision contracts should expose typed fields for resource limits, restart behavior, lifetime, ownership, and user-visible failure state rather than embedding arbitrary shell launch commands into UI metadata.
