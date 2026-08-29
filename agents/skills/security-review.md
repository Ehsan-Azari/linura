# Security review task guide

Threat-model the change before approving new authority.

Check: model/prompt injection, confused deputy, stale approval, privilege escalation, malicious imported profile, untrusted package/plugin, TOCTOU drift, persistence corruption, update bypass, recovery lockout, and data leakage in fixtures/logs.

Security review must distinguish model compromise from control-plane compromise. A malicious model should still be unable to bypass typed policy/executor boundaries.
