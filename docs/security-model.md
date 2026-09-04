# Security model

## Trust boundaries

```text
natural language / imported content / external services
                       │
                model providers
                       │
               IntentProposal only
                       ▼
Linura First Boot / Linura Agent / Linura Control Center / CLI / Linura SDK / Extensions
                       │
                authenticated IPC
                       ▼
             ┌──────────────────┐
             │     linurad      │  unprivileged authority
             │ graph/plan/policy│
             └────────┬─────────┘
                      │ explicit typed privileged effect
                    Polkit
                      │
          ┌───────────▼───────────┐
          │ narrow root executors │
          └───────────┬───────────┘
                      │
               Linux subsystems
```

A compromised model provider is expected to be capable of proposing malicious intent. It must **not** be equivalent to compromise of the authority plane.

## Privileged executor rules

Each executor:
- owns one narrow domain;
- validates identifiers/arguments again at the privilege boundary;
- receives correlated request/plan context;
- performs no general orchestration or natural-language interpretation;
- exposes no arbitrary command/shell endpoint;
- uses native system APIs where available;
- returns structured outcome for independent verification;
- is separately sandboxed/hardened.

## Agent/model rules

- model output is untrusted data;
- agents emit typed `IntentProposal`/advice, not executable code;
- agent context is least-privilege and minimized;
- secrets use references/handles and are excluded from general prompts by default;
- prompt injection from web/files/repository content cannot create authority;
- tool/model/provider identity does not substitute for OS actor identity;
- sensitive mutations require policy/approval regardless of model confidence;
- offline/no-model behavior is a required security/recovery path.

## Local caller security

“Local” is not equivalent to trusted. D-Bus sender credentials bind calls to OS identities. Agent identity and grant scope are additional authenticated context, not caller-provided labels.

## State integrity

Intent, desired state, graph/provenance and audit persistence require migrations, crash consistency, idempotency and corruption detection. Observed state is always re-derived from Linux providers when truth matters for execution.

## Remote/fleet security

A future fleet gateway is a separate process/service with mutual authentication, explicit enrollment, revocation, replay resistance, scoped device identity, staged rollout and its own threat model. No remote listener is added to `linurad` as a shortcut.

## v0.5 isolated executor and verifier boundary

The v0.5 systemd executor is a qualification-only root service, not a second authority plane. It derives caller identity from the system-bus sender, checks one fixed Polkit action, revalidates the dedicated fixture namespace and exact effect binding, and invokes only systemd's native `RestartUnit` operation. Durable identifiers and digests are correlation/integrity material and never bearer authority.

Executor acknowledgement proves dispatch only. The independent verifier has no D-Bus, native-systemd, executor, Control, policy, transaction, or persistence dependency; it consumes a fresh canonical `ObservationEnvelope` and requires native authority, current freshness, exact resource/capability identity, active state, and an advanced monotonic activation timestamp. Test-only qualification principals and Polkit grants remain under `tests/` and are not production policy. The full managed mutation lifecycle remains unavailable until v0.6 integration.
